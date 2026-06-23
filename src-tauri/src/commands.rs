use crate::chat;
use crate::error::AppResult;
use crate::inference;
use crate::models;
use crate::settings::Settings;
use crate::state::ChatMsg;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub loaded: bool,
    pub model: Option<String>,
    pub model_name: Option<String>,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub models_dir: String,
    pub llama_binary: Option<String>,
    pub os: String,
    pub arch: String,
}

#[tauri::command]
pub async fn list_catalog_models() -> AppResult<Vec<models::CatalogModel>> {
    Ok(models::list_catalog())
}

#[tauri::command]
pub async fn search_hf(query: String) -> AppResult<Vec<models::HfModel>> {
    models::search_hf(&query).await
}

#[tauri::command]
pub async fn list_model_files(repo: String) -> AppResult<Vec<models::HfFile>> {
    models::list_model_files(&repo).await
}

#[tauri::command]
pub async fn list_local_models(
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<Vec<models::LocalModel>> {
    let mut dirs = vec![state.models_dir.clone()];
    let extra = state.settings.lock().await.extra_model_dirs.clone();
    dirs.extend(extra.into_iter().map(std::path::PathBuf::from));
    models::list_local(&dirs)
}

#[tauri::command]
pub async fn list_model_dirs(
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<Vec<String>> {
    Ok(state.settings.lock().await.extra_model_dirs.clone())
}

#[tauri::command]
pub async fn add_model_dir(
    state: State<'_, Arc<crate::state::AppState>>,
    path: String,
) -> AppResult<Vec<String>> {
    let mut s = state.settings.lock().await;
    if !s.extra_model_dirs.contains(&path) {
        s.extra_model_dirs.push(path);
    }
    s.save()?;
    Ok(s.extra_model_dirs.clone())
}

#[tauri::command]
pub async fn remove_model_dir(
    state: State<'_, Arc<crate::state::AppState>>,
    path: String,
) -> AppResult<Vec<String>> {
    let mut s = state.settings.lock().await;
    s.extra_model_dirs.retain(|d| d != &path);
    s.save()?;
    Ok(s.extra_model_dirs.clone())
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    repo: String,
    filename: String,
) -> AppResult<String> {
    models::download_model(app, state.inner().clone(), repo, filename).await
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, Arc<crate::state::AppState>>,
    id: String,
) -> AppResult<()> {
    models::cancel_download(state.inner(), &id).await
}

#[tauri::command]
pub async fn delete_model(path: String) -> AppResult<()> {
    models::delete_model(&path)
}

#[tauri::command]
pub async fn load_model(
    app: AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    path: String,
) -> AppResult<()> {
    // Detener servidor existente si lo hay
    {
        let mut server = state.server.lock().await;
        if let Some(mut h) = server.take() {
            tracing::info!("Deteniendo servidor anterior...");
            let _ = inference::stop_server(&mut h).await;
        }
    }
    *state.server_port.lock().await = 0;
    *state.active_model.lock().await = None;

    let settings = state.settings.lock().await.clone();
    let handle = inference::start_server(&app, &path, &settings).await?;
    let port = handle.port;
    let model = handle.model.clone();
    *state.server_port.lock().await = port;
    *state.active_model.lock().await = Some(model.clone());
    *state.server.lock().await = Some(handle);

    let _ = app.emit("model://status", model_status_inner(&state).await);
    Ok(())
}

#[tauri::command]
pub async fn unload_model(
    app: AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<()> {
    {
        let mut server = state.server.lock().await;
        if let Some(mut h) = server.take() {
            let _ = inference::stop_server(&mut h).await;
        }
    }
    *state.server_port.lock().await = 0;
    *state.active_model.lock().await = None;
    let _ = app.emit("model://status", model_status_inner(&state).await);
    Ok(())
}

#[tauri::command]
pub async fn model_status(
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<ModelStatus> {
    Ok(model_status_inner(&state).await)
}

async fn model_status_inner(state: &Arc<crate::state::AppState>) -> ModelStatus {
    let active = state.active_model.lock().await.clone();
    let port = *state.server_port.lock().await;
    let model_name = active.as_ref().map(|p| {
        std::path::Path::new(p)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string()
    });
    ModelStatus {
        loaded: active.is_some(),
        model: active,
        model_name,
        port,
    }
}

#[tauri::command]
pub async fn send_chat(
    app: AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    session_id: String,
    messages: Vec<ChatMsg>,
) -> AppResult<()> {
    chat::send_chat(app, state.inner().clone(), session_id, messages).await
}

#[tauri::command]
pub async fn stop_chat(
    state: State<'_, Arc<crate::state::AppState>>,
    session_id: String,
) -> AppResult<()> {
    chat::stop_chat(state.inner(), &session_id).await
}

#[tauri::command]
pub async fn agent_send(
    app: AppHandle,
    state: State<'_, Arc<crate::state::AppState>>,
    session_id: String,
    working_dir: String,
    mode: String,
    input: String,
) -> AppResult<()> {
    crate::agent::run_agent(
        app,
        state.inner().clone(),
        session_id,
        working_dir,
        mode,
        input,
    )
    .await
}

#[tauri::command]
pub async fn agent_stop(
    state: State<'_, Arc<crate::state::AppState>>,
    session_id: String,
) -> AppResult<()> {
    if let Some(t) = state.cancel_tokens.lock().await.get(&session_id) {
        t.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn list_agent_sessions() -> AppResult<Vec<crate::agent::session_store::SessionMeta>> {
    Ok(crate::agent::session_store::list())
}

#[tauri::command]
pub async fn load_agent_session(
    id: String,
) -> AppResult<Option<crate::agent::session_store::StoredSession>> {
    Ok(crate::agent::session_store::load(&id))
}

#[tauri::command]
pub async fn delete_agent_session(id: String) -> AppResult<()> {
    crate::agent::session_store::delete(&id)
}

#[tauri::command]
pub async fn respond_permission(
    state: State<'_, Arc<crate::state::AppState>>,
    request_id: String,
    approved: bool,
) -> AppResult<()> {
    if let Some(tx) = state.pending_permissions.lock().await.remove(&request_id) {
        let _ = tx.send(approved);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_settings(
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<Settings> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, Arc<crate::state::AppState>>,
    settings: Settings,
) -> AppResult<()> {
    settings.save()?;
    *state.settings.lock().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn get_app_info(
    state: State<'_, Arc<crate::state::AppState>>,
) -> AppResult<AppInfo> {
    let llama = inference::llama_binary_path().ok().map(|p| p.to_string_lossy().into_owned());
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        models_dir: state.models_dir.to_string_lossy().into_owned(),
        llama_binary: llama,
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
    })
}

#[tauri::command]
pub async fn list_gpus() -> AppResult<Vec<inference::GpuDevice>> {
    inference::list_gpu_devices().await
}

#[allow(dead_code)]
fn _unused(_state: &State<'_, Arc<crate::state::AppState>>) {}
