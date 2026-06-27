use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use crate::state::ChatMsg;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize)]
pub struct ChatTokenEvent {
    pub session_id: String,
    pub token: String,
    pub is_reasoning: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatDoneEvent {
    pub session_id: String,
    pub full: String,
    pub reasoning: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChunk {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatChoice {
    delta: ChatDelta,
}

#[derive(Debug, Clone, Deserialize)]
struct ChatDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

pub async fn send_chat(
    app: AppHandle,
    state: Arc<crate::state::AppState>,
    session_id: String,
    messages: Vec<ChatMsg>,
) -> AppResult<()> {
    let port = *state.server_port.lock().await;
    if port == 0 {
        return Err(AppError::Busy(
            "No model loaded. Load one first.".into(),
        ));
    }

    let settings = state.settings.lock().await.clone();

    // Construir messages con system prompt al inicio si existe
    let mut full = Vec::with_capacity(messages.len() + 1);
    let sys = settings.system_prompt.trim();
    if !sys.is_empty() {
        full.push(ChatMsg {
            role: "system".into(),
            content: sys.to_string(),
        });
    }
    full.extend(messages.iter().cloned());

    let body = serde_json::json!({
        "model": "local",
        "messages": full,
        "temperature": settings.temperature,
        "top_p": settings.top_p,
        "max_tokens": settings.max_tokens,
        "repeat_penalty": settings.repeat_penalty,
        "stream": true,
        "chat_template_kwargs": {
            "enable_thinking": settings.enable_thinking,
        },
    });

    let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let cancel = CancellationToken::new();
    state.cancel_tokens.lock().await.insert(session_id.clone(), cancel.clone());

    // Marcar sesión como generando
    {
        let mut sessions = state.sessions.lock().await;
        if let Some(s) = sessions.get_mut(&session_id) {
            s.generating = true;
        } else {
            sessions.insert(
                session_id.clone(),
                crate::state::ChatSession {
                    id: session_id.clone(),
                    messages: messages.clone(),
                    generating: true,
                },
            );
        }
    }

    let app_clone = app.clone();
    let state_clone = state.clone();
    let session_id_clone = session_id.clone();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        let result = run_stream(&client, &url, body, &app_clone, &session_id_clone, cancel_clone).await;

        let _ = state_clone.cancel_tokens.lock().await.remove(&session_id_clone);
        {
            let mut sessions = state_clone.sessions.lock().await;
            if let Some(s) = sessions.get_mut(&session_id_clone) {
                s.generating = false;
            }
        }

        match result {
            Ok((full_text, reasoning_text)) => {
                let _ = app_clone.emit(
                    "chat://done",
                    ChatDoneEvent {
                        session_id: session_id_clone,
                        full: full_text,
                        reasoning: reasoning_text,
                        error: None,
                    },
                );
            }
            Err(AppError::Other(ref m)) if m == "cancelled" => {
                let _ = app_clone.emit(
                    "chat://done",
                    ChatDoneEvent {
                        session_id: session_id_clone,
                        full: String::new(),
                        reasoning: String::new(),
                        error: Some("cancelled".into()),
                    },
                );
            }
            Err(e) => {
                let _ = app_clone.emit(
                    "chat://done",
                    ChatDoneEvent {
                        session_id: session_id_clone,
                        full: String::new(),
                        reasoning: String::new(),
                        error: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(())
}

async fn run_stream(
    client: &reqwest::Client,
    url: &str,
    body: serde_json::Value,
    app: &AppHandle,
    session_id: &str,
    cancel: CancellationToken,
) -> AppResult<(String, String)> {
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
    let mut full = String::new();
    let mut reasoning = String::new();

    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(AppError::Other("cancelled".to_string()));
        }
        let chunk = chunk?;
        buf.push_str(std::str::from_utf8(&chunk).unwrap_or(""));
        while let Some(idx) = buf.find("\n") {
            let line = buf[..idx].trim().to_string();
            buf.drain(..=idx);
            if line.is_empty() {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Ok((full, reasoning));
                }
                if let Ok(parsed) = serde_json::from_str::<ChatChunk>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(tok) = choice.delta.reasoning_content.as_ref() {
                            if !tok.is_empty() {
                                reasoning.push_str(tok);
                                let _ = app.emit(
                                    "chat://token",
                                    ChatTokenEvent {
                                        session_id: session_id.to_string(),
                                        token: tok.clone(),
                                        is_reasoning: true,
                                    },
                                );
                            }
                        }
                        if let Some(tok) = choice.delta.content.as_ref() {
                            if !tok.is_empty() {
                                full.push_str(tok);
                                let _ = app.emit(
                                    "chat://token",
                                    ChatTokenEvent {
                                        session_id: session_id.to_string(),
                                        token: tok.clone(),
                                        is_reasoning: false,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok((full, reasoning))
}

pub async fn stop_chat(state: &Arc<crate::state::AppState>, session_id: &str) -> AppResult<()> {
    if let Some(t) = state.cancel_tokens.lock().await.get(session_id) {
        t.cancel();
    }
    Ok(())
}

#[allow(dead_code)]
pub fn _unused_settings(_s: Settings) {}
