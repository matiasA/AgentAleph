use crate::agent::context;
use crate::agent::message::{approx_tokens, to_wire, to_wire_native, AgentMsg, ToolCall};
use crate::agent::permissions::{self, AgentMode, Decision};
use crate::agent::session_store;
use crate::agent::{grammar::tool_call_grammar, prompt::system_prompt, tools, tools::Registry};
use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::state::AppState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

/// Hard per-turn step cap to prevent infinite loops in weaker models.
pub const MAX_STEPS: usize = 25;

/// Final message when aborting because the same call repeated too many times.
const LOOP_MSG: &str = "I kept repeating the same action without making progress. Stopping the task; \
                        review the previous step results.";

/// Result of processing one tool call inside a step.
enum CallFlow {
    /// Call handled: continue.
    Continue,
    /// Same call repeated too many times: abort the turn.
    Loop,
}

// ---------- Frontend Events ----------

#[derive(Clone, Serialize)]
pub struct AgentTokenEvent {
    pub session_id: String,
    pub token: String,
    pub is_reasoning: bool,
}

#[derive(Clone, Serialize)]
pub struct AgentStepEvent {
    pub session_id: String,
    pub step: usize,
    pub phase: String, // "model_start"
}

#[derive(Clone, Serialize)]
pub struct AgentToolEvent {
    pub session_id: String,
    pub step: usize,
    pub tool: String,
    pub args: String,
    pub result: String,
    pub is_error: bool,
}

#[derive(Clone, Serialize)]
pub struct AgentDoneEvent {
    pub session_id: String,
    pub text: String,
    pub reason: String, // "done" | "max_steps" | "cancelled" | "error" | "loop"
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct AgentPermissionEvent {
    pub session_id: String,
    pub request_id: String,
    pub tool: String,
    pub args: String,
    pub summary: String,
}

// ---------- Sink de eventos: abstrae el destino de los eventos del loop ----------
//
// El cuerpo del loop de agente vive desacoplado de `tauri::AppHandle`: emite a un
// `LoopSink`. La ruta Tauri usa `TauriSink` (reemite eventos a la webview); un
// harness headless (tests/eval) puede usar un sink que solo registra o imprime,
// sin necesidad de una app de Tauri corriendo. Así probamos el mismo código del
// harness que ejecuta la app real.
pub trait LoopSink: Send + Sync {
    fn emit_step(&self, session_id: &str, step: usize, phase: &str);
    fn emit_token(&self, session_id: &str, token: &str, is_reasoning: bool);
    fn emit_tool(
        &self,
        session_id: &str,
        step: usize,
        tool: &str,
        args: &str,
        result: &str,
        is_error: bool,
    );
    fn emit_permission(
        &self,
        session_id: &str,
        request_id: &str,
        tool: &str,
        args: &str,
        summary: &str,
    );
    fn emit_done(&self, session_id: &str, text: &str, reason: &str, error: Option<&str>);
}

/// `LoopSink` que reemite los eventos al frontend vía `app.emit` (ruta Tauri real).
struct TauriSink {
    app: AppHandle,
}

impl LoopSink for TauriSink {
    fn emit_step(&self, session_id: &str, step: usize, phase: &str) {
        let _ = self.app.emit(
            "agent://step",
            AgentStepEvent {
                session_id: session_id.to_string(),
                step,
                phase: phase.to_string(),
            },
        );
    }
    fn emit_token(&self, session_id: &str, token: &str, is_reasoning: bool) {
        let _ = self.app.emit(
            "agent://token",
            AgentTokenEvent {
                session_id: session_id.to_string(),
                token: token.to_string(),
                is_reasoning,
            },
        );
    }
    fn emit_tool(
        &self,
        session_id: &str,
        step: usize,
        tool: &str,
        args: &str,
        result: &str,
        is_error: bool,
    ) {
        let _ = self.app.emit(
            "agent://tool",
            AgentToolEvent {
                session_id: session_id.to_string(),
                step,
                tool: tool.to_string(),
                args: args.to_string(),
                result: result.to_string(),
                is_error,
            },
        );
    }
    fn emit_permission(
        &self,
        session_id: &str,
        request_id: &str,
        tool: &str,
        args: &str,
        summary: &str,
    ) {
        let _ = self.app.emit(
            "agent://permission",
            AgentPermissionEvent {
                session_id: session_id.to_string(),
                request_id: request_id.to_string(),
                tool: tool.to_string(),
                args: args.to_string(),
                summary: summary.to_string(),
            },
        );
    }
    fn emit_done(&self, session_id: &str, text: &str, reason: &str, error: Option<&str>) {
        let _ = self.app.emit(
            "agent://done",
            AgentDoneEvent {
                session_id: session_id.to_string(),
                text: text.to_string(),
                reason: reason.to_string(),
                error: error.map(|s| s.to_string()),
            },
        );
    }
}

// ---------- Parsing del stream de llama-server ----------

#[derive(Deserialize)]
struct ChatChunk {
    choices: Vec<ChatChoice>,
}
#[derive(Deserialize)]
struct ChatChoice {
    delta: ChatDelta,
}
#[derive(Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<DeltaToolCall>>,
}
#[derive(Deserialize)]
struct DeltaToolCall {
    #[serde(default)]
    index: usize,
    id: Option<String>,
    function: Option<DeltaFunc>,
}
#[derive(Deserialize)]
struct DeltaFunc {
    name: Option<String>,
    arguments: Option<String>,
}

/// Resultado de un paso de inferencia: texto del asistente y, en la ruta nativa, las llamadas
/// a herramienta emitidas. En la ruta GBNF `tool_calls` queda vacío (la llamada va en `content`).
#[derive(Default)]
struct StepOutput {
    content: String,
    tool_calls: Vec<ToolCall>,
}

/// Acumulador de una llamada nativa cuyos fragmentos llegan repartidos entre chunks del stream.
#[derive(Default)]
struct AccCall {
    id: String,
    name: String,
    args: String,
}

/// Decide si usar la ruta nativa para el modelo cargado, según la preferencia del usuario.
/// En `"auto"` aplica una heurística por familia/tamaño del modelo (las familias con
/// tool-calling fiable van por nativo; los modelos chicos siguen con GBNF, más robusto).
pub fn use_native_tools(pref: &str, model_name: &str) -> bool {
    match pref {
        "native" => true,
        "grammar" => false,
        _ => auto_native(model_name),
    }
}

fn auto_native(model_name: &str) -> bool {
    let n = model_name.to_lowercase();
    // Modelos chicos: el tool-calling nativo es poco fiable → preferimos GBNF.
    const SMALL: &[&str] = &["0.5b", "0.8b", "-1b", "1.5b", "-2b", "-3b", "1.7b"];
    if SMALL.iter().any(|s| n.contains(s)) {
        return false;
    }
    // Familias con soporte nativo de herramientas razonablemente fiable en GGUF.
    const NATIVE: &[&str] = &[
        "coder",
        "qwen2.5",
        "qwen3",
        "llama-3.1",
        "llama-3.2",
        "llama-3.3",
        "hermes",
        "functionary",
        "firefunction",
        "command-r",
        "mistral",
        "mixtral",
    ];
    NATIVE.iter().any(|s| n.contains(s))
}

// ---------- Entradas públicas ----------

/// Lanza un turno de agente en una tarea en segundo plano y emite eventos `agent://*`.
/// Ruta Tauri: usa `TauriSink` y resuelve permisos vía la UI (`AppState`).
pub async fn run_agent(
    app: AppHandle,
    state: Arc<AppState>,
    session_id: String,
    working_dir: String,
    mode: String,
    user_input: String,
) -> AppResult<()> {
    let mode_enum = AgentMode::from_str(&mode);
    let port = *state.server_port.lock().await;
    if port == 0 {
        return Err(AppError::Busy(
            "No model loaded. Load one first.".into(),
        ));
    }
    let settings = state.settings.lock().await.clone();
    let active_model = state.active_model.lock().await.clone().unwrap_or_default();

    let cancel = CancellationToken::new();
    state
        .cancel_tokens
        .lock()
        .await
        .insert(session_id.clone(), cancel.clone());

    let ctx = tools::ToolCtx {
        working_dir: PathBuf::from(&working_dir),
    };

    let sink = Arc::new(TauriSink { app: app.clone() });
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        let res = run_inner(
            sink.as_ref(),
            Some(&state_clone),
            false,            // no auto-allow: la UI aprueba cada Ask
            true,             // persistir sesión
            port,
            &settings,
            &active_model,
            &session_id_clone,
            &working_dir,
            mode_enum,
            &ctx,
            user_input,
            cancel,
        )
        .await;

        state_clone.cancel_tokens.lock().await.remove(&session_id_clone);

        let sink = TauriSink { app };
        match res {
            Ok((text, reason)) => sink.emit_done(&session_id_clone, &text, &reason, None),
            Err(AppError::Other(ref m)) if m == "cancelled" => {
                sink.emit_done(&session_id_clone, "", "cancelled", Some("cancelled"))
            }
            Err(e) => sink.emit_done(&session_id_clone, "", "error", Some(&e.to_string())),
        }
    });

    Ok(())
}

/// Entrada headless: ejecuta UN turno de agente contra un `llama-server` ya levantado en
/// `port`, con permisos auto-aprobados (`auto_allow = true`) y sin persistir sesión. Pensada
/// para harnesses de test/eval que quieren ejercitar el mismo `run_inner` que la app real.
///
/// Devuelve `(texto_final, reason)` donde `reason` ∈ {"done","max_steps","loop","cancelled",...}.
pub async fn run_turn(
    sink: &dyn LoopSink,
    port: u16,
    settings: &Settings,
    active_model: &str,
    working_dir: &str,
    mode: &str,
    user_input: String,
    cancel: CancellationToken,
) -> AppResult<(String, String)> {
    let mode_enum = AgentMode::from_str(mode);
    let ctx = tools::ToolCtx {
        working_dir: PathBuf::from(working_dir),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    run_inner(
        sink,
        None,        // sin AppState en headless
        true,        // auto-approve: el harness decide todo
        false,       // no persistir sesión (evita basura en ~/.local/share)
        port,
        settings,
        active_model,
        &session_id,
        working_dir,
        mode_enum,
        &ctx,
        user_input,
        cancel,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_inner(
    sink: &dyn LoopSink,
    state: Option<&Arc<AppState>>,
    auto_allow: bool,
    persist: bool,
    port: u16,
    settings: &Settings,
    active_model: &str,
    session_id: &str,
    working_dir: &str,
    mode: AgentMode,
    ctx: &tools::ToolCtx,
    user_input: String,
    cancel: CancellationToken,
) -> AppResult<(String, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);
    let registry = Registry::new();
    let docs = registry.docs();
    let native = use_native_tools(&settings.tool_calling, active_model);
    let grammar = tool_call_grammar(&docs);
    let tools_schema = registry.openai_tools();
    tracing::info!(
        "agent: tool-calling route = {} (model: {active_model})",
        if native { "native" } else { "GBNF" }
    );

    // Límite de tokens de la respuesta por paso y presupuesto de contexto para el prompt.
    // Se respeta el `max_tokens` del usuario; solo se acota por el contexto disponible
    // (no por un tope fijo, que truncaba a mitad de un `write_file` con contenido largo).
    let step_max_tokens = (settings.max_tokens as usize)
        .min((settings.context_size as usize).saturating_sub(256).max(256));
    // Presupuesto de contexto para el prompt. Reservamos ~25% del contexto para (respuesta
    // del paso + el error de estimación de `approx_tokens` + margen de seguridad). Esto es
    // crítico: `approx_tokens` (bytes/3) subestima los tokens reales del tokenizer en código
    // y números (~1 token/cácter), lo que con un margen chico hacía desbordar el KV-cache:
    // una conversación que `budget_view` creía dentro del presupuesto era rechazada por
    // llama-server con "request exceeds the available context size" (verificado en eval con
    // Qwen3.5-4B, tarea r3, 8248 tokens reales vs 8192 de ctx).
    let ctx_budget = ((settings.context_size as usize) * 3 / 4)
        .saturating_sub(step_max_tokens)
        .max(512);

    // Load the previous session or start a new one.
    let prev = if persist { session_store::load(session_id) } else { None };
    let title = prev
        .as_ref()
        .map(|s| s.title.clone())
        .unwrap_or_else(|| session_store::make_title(&user_input));
    let created = prev
        .as_ref()
        .map(|s| s.created.clone())
        .unwrap_or_else(session_store::now_iso);

    let mut convo: Vec<AgentMsg> = match prev {
        Some(s) if !s.messages.is_empty() => s.messages,
        _ => {
            let skills = crate::agent::skills::enabled_docs();
            vec![AgentMsg::system(system_prompt(working_dir, &docs, native, &skills))]
        }
    };
    convo.push(AgentMsg::user(user_input));

    // Count identical calls (tool + args) to detect loops.
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut outcome: (String, String) = (
        "Step limit reached before completing the task.".into(),
        "max_steps".into(),
    );

    for step in 0..MAX_STEPS {
        if cancel.is_cancelled() {
            return Err(AppError::Other("cancelled".into()));
        }

        sink.emit_step(session_id, step, "model_start");

        // Al cruzar el presupuesto, compactar: resumir los turnos antiguos con una llamada
        // dedicada al modelo y sustituirlos por un único resumen (en vez de borrarlos).
        match context::maybe_compact(&client, &url, settings, &mut convo, ctx_budget, &cancel)
            .await
        {
            Ok(true) => tracing::info!("context compacted at step {step}"),
            Ok(false) => {}
            Err(AppError::Other(ref m)) if m == "cancelled" => {
                return Err(AppError::Other("cancelled".into()))
            }
            Err(e) => tracing::warn!("compaction: {e}"),
        }

        // `budget_view` queda como red de seguridad barata si aún excede tras compactar.
        let request_msgs = budget_view(&convo, ctx_budget);
        let out = infer_step(
            &client, &url, settings, step_max_tokens, native, &grammar, &tools_schema,
            &request_msgs, sink, session_id, &cancel,
        )
        .await;
        // DEBUG: si la inferencia falla por exceso de contexto, volcar el request para
        // diagnosticar qué mensaje está disparando el desbordamiento (env AGENT_EVAL_DEBUG).
        if let Err(AppError::Inference(ref m)) = out {
            if m.contains("exceeds the available context size") {
                if std::env::var("AGENT_EVAL_DEBUG").is_ok() {
                    eprintln!("\n=== CONTEXT OVERFLOW DEBUG (paso {step}) ===");
                    let total_approx: usize = request_msgs.iter().map(|x| approx_tokens(&x.content) + 4).sum();
                    eprintln!("total approx = {total_approx}, msgs = {}", request_msgs.len());
                    for (i, m) in request_msgs.iter().enumerate() {
                        let a = approx_tokens(&m.content) + 4;
                        let tag = m.tool_name.as_deref().unwrap_or("");
                        let snip: String = m.content.chars().take(120).collect();
                        eprintln!("  [{i}] role={:?} tool={tag} approx={a} len_bytes={} lines={} | {snip:?}",
                            m.role, m.content.len(), m.content.lines().count());
                    }
                    eprintln!("=== fin debug ===\n");
                }
            }
        }
        let out = out?;

        if native {
            // Ruta nativa: sin tool_calls ⇒ el modelo dio su respuesta final en texto.
            if out.tool_calls.is_empty() {
                convo.push(AgentMsg::assistant(out.content.clone()));
                outcome = (out.content, "done".into());
                break;
            }
            convo.push(AgentMsg::assistant_calls(out.content.clone(), out.tool_calls.clone()));
            let mut aborted = false;
            for call in out.tool_calls {
                let flow = process_call(
                    sink, state, auto_allow, &registry, ctx, mode, session_id, step, &cancel,
                    &mut convo, &mut seen, &call.name, &call.args, Some(call.id),
                )
                .await?;
                if let CallFlow::Loop = flow {
                    outcome = (LOOP_MSG.into(), "loop".into());
                    aborted = true;
                    break;
                }
            }
            if aborted {
                break;
            }
        } else {
            // Ruta GBNF: la llamada llega como objeto JSON en el contenido.
            let raw = out.content;
            let (tool, args) = match parse_tool_call(&raw) {
                Ok(v) => v,
                Err(e) => {
                    // La gramática GBNF debería forzar JSON válido; si llegamos aquí fue por
                    // truncado de `max_tokens` a mitad de un string largo o por un caso raro
                    // del modelo. Guardar el raw completo (hasta ~1024 tokens) en el contexto
                    // lo infla sin valor informativo y puede desbordar el KV-cache en pocos
                    // pasos. Truncamos a un prefijo como evidencia para el modelo.
                    let snippet: String = raw.chars().take(400).collect();
                    let ellipsis = if raw.chars().count() > 400 { " ...[truncated]" } else { "" };
                    convo.push(AgentMsg::assistant(format!("{snippet}{ellipsis}")));
                    convo.push(AgentMsg::harness_error(format!(
                        "Your response was not a valid JSON object ({e}). Respond with a single JSON object shaped like {{\"tool\": ..., \"args\": ...}}."
                    )));
                    continue;
                }
            };
            convo.push(AgentMsg::assistant(raw.clone()));
            if tool == "final" {
                let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                outcome = (text, "done".into());
                break;
            }
            let flow = process_call(
                sink, state, auto_allow, &registry, ctx, mode, session_id, step, &cancel,
                &mut convo, &mut seen, &tool, &args, None,
            )
            .await?;
            if let CallFlow::Loop = flow {
                outcome = (LOOP_MSG.into(), "loop".into());
                break;
            }
        }
    }

    // Persist the full session history after completing the turn, if applicable.
    if persist {
        let sess = session_store::StoredSession {
            id: session_id.to_string(),
            title,
            working_dir: working_dir.to_string(),
            mode: mode.as_str().to_string(),
            created,
            updated: session_store::now_iso(),
            messages: convo,
        };
        if let Err(e) = session_store::save(&sess) {
            tracing::warn!("could not save session {}: {e}", session_id);
        }
    }

    Ok(outcome)
}

/// Process one tool call: validate args, detect loops, evaluate permissions, execute,
/// and feed the result back into the conversation.
#[allow(clippy::too_many_arguments)]
async fn process_call(
    sink: &dyn LoopSink,
    state: Option<&Arc<AppState>>,
    auto_allow: bool,
    registry: &Registry,
    ctx: &tools::ToolCtx,
    mode: AgentMode,
    session_id: &str,
    step: usize,
    cancel: &CancellationToken,
    convo: &mut Vec<AgentMsg>,
    seen: &mut std::collections::HashMap<String, usize>,
    tool: &str,
    args: &Value,
    call_id: Option<String>,
) -> AppResult<CallFlow> {
    let args_str = serde_json::to_string(args).unwrap_or_else(|_| "{}".into());

    // Validate args against the tool schema before anything else.
    if let Err(e) = registry.validate(tool, args) {
        let msg = format!("invalid arguments: {e}");
        sink.emit_tool(session_id, step, tool, &args_str, &msg, true);
        convo.push(AgentMsg::tool_result(tool, msg, true).with_call_id(call_id));
        return Ok(CallFlow::Continue);
    }

    // Loop detection: same tool + same args repeated.
    let sig = format!("{tool}:{args_str}");
    let count = {
        let c = seen.entry(sig).or_insert(0);
        *c += 1;
        *c
    };
    if count >= 4 {
        return Ok(CallFlow::Loop);
    }
    if count >= 2 {
        let nudge = format!(
            "You already ran '{tool}' with the same arguments and the result was identical. \
             Do not repeat the same call. If you already have the information you need, provide \
             your final answer."
        );
        sink.emit_tool(session_id, step, tool, &args_str, &nudge, true);
        convo.push(AgentMsg::harness_note(nudge));
        return Ok(CallFlow::Continue);
    }

    // Evaluate permissions based on mode and tool risk.
    let risk = registry.risk(tool);
    let mut decision = match risk {
        Some(r) => permissions::decide(mode, r),
        None => Decision::Allow, // unknown tool: execute() will return the error
    };
    // In headless mode, auto_allow turns Ask into Allow. In Tauri mode, a previous
    // "always allow" skips future asks for this tool in the same session.
    if decision == Decision::Ask {
        if auto_allow {
            decision = Decision::Allow;
        } else if let Some(state) = state {
            let remembered = state
                .session_allow
                .lock()
                .await
                .get(session_id)
                .is_some_and(|set| set.contains(tool));
            if remembered {
                decision = Decision::Allow;
            }
        }
    }
    let allowed = match decision {
        Decision::Allow => true,
        Decision::Deny => {
            let msg = format!("The tool '{tool}' is denied in the current mode (plan).");
            sink.emit_tool(session_id, step, tool, &args_str, &msg, true);
            convo.push(AgentMsg::tool_result(tool, msg, true).with_call_id(call_id));
            return Ok(CallFlow::Continue);
        }
        Decision::Ask => {
            // Only reachable in Tauri mode.
            match state {
                Some(state) => request_permission(sink, state, session_id, tool, args, cancel)
                    .await?,
                None => true,
            }
        }
    };
    if !allowed {
        let msg = "The user denied execution of this tool.".to_string();
        sink.emit_tool(session_id, step, tool, &args_str, &msg, true);
        convo.push(AgentMsg::tool_result(tool, msg, true).with_call_id(call_id));
        return Ok(CallFlow::Continue);
    }

    // Execute tool. The UI receives full output; the model-context copy is capped.
    let (result, is_error) = match registry.execute(tool, args, ctx).await {
        Ok(out) => (out, false),
        Err(e) => (e.to_string(), true),
    };
    sink.emit_tool(session_id, step, tool, &args_str, &result, is_error);
    let capped = cap_for_context(&result);
    convo.push(AgentMsg::tool_result(tool, capped, is_error).with_call_id(call_id));

    // Successful writes/commands change the world, so previous signatures stop being loop evidence.
    if !is_error && matches!(risk, Some(tools::Risk::Write) | Some(tools::Risk::Exec)) {
        seen.clear();
    }
    Ok(CallFlow::Continue)
}

/// Hace una petición de inferencia (nativa o con gramática) y devuelve texto + tool-calls.
#[allow(clippy::too_many_arguments)]
async fn infer_step(
    client: &reqwest::Client,
    url: &str,
    settings: &Settings,
    max_tokens: usize,
    native: bool,
    grammar: &str,
    tools_schema: &Value,
    convo: &[AgentMsg],
    sink: &dyn LoopSink,
    session_id: &str,
    cancel: &CancellationToken,
) -> AppResult<StepOutput> {
    let mut body = serde_json::json!({
        "model": "local",
        "temperature": settings.temperature,
        "top_p": settings.top_p,
        "max_tokens": max_tokens,
        "repeat_penalty": settings.repeat_penalty,
        "stream": true,
        "chat_template_kwargs": { "enable_thinking": settings.enable_thinking },
    });
    if native {
        // Ruta nativa: rol `tool` real + esquema de `tools`; el modelo elige cuándo llamarlas.
        body["messages"] = Value::Array(to_wire_native(convo));
        body["tools"] = tools_schema.clone();
        body["tool_choice"] = Value::String("auto".into());
    } else {
        // Ruta GBNF: la gramática obliga a un JSON de tool-call válido en el contenido.
        body["messages"] = serde_json::to_value(to_wire(convo)).unwrap_or_default();
        body["grammar"] = Value::String(grammar.to_string());
    }

    let resp = client.post(url).json(&body).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Inference(format!(
            "HTTP {}: {}",
            status,
            text.chars().take(500).collect::<String>()
        )));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut content = String::new();
    let mut acc: Vec<AccCall> = Vec::new();

    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(AppError::Other("cancelled".into()));
        }
        let chunk = chunk?;
        buf.push_str(std::str::from_utf8(&chunk).unwrap_or(""));
        while let Some(idx) = buf.find('\n') {
            let line = buf[..idx].trim().to_string();
            buf.drain(..=idx);
            if line.is_empty() {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Ok(build_step_output(content, acc));
                }
                if let Ok(parsed) = serde_json::from_str::<ChatChunk>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(tok) = choice.delta.reasoning_content.as_ref() {
                            if !tok.is_empty() {
                                sink.emit_token(session_id, tok, true);
                            }
                        }
                        if let Some(tok) = choice.delta.content.as_ref() {
                            if !tok.is_empty() {
                                content.push_str(tok);
                                sink.emit_token(session_id, tok, false);
                            }
                        }
                        if let Some(tcs) = choice.delta.tool_calls.as_ref() {
                            for tc in tcs {
                                if tc.index >= acc.len() {
                                    acc.resize_with(tc.index + 1, AccCall::default);
                                }
                                let slot = &mut acc[tc.index];
                                if let Some(id) = &tc.id {
                                    if !id.is_empty() {
                                        slot.id = id.clone();
                                    }
                                }
                                if let Some(f) = &tc.function {
                                    if let Some(name) = &f.name {
                                        if !name.is_empty() {
                                            slot.name = name.clone();
                                        }
                                    }
                                    if let Some(a) = &f.arguments {
                                        slot.args.push_str(a);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(build_step_output(content, acc))
}

/// Convierte los acumuladores de llamadas (fragmentadas en el stream) en `ToolCall` listos.
fn build_step_output(content: String, acc: Vec<AccCall>) -> StepOutput {
    let mut tool_calls = Vec::new();
    for (i, c) in acc.into_iter().enumerate() {
        if c.name.is_empty() {
            continue;
        }
        let args = if c.args.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(&c.args).unwrap_or_else(|_| Value::Object(Default::default()))
        };
        let id = if c.id.is_empty() {
            format!("call_{i}")
        } else {
            c.id
        };
        tool_calls.push(ToolCall {
            id,
            name: c.name,
            args,
        });
    }
    StepOutput {
        content,
        tool_calls,
    }
}

/// Extrae `{tool, args}` de la respuesta del modelo, tolerando texto sobrante.
pub fn parse_tool_call(raw: &str) -> Result<(String, Value), String> {
    let json_slice = extract_json_object(raw).ok_or("no JSON object was found")?;
    let v: Value = serde_json::from_str(json_slice).map_err(|e| e.to_string())?;
    let tool = v
        .get("tool")
        .and_then(|t| t.as_str())
        .ok_or("falta el campo 'tool'")?
        .to_string();
    let args = v.get("args").cloned().unwrap_or(Value::Object(Default::default()));
    Ok((tool, args))
}

/// Devuelve el primer bloque balanceado `{...}` del texto.
fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Return a trimmed copy of the conversation that fits the token budget while preserving
/// the system prompt and original user task.
fn budget_view(convo: &[AgentMsg], budget: usize) -> Vec<AgentMsg> {
    let total = |c: &[AgentMsg]| -> usize {
        c.iter().map(|m| approx_tokens(&m.content) + 4).sum()
    };
    if convo.len() <= 3 || total(convo) <= budget {
        return convo.to_vec();
    }
    let mut view = convo.to_vec();
    let mut removed = 0;
    while view.len() > 3 && total(&view) > budget {
        view.remove(2);
        removed += 1;
    }
    if removed > 0 {
        view.insert(
            2,
            AgentMsg::harness_note(format!(
                "[... {removed} earlier messages omitted to save context ...]"
            )),
        );
    }
    view
}

/// Request user confirmation and wait for the response or cancellation.
async fn request_permission(
    sink: &dyn LoopSink,
    state: &Arc<AppState>,
    session_id: &str,
    tool: &str,
    args: &Value,
    cancel: &CancellationToken,
) -> AppResult<bool> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel::<permissions::PermissionResponse>();
    state
        .pending_permissions
        .lock()
        .await
        .insert(request_id.clone(), tx);

    sink.emit_permission(
        session_id,
        &request_id,
        tool,
        &serde_json::to_string(args).unwrap_or_else(|_| "{}".into()),
        &permission_summary(tool, args),
    );

    let resp = tokio::select! {
        _ = cancel.cancelled() => {
            state.pending_permissions.lock().await.remove(&request_id);
            return Err(AppError::Other("cancelled".into()));
        }
        r = rx => r.unwrap_or(permissions::PermissionResponse { approved: false, remember: false }),
    };
    state.pending_permissions.lock().await.remove(&request_id);
    if resp.approved && resp.remember {
        state
            .session_allow
            .lock()
            .await
            .entry(session_id.to_string())
            .or_default()
            .insert(tool.to_string());
    }
    Ok(resp.approved)
}

/// Readable summary of the action that needs confirmation.
fn permission_summary(tool: &str, args: &Value) -> String {
    let get = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("");
    match tool {
        "bash" => format!("Run: {}", get("command")),
        "write_file" => format!("Write file: {}", get("path")),
        "edit" => format!("Edit file: {}", get("path")),
        _ => format!("Run {tool}"),
    }
}

/// Character cap for a tool result inserted into model context. The UI receives the full output.
const RESULT_CTX_CAP: usize = 6000;

fn cap_for_context(s: &str) -> String {
    let total = s.chars().count();
    if total <= RESULT_CTX_CAP {
        return s.to_string();
    }
    let head: String = s.chars().take(RESULT_CTX_CAP).collect();
    format!(
        "{head}\n\n... [output truncated to {RESULT_CTX_CAP} of {total} characters to save context; use offset/limit or grep to inspect the rest]"
    )
}
