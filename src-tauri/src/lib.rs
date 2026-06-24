pub mod agent;
pub mod chat;
pub mod commands;
pub mod error;
pub mod inference;
pub mod models;
pub mod settings;
pub mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,agent_aleph_lib=debug".into()),
        )
        .with_target(false)
        .init();

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .manage(Arc::new(app_state))
        .invoke_handler(tauri::generate_handler![
            commands::list_catalog_models,
            commands::search_hf,
            commands::browse_hf,
            commands::list_model_files,
            commands::list_local_models,
            commands::list_model_dirs,
            commands::add_model_dir,
            commands::remove_model_dir,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::load_model,
            commands::unload_model,
            commands::model_status,
            commands::send_chat,
            commands::stop_chat,
            commands::agent_send,
            commands::agent_stop,
            commands::respond_permission,
            commands::list_agent_sessions,
            commands::load_agent_session,
            commands::delete_agent_session,
            commands::get_settings,
            commands::save_settings,
            commands::get_app_info,
            commands::list_gpus,
            commands::list_skills,
            commands::set_skill_enabled,
            commands::create_skill,
            commands::import_skill,
            commands::delete_skill,
            commands::read_skill,
            commands::read_context_file,
        ])
        .setup(|app| {
            let state: tauri::State<'_, Arc<AppState>> = app.state();
            state.ensure_dirs().map_err(|e| {
                let boxed: Box<dyn std::error::Error> = Box::new(e);
                boxed
            })?;
            tracing::info!(
                "Agent Aleph iniciado. Modelos en: {:?}",
                state.models_dir
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
