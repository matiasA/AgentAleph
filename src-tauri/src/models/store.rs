use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub path: String,
    pub size_bytes: u64,
    pub exists: bool,
}

/// Escanea una o más carpetas (recursivamente) en busca de GGUF, deduplicando por ruta.
pub fn list_local(dirs: &[PathBuf]) -> AppResult<Vec<LocalModel>> {
    let mut out: Vec<LocalModel> = vec![];
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for root in dirs {
        if !root.exists() {
            continue;
        }
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            let rd = match std::fs::read_dir(&dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                if p.extension().and_then(|e| e.to_str()) != Some("gguf") {
                    continue;
                }
                let path_str = p.to_string_lossy().into_owned();
                if !seen.insert(path_str.clone()) {
                    continue;
                }
                let meta = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let size = meta.len();
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                out.push(LocalModel {
                    path: path_str,
                    name: p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?")
                        .into(),
                    size_bytes: size,
                    size_human: human_size(size),
                    modified,
                });
            }
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(out)
}

pub fn delete_model(path: &str) -> AppResult<()> {
    let p = PathBuf::from(path);
    if !p.exists() {
        return Err(AppError::NotFound(format!("No existe: {}", path)));
    }
    std::fs::remove_file(&p)?;
    let parent = p.parent();
    if let Some(parent) = parent {
        if let Ok(rd) = std::fs::read_dir(parent) {
            if rd.filter_map(|e| e.ok()).next().is_none() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }
    Ok(())
}

pub fn model_info(path: &str) -> ModelInfo {
    let p = PathBuf::from(path);
    let exists = p.exists();
    let size_bytes = p.metadata().map(|m| m.len()).unwrap_or(0);
    ModelInfo {
        path: path.into(),
        size_bytes,
        exists,
    }
}

fn human_size(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.2} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}
