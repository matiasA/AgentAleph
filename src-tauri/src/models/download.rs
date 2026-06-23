use crate::error::{AppError, AppResult};
use crate::state::DownloadState;
use crate::state::DownloadStatus;
use bytes::Bytes;
use futures_util::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

const HF_BASE: &str = "https://huggingface.co";

pub async fn download_model(
    app: AppHandle,
    state: Arc<crate::state::AppState>,
    repo: String,
    filename: String,
) -> AppResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let models_dir = state.models_dir.clone();

    // Subdir por repo (sanitizado) para evitar colisiones
    let safe_repo = repo.replace('/', "__");
    let dest_dir = models_dir.join(&safe_repo);
    std::fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(&filename);
    let tmp_path = dest_path.with_extension("gguf.part");

    let entry = DownloadState {
        id: id.clone(),
        repo: repo.clone(),
        filename: filename.clone(),
        downloaded: 0,
        total: 0,
        speed_bps: 0,
        status: DownloadStatus::Pending,
    };
    state.downloads.lock().await.insert(id.clone(), entry.clone());
    emit(&app, &entry);

    let url = format!("{}/{}/resolve/main/{}", HF_BASE, repo, filename);
    let cancel = CancellationToken::new();
    state.cancel_tokens.lock().await.insert(id.clone(), cancel.clone());

    let app_clone = app.clone();
    let state_clone = state.clone();
    let id_clone = id.clone();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        let result = run_download(
            &url,
            &tmp_path,
            &dest_path,
            &app_clone,
            &state_clone,
            &id_clone,
            cancel_clone,
        )
        .await;

        let mut downloads = state_clone.downloads.lock().await;
        if let Some(e) = downloads.get_mut(&id_clone) {
            match result {
                Ok(()) => {
                    e.status = DownloadStatus::Completed;
                    e.downloaded = e.total;
                }
                Err(AppError::Other(ref msg)) if msg == "cancelled" => {
                    e.status = DownloadStatus::Cancelled;
                }
                Err(ref err) => {
                    e.status = DownloadStatus::Failed(err.to_string());
                }
            }
            emit(&app_clone, e);
        }
        state_clone.cancel_tokens.lock().await.remove(&id_clone);
    });

    Ok(id)
}

async fn run_download(
    url: &str,
    tmp_path: &PathBuf,
    dest_path: &PathBuf,
    app: &AppHandle,
    state: &Arc<crate::state::AppState>,
    id: &str,
    cancel: CancellationToken,
) -> AppResult<()> {
    let client = reqwest::Client::builder()
        .user_agent("agent-aleph/0.1")
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "HTTP {} al descargar {}",
            resp.status(),
            url
        )));
    }
    let total = resp.content_length().unwrap_or(0);

    {
        let mut downloads = state.downloads.lock().await;
        if let Some(e) = downloads.get_mut(id) {
            e.total = total;
            e.status = DownloadStatus::Downloading;
            emit(app, e);
        }
    }

    let mut file = tokio::fs::File::create(tmp_path).await?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();
    let mut last_bytes: u64 = 0;
    let chunk_min = 64 * 1024;

    while let Some(chunk) = stream.next().await {
        if cancel.is_cancelled() {
            drop(file);
            let _ = tokio::fs::remove_file(tmp_path).await;
            return Err(AppError::Other("cancelled".to_string()));
        }
        let chunk: Bytes = chunk?;
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed() > std::time::Duration::from_millis(200) {
            let elapsed = last_emit.elapsed().as_secs_f64().max(0.001);
            let speed = ((downloaded - last_bytes) as f64 / elapsed) as u64;
            last_emit = std::time::Instant::now();
            last_bytes = downloaded;

            let mut downloads = state.downloads.lock().await;
            if let Some(e) = downloads.get_mut(id) {
                e.downloaded = downloaded;
                e.speed_bps = speed;
                emit(app, e);
            }
        }

        if downloaded < chunk_min {
            tokio::task::yield_now().await;
        }
    }
    file.flush().await?;
    drop(file);

    tokio::fs::rename(tmp_path, dest_path).await?;

    let mut downloads = state.downloads.lock().await;
    if let Some(e) = downloads.get_mut(id) {
        e.downloaded = downloaded;
        e.total = if total == 0 { downloaded } else { total };
    }
    Ok(())
}

pub async fn cancel_download(state: &Arc<crate::state::AppState>, id: &str) -> AppResult<()> {
    if let Some(token) = state.cancel_tokens.lock().await.get(id) {
        token.cancel();
    }
    Ok(())
}

fn emit(app: &AppHandle, e: &DownloadState) {
    let _ = app.emit("download://progress", e);
}
