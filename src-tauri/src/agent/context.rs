//! Gestión de contexto: compactación con resumen.
//!
//! Los modelos locales tienen contexto pequeño (4k–32k). Cuando la conversación supera el
//! presupuesto, en vez de borrar mensajes del medio (que pierde información), resumimos los
//! turnos antiguos con una llamada dedicada al modelo y los sustituimos por un único mensaje
//! de resumen que conserva objetivos, decisiones y estado actual.

use crate::agent::message::{approx_tokens, AgentMsg, Role};
use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

/// Cabecera siempre preservada: system prompt (0) + tarea original del usuario (1).
const HEAD: usize = 2;
/// Tope de tokens del resumen generado.
const SUMMARY_MAX_TOKENS: usize = 512;
/// Temperatura baja para un resumen estable y fiel.
const SUMMARY_TEMP: f32 = 0.3;

/// Suma aproximada de tokens de la conversación (mismo criterio que `budget_view`).
pub fn total_tokens(convo: &[AgentMsg]) -> usize {
    convo.iter().map(|m| approx_tokens(&m.content) + 4).sum()
}

#[derive(Deserialize)]
struct CompletionResp {
    choices: Vec<CompletionChoice>,
}
#[derive(Deserialize)]
struct CompletionChoice {
    message: CompletionMessage,
}
#[derive(Deserialize)]
struct CompletionMessage {
    content: Option<String>,
}

/// Si la conversación supera `budget`, resume los turnos antiguos y los reemplaza por un único
/// mensaje de resumen, mutando `convo` in situ. Conserva la cabecera (`HEAD`) y los mensajes
/// recientes que quepan en ~la mitad del presupuesto. Devuelve `true` si compactó.
///
/// Es caro (un round-trip al modelo), así que solo dispara al cruzar el umbral; tras compactar
/// la conversación queda por debajo del presupuesto y no vuelve a hacerlo hasta que crezca.
pub async fn maybe_compact(
    client: &reqwest::Client,
    url: &str,
    settings: &Settings,
    convo: &mut Vec<AgentMsg>,
    budget: usize,
    cancel: &CancellationToken,
) -> AppResult<bool> {
    // Necesitamos al menos cabecera + algo de medio + algo de cola para que compactar valga.
    if convo.len() < HEAD + 4 || total_tokens(convo) <= budget {
        return Ok(false);
    }

    // Conservar los mensajes recientes que quepan en la mitad del presupuesto.
    let tail_budget = (budget / 2).max(256);
    let mut tail_start = convo.len();
    let mut acc = 0usize;
    while tail_start > HEAD {
        let cost = approx_tokens(&convo[tail_start - 1].content) + 4;
        if acc + cost > tail_budget {
            break;
        }
        acc += cost;
        tail_start -= 1;
    }

    // El medio a resumir es [HEAD, tail_start). Si no hay nada que resumir, no hacemos nada.
    if tail_start <= HEAD {
        return Ok(false);
    }
    let middle = &convo[HEAD..tail_start];
    if middle.is_empty() {
        return Ok(false);
    }

    let transcript = render_transcript(middle);
    let summary = match summarize(client, url, settings, &transcript, cancel).await {
        Ok(s) if !s.trim().is_empty() => s,
        // Si el resumen falla o sale vacío, no rompemos el turno: `budget_view` recortará.
        Ok(_) => return Ok(false),
        Err(AppError::Other(ref m)) if m == "cancelled" => return Err(AppError::Other(m.clone())),
        Err(e) => {
            tracing::warn!("compactación: falló el resumen, se omite: {e}");
            return Ok(false);
        }
    };

    let note = AgentMsg::harness_note(format!(
        "Resumen de los pasos previos (compactados para ahorrar contexto):\n{}",
        summary.trim()
    ));
    convo.splice(HEAD..tail_start, std::iter::once(note));
    Ok(true)
}

/// Renderiza el tramo a resumir de forma legible para el modelo resumidor.
fn render_transcript(msgs: &[AgentMsg]) -> String {
    let mut out = String::new();
    for m in msgs {
        let label = match m.role {
            Role::System => "NOTA",
            Role::User => "USUARIO",
            Role::Assistant => "ASISTENTE",
            Role::Tool => "RESULTADO",
        };
        let tag = m.tool_name.as_deref().unwrap_or("");
        if m.role == Role::Tool && !tag.is_empty() {
            out.push_str(&format!("[{label}:{tag}] {}\n", m.content));
        } else {
            out.push_str(&format!("[{label}] {}\n", m.content));
        }
    }
    out
}

/// Llamada dedicada (no-streaming, sin gramática) que resume el tramo dado.
async fn summarize(
    client: &reqwest::Client,
    url: &str,
    settings: &Settings,
    transcript: &str,
    cancel: &CancellationToken,
) -> AppResult<String> {
    let body = serde_json::json!({
        "model": "local",
        "messages": [
            {
                "role": "system",
                "content": "Eres un asistente que resume el historial de una sesión de un agente \
                    de programación para liberar contexto. Resume en español, de forma concisa y \
                    fiel, conservando: el objetivo de la tarea, las decisiones tomadas, los \
                    hallazgos relevantes (archivos, rutas, símbolos), los cambios ya aplicados y \
                    el estado actual / próximos pasos. No inventes nada que no aparezca. No uses \
                    herramientas ni JSON: responde solo con el texto del resumen."
            },
            {
                "role": "user",
                "content": format!("Historial a resumir:\n\n{transcript}")
            }
        ],
        "temperature": SUMMARY_TEMP,
        "top_p": settings.top_p,
        "repeat_penalty": settings.repeat_penalty,
        "max_tokens": SUMMARY_MAX_TOKENS,
        "stream": false,
        // El resumen no debe gastar presupuesto en razonamiento.
        "chat_template_kwargs": { "enable_thinking": false },
    });

    let send = client.post(url).json(&body).send();
    let resp = tokio::select! {
        _ = cancel.cancelled() => return Err(AppError::Other("cancelled".into())),
        r = send => r?,
    };
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Inference(format!(
            "HTTP {} al resumir: {}",
            status,
            text.chars().take(300).collect::<String>()
        )));
    }
    let parsed: CompletionResp = resp.json().await?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();
    Ok(content)
}
