use crate::agent::message::AgentMsg;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sesión de agente persistida en disco (fuente de verdad para reanudar y mostrar).
#[derive(Serialize, Deserialize, Clone)]
pub struct StoredSession {
    pub id: String,
    pub title: String,
    pub working_dir: String,
    pub mode: String,
    pub created: String,
    pub updated: String,
    /// Conversación completa con el modelo (incluye el system prompt en la posición 0).
    pub messages: Vec<AgentMsg>,
}

/// Metadatos ligeros para listar sesiones sin cargar toda la conversación.
#[derive(Serialize, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub updated: String,
    pub working_dir: String,
}

pub fn sessions_dir() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("agent-aleph");
    p.push("agent-sessions");
    p
}

fn session_path(id: &str) -> PathBuf {
    sessions_dir().join(format!("{id}.json"))
}

pub fn load(id: &str) -> Option<StoredSession> {
    let s = std::fs::read_to_string(session_path(id)).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn save(sess: &StoredSession) -> AppResult<()> {
    std::fs::create_dir_all(sessions_dir())?;
    std::fs::write(session_path(&sess.id), serde_json::to_string_pretty(sess)?)?;
    Ok(())
}

pub fn delete(id: &str) -> AppResult<()> {
    let p = session_path(id);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub fn list() -> Vec<SessionMeta> {
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(sessions_dir()) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(ss) = serde_json::from_str::<StoredSession>(&s) {
                    out.push(SessionMeta {
                        id: ss.id,
                        title: ss.title,
                        updated: ss.updated,
                        working_dir: ss.working_dir,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated.cmp(&a.updated));
    out
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Deriva un título corto a partir del primer mensaje del usuario.
pub fn make_title(s: &str) -> String {
    let t: String = s.trim().chars().take(50).collect();
    if t.is_empty() {
        "Session".into()
    } else {
        t
    }
}
