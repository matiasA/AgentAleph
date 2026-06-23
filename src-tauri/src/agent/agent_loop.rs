use crate::agent::permissions::{self, AgentMode, Decision};
use crate::agent::session_store;
use crate::agent::{grammar::tool_call_grammar, prompt::system_prompt, tools, tools::Registry};
use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::state::{AppState, ChatMsg};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

/// Tope duro de pasos por turno para evitar bucles infinitos en modelos débiles.
const MAX_STEPS: usize = 25;

// ---------- Eventos hacia el frontend ----------

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
    pub reason: String, // "done" | "max_steps" | "cancelled" | "error"
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
}

/// Lanza un turno de agente en una tarea en segundo plano y emite eventos `agent://*`.
pub async fn run_agent(
    app: AppHandle,
    state: Arc<AppState>,
    session_id: String,
    working_dir: String,
    mode: String,
    user_input: String,
) -> AppResult<()> {
    let mode = AgentMode::from_str(&mode);
    let port = *state.server_port.lock().await;
    if port == 0 {
        return Err(AppError::Busy(
            "No hay modelo cargado. Carga uno primero.".into(),
        ));
    }
    let settings = state.settings.lock().await.clone();

    let cancel = CancellationToken::new();
    state
        .cancel_tokens
        .lock()
        .await
        .insert(session_id.clone(), cancel.clone());

    let ctx = tools::ToolCtx {
        working_dir: PathBuf::from(&working_dir),
    };

    tokio::spawn(async move {
        let res = run_inner(
            &app,
            &state,
            port,
            &settings,
            &session_id,
            &working_dir,
            mode,
            &ctx,
            user_input,
            cancel,
        )
        .await;

        state.cancel_tokens.lock().await.remove(&session_id);

        match res {
            Ok((text, reason)) => emit_done(&app, &session_id, &text, &reason, None),
            Err(AppError::Other(ref m)) if m == "cancelled" => {
                emit_done(&app, &session_id, "", "cancelled", Some("cancelled".into()))
            }
            Err(e) => emit_done(&app, &session_id, "", "error", Some(e.to_string())),
        }
    });

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_inner(
    app: &AppHandle,
    state: &Arc<AppState>,
    port: u16,
    settings: &Settings,
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
    let grammar = tool_call_grammar(&docs);

    // Límite de tokens de la respuesta por paso y presupuesto de contexto para el prompt.
    let step_max_tokens = (settings.max_tokens as usize).min(1024);
    let ctx_budget = (settings.context_size as usize)
        .saturating_sub(step_max_tokens + 256)
        .max(512);

    // Cargar la sesión previa (memoria entre turnos) o iniciar una nueva.
    let prev = session_store::load(session_id);
    let title = prev
        .as_ref()
        .map(|s| s.title.clone())
        .unwrap_or_else(|| session_store::make_title(&user_input));
    let created = prev
        .as_ref()
        .map(|s| s.created.clone())
        .unwrap_or_else(session_store::now_iso);

    let mut convo: Vec<ChatMsg> = match prev {
        Some(s) if !s.messages.is_empty() => s.messages,
        _ => vec![ChatMsg {
            role: "system".into(),
            content: system_prompt(working_dir, &docs),
        }],
    };
    convo.push(ChatMsg {
        role: "user".into(),
        content: user_input,
    });

    // Conteo de llamadas idénticas (herramienta + args) para detectar bucles.
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut outcome: (String, String) = (
        "Se alcanzó el límite de pasos sin completar la tarea.".into(),
        "max_steps".into(),
    );

    for step in 0..MAX_STEPS {
        if cancel.is_cancelled() {
            return Err(AppError::Other("cancelled".into()));
        }

        let _ = app.emit(
            "agent://step",
            AgentStepEvent {
                session_id: session_id.to_string(),
                step,
                phase: "model_start".into(),
            },
        );

        // Se envía una vista recortada al presupuesto; `convo` conserva el historial completo.
        let request_msgs = budget_view(&convo, ctx_budget);
        let raw = infer_step(
            &client, &url, settings, step_max_tokens, &grammar, &request_msgs, app, session_id,
            &cancel,
        )
        .await?;

        let (tool, args) = match parse_tool_call(&raw) {
            Ok(v) => v,
            Err(e) => {
                // Modelo emitió algo no parseable: devolvemos feedback y reintentamos.
                convo.push(ChatMsg {
                    role: "assistant".into(),
                    content: raw.clone(),
                });
                convo.push(ChatMsg {
                    role: "user".into(),
                    content: format!(
                        "Error: tu respuesta no fue un objeto JSON válido ({e}). Responde con un único objeto JSON con la forma {{\"tool\": ..., \"args\": ...}}."
                    ),
                });
                continue;
            }
        };

        // Guardamos la decisión del modelo en la conversación.
        convo.push(ChatMsg {
            role: "assistant".into(),
            content: raw.clone(),
        });

        if tool == "final" {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            outcome = (text, "done".into());
            break;
        }

        let args_str = serde_json::to_string(&args).unwrap_or_else(|_| "{}".into());

        // Validar args contra el esquema de la herramienta antes de cualquier otra cosa.
        if let Err(e) = registry.validate(&tool, &args) {
            let msg = format!("argumentos inválidos: {e}");
            emit_tool(app, session_id, step, &tool, &args_str, &msg, true);
            convo.push(ChatMsg {
                role: "user".into(),
                content: format!("Resultado de {tool} (ERROR):\n{msg}"),
            });
            continue;
        }

        // Detección de bucles: misma herramienta + args repetida.
        let sig = format!("{tool}:{args_str}");
        let count = {
            let c = seen.entry(sig).or_insert(0);
            *c += 1;
            *c
        };
        if count >= 4 {
            outcome = (
                "Me quedé repitiendo la misma acción sin avanzar. Detengo la tarea; \
                 revisa el resultado de los pasos anteriores."
                    .into(),
                "loop".into(),
            );
            break;
        }
        if count >= 2 {
            let nudge = format!(
                "Ya ejecutaste '{tool}' con esos mismos argumentos y el resultado fue idéntico. \
                 No repitas la misma llamada. Si ya tienes la información necesaria, responde con \
                 la herramienta 'final'."
            );
            emit_tool(app, session_id, step, &tool, &args_str, &nudge, true);
            convo.push(ChatMsg {
                role: "user".into(),
                content: nudge,
            });
            continue;
        }

        // Evaluar permisos según el modo y el riesgo de la herramienta.
        let risk = registry.risk(&tool);
        let decision = match risk {
            Some(r) => permissions::decide(mode, r),
            None => Decision::Allow, // herramienta desconocida: execute() devolverá error
        };

        let allowed = match decision {
            Decision::Allow => true,
            Decision::Deny => {
                let msg = format!(
                    "La herramienta '{tool}' está denegada en el modo actual (plan)."
                );
                emit_tool(app, session_id, step, &tool, &args_str, &msg, true);
                convo.push(ChatMsg {
                    role: "user".into(),
                    content: format!("Resultado de {tool} (ERROR):\n{msg}"),
                });
                continue;
            }
            Decision::Ask => {
                request_permission(app, state, session_id, &tool, &args, &cancel).await?
            }
        };

        if !allowed {
            let msg = "El usuario denegó la ejecución de esta herramienta.".to_string();
            emit_tool(app, session_id, step, &tool, &args_str, &msg, true);
            convo.push(ChatMsg {
                role: "user".into(),
                content: format!("Resultado de {tool} (ERROR):\n{msg}"),
            });
            continue;
        }

        // Ejecutar herramienta.
        let (result, is_error) = match registry.execute(&tool, &args, ctx).await {
            Ok(out) => (out, false),
            Err(e) => (e.to_string(), true),
        };

        emit_tool(app, session_id, step, &tool, &args_str, &result, is_error);

        // Reinyectar el resultado como mensaje de usuario (compatible con cualquier plantilla).
        convo.push(ChatMsg {
            role: "user".into(),
            content: format!(
                "Resultado de {tool}{}:\n{result}",
                if is_error { " (ERROR)" } else { "" }
            ),
        });
    }

    // Persistir la sesión (historial completo) tras completar el turno.
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
        tracing::warn!("no se pudo guardar la sesión {}: {e}", session_id);
    }

    Ok(outcome)
}

/// Hace una petición de inferencia con gramática y devuelve el contenido acumulado.
#[allow(clippy::too_many_arguments)]
async fn infer_step(
    client: &reqwest::Client,
    url: &str,
    settings: &Settings,
    max_tokens: usize,
    grammar: &str,
    convo: &[ChatMsg],
    app: &AppHandle,
    session_id: &str,
    cancel: &CancellationToken,
) -> AppResult<String> {
    let body = serde_json::json!({
        "model": "local",
        "messages": convo,
        "temperature": settings.temperature,
        "top_p": settings.top_p,
        "max_tokens": max_tokens,
        "repeat_penalty": settings.repeat_penalty,
        "grammar": grammar,
        "stream": true,
        "chat_template_kwargs": { "enable_thinking": settings.enable_thinking },
    });

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
                    return Ok(content);
                }
                if let Ok(parsed) = serde_json::from_str::<ChatChunk>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(tok) = choice.delta.reasoning_content.as_ref() {
                            if !tok.is_empty() {
                                emit_token(app, session_id, tok, true);
                            }
                        }
                        if let Some(tok) = choice.delta.content.as_ref() {
                            if !tok.is_empty() {
                                content.push_str(tok);
                                emit_token(app, session_id, tok, false);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(content)
}

/// Extrae `{tool, args}` de la respuesta del modelo, tolerando texto sobrante.
fn parse_tool_call(raw: &str) -> Result<(String, Value), String> {
    let json_slice = extract_json_object(raw).ok_or("no se encontró un objeto JSON")?;
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

/// Estimación conservadora de tokens (sobreestima para no desbordar el contexto).
fn approx_tokens(s: &str) -> usize {
    s.len() / 3 + 1
}

/// Devuelve una copia de la conversación recortada para que quepa en `budget` tokens,
/// preservando el system prompt (índice 0) y la tarea original (índice 1). `convo` original
/// no se modifica (conserva el historial completo para persistencia).
fn budget_view(convo: &[ChatMsg], budget: usize) -> Vec<ChatMsg> {
    let total = |c: &[ChatMsg]| -> usize {
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
            ChatMsg {
                role: "user".into(),
                content: format!("[… {removed} mensajes anteriores omitidos para ahorrar contexto …]"),
            },
        );
    }
    view
}

/// Solicita confirmación al usuario y espera su respuesta (o la cancelación del turno).
async fn request_permission(
    app: &AppHandle,
    state: &Arc<AppState>,
    session_id: &str,
    tool: &str,
    args: &Value,
    cancel: &CancellationToken,
) -> AppResult<bool> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
    state
        .pending_permissions
        .lock()
        .await
        .insert(request_id.clone(), tx);

    let _ = app.emit(
        "agent://permission",
        AgentPermissionEvent {
            session_id: session_id.to_string(),
            request_id: request_id.clone(),
            tool: tool.to_string(),
            args: serde_json::to_string(args).unwrap_or_else(|_| "{}".into()),
            summary: permission_summary(tool, args),
        },
    );

    let approved = tokio::select! {
        _ = cancel.cancelled() => {
            state.pending_permissions.lock().await.remove(&request_id);
            return Err(AppError::Other("cancelled".into()));
        }
        r = rx => r.unwrap_or(false),
    };
    state.pending_permissions.lock().await.remove(&request_id);
    Ok(approved)
}

/// Resumen legible de la acción que se va a confirmar.
fn permission_summary(tool: &str, args: &Value) -> String {
    let get = |k: &str| args.get(k).and_then(|v| v.as_str()).unwrap_or("");
    match tool {
        "bash" => format!("Ejecutar: {}", get("command")),
        "write_file" => format!("Escribir archivo: {}", get("path")),
        "edit" => format!("Editar archivo: {}", get("path")),
        _ => format!("Ejecutar {tool}"),
    }
}

fn emit_tool(
    app: &AppHandle,
    session_id: &str,
    step: usize,
    tool: &str,
    args: &str,
    result: &str,
    is_error: bool,
) {
    let _ = app.emit(
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

fn emit_token(app: &AppHandle, session_id: &str, token: &str, is_reasoning: bool) {
    let _ = app.emit(
        "agent://token",
        AgentTokenEvent {
            session_id: session_id.to_string(),
            token: token.to_string(),
            is_reasoning,
        },
    );
}

fn emit_done(app: &AppHandle, session_id: &str, text: &str, reason: &str, error: Option<String>) {
    let _ = app.emit(
        "agent://done",
        AgentDoneEvent {
            session_id: session_id.to_string(),
            text: text.to_string(),
            reason: reason.to_string(),
            error,
        },
    );
}
