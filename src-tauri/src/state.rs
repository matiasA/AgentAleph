use crate::error::AppResult;
use crate::inference::ServerHandle;
use crate::settings::Settings;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadState {
    pub id: String,
    pub repo: String,
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: u64,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatSession {
    pub id: String,
    pub messages: Vec<ChatMsg>,
    pub generating: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMsg {
    pub role: String,
    pub content: String,
}

pub struct AppState {
    pub models_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub server: Mutex<Option<ServerHandle>>,
    pub server_port: Mutex<u16>,
    pub active_model: Mutex<Option<String>>,
    pub downloads: Mutex<std::collections::HashMap<String, DownloadState>>,
    pub sessions: Mutex<std::collections::HashMap<String, ChatSession>>,
    pub cancel_tokens: Mutex<std::collections::HashMap<String, tokio_util::sync::CancellationToken>>,
    /// Permisos de agente en espera de confirmación del usuario, por request id.
    pub pending_permissions:
        Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<bool>>>,
}

impl AppState {
    pub fn new() -> Self {
        let models_dir = {
            let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push("agent-aleph");
            p.push("models");
            p
        };
        Self {
            models_dir,
            settings: Mutex::new(Settings::load()),
            server: Mutex::new(None),
            server_port: Mutex::new(0),
            active_model: Mutex::new(None),
            downloads: Mutex::new(Default::default()),
            sessions: Mutex::new(Default::default()),
            cancel_tokens: Mutex::new(Default::default()),
            pending_permissions: Mutex::new(Default::default()),
        }
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        std::fs::create_dir_all(&self.models_dir)?;
        let cfg = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(cfg.join("agent-aleph"))?;
        Ok(())
    }
}

pub fn shared_state<'a>(state: &'a tauri::State<'a, Arc<AppState>>) -> &'a Arc<AppState> {
    state.inner()
}
