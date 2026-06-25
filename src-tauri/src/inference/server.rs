use crate::error::{AppError, AppResult};
use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::process::{Child, Command};

pub struct ServerHandle {
    pub child: Child,
    pub port: u16,
    pub model: String,
}

impl ServerHandle {
    pub async fn kill(&mut self) -> AppResult<()> {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
        Ok(())
    }
}

pub fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr().map(|a| a.port()))
        .unwrap_or(8080)
}

pub fn llama_binary_path() -> AppResult<PathBuf> {
    let candidates: Vec<PathBuf> = vec![
        std::env::current_dir()?.join("binaries/llama-linux-x64/llama-server"),
        PathBuf::from("binaries/llama-linux-x64/llama-server"),
        dirs::data_dir()
            .map(|d| d.join("agent-aleph/binaries/llama-linux-x64/llama-server"))
            .unwrap_or_else(|| PathBuf::from("llama-server")),
    ];

    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    Err(AppError::NotFound(
        "No se encontró el binario llama-server".into(),
    ))
}

pub fn llama_lib_dir() -> AppResult<PathBuf> {
    let bin = llama_binary_path()?;
    Ok(bin
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".")))
}

/// Progreso de carga de un modelo, emitido vía evento `model://loading`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoadProgress {
    pub model: String,
    pub model_name: String,
    pub phase: String,
    pub percent: u8,
    pub error: Option<String>,
}

/// Emisor de progreso monótono: nunca retrocede el porcentaje y deduplica el
/// último valor enviado. Clonable para compartirlo entre tareas (poller de IO y
/// parser de stderr).
#[derive(Clone)]
struct ProgressEmitter {
    app: AppHandle,
    model: String,
    model_name: String,
    cur: Arc<AtomicU8>,
}

impl ProgressEmitter {
    fn new(app: &AppHandle, model_path: &str) -> Self {
        let model_name = Path::new(model_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        Self {
            app: app.clone(),
            model: model_path.to_string(),
            model_name,
            cur: Arc::new(AtomicU8::new(0)),
        }
    }

    /// Fija el porcentaje (sólo si avanza) y emite el evento.
    fn set(&self, target: u8, phase: &str) {
        let prev = self.cur.load(Ordering::SeqCst);
        if target <= prev {
            return;
        }
        self.cur.store(target, Ordering::SeqCst);
        let _ = self.app.emit(
            "model://loading",
            LoadProgress {
                model: self.model.clone(),
                model_name: self.model_name.clone(),
                phase: phase.to_string(),
                percent: target,
                error: None,
            },
        );
    }

    /// Interpola una fracción [0,1] dentro de una banda [lo,hi] de porcentaje.
    fn set_band(&self, frac: f64, lo: u8, hi: u8, phase: &str) {
        let frac = frac.clamp(0.0, 1.0);
        let span = (hi - lo) as f64;
        let target = lo + (span * frac).round() as u8;
        self.set(target, phase);
    }

    fn reached(&self, p: u8) -> bool {
        self.cur.load(Ordering::SeqCst) >= p
    }

    /// Emite un fallo terminal de carga.
    fn fail(&self, msg: &str) {
        let _ = self.app.emit(
            "model://loading",
            LoadProgress {
                model: self.model.clone(),
                model_name: self.model_name.clone(),
                phase: "Error".into(),
                percent: self.cur.load(Ordering::SeqCst),
                error: Some(msg.to_string()),
            },
        );
    }
}

/// Lee bytes leídos por el proceso desde `/proc/<pid>/io`. Devuelve el máximo
/// entre `read_bytes` (lecturas físicas de disco, cuenta también los page-faults
/// de mmap en frío) y `rchar` (bytes vía syscalls read, ruta sin mmap), de modo
/// que funciona tanto con mmap como con `--no-mmap`.
fn proc_read_bytes(pid: u32) -> Option<u64> {
    let txt = std::fs::read_to_string(format!("/proc/{pid}/io")).ok()?;
    let mut read_bytes = 0u64;
    let mut rchar = 0u64;
    for line in txt.lines() {
        if let Some(v) = line.strip_prefix("read_bytes:") {
            read_bytes = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("rchar:") {
            rchar = v.trim().parse().unwrap_or(0);
        }
    }
    Some(read_bytes.max(rchar))
}

pub async fn start_server(
    app: &AppHandle,
    model: &str,
    settings: &Settings,
) -> AppResult<ServerHandle> {
    let bin = llama_binary_path()?;
    let lib_dir = llama_lib_dir()?;
    let port = find_free_port();

    if !Path::new(model).exists() {
        return Err(AppError::NotFound(format!(
            "Modelo no encontrado: {}",
            model
        )));
    }

    let progress = ProgressEmitter::new(app, model);
    progress.set(3, "Iniciando llama-server");
    let model_size = std::fs::metadata(model).map(|m| m.len()).unwrap_or(0);

    let mut cmd = Command::new(&bin);
    cmd.env("LD_LIBRARY_PATH", &lib_dir);
    cmd.arg("--model").arg(model);
    cmd.arg("--port").arg(port.to_string());
    cmd.arg("--host").arg("127.0.0.1");
    cmd.arg("--ctx-size").arg(settings.context_size.to_string());
    cmd.arg("--temp").arg(settings.temperature.to_string());
    cmd.arg("--top-p").arg(settings.top_p.to_string());
    cmd.arg("--repeat-penalty").arg(settings.repeat_penalty.to_string());
    cmd.arg("--threads").arg(settings.threads.to_string());
    // En modo auto NO fijamos --n-gpu-layers: llama.cpp (-fit) reparte capas según la
    // memoria libre de cada dispositivo. Fijarlo aborta el auto-fit y puede causar OOM.
    if !settings.gpu_layers_auto {
        cmd.arg("--n-gpu-layers").arg(settings.n_gpu_layers.to_string());
    }
    cmd.arg("--flash-attn").arg("auto");
    cmd.arg("--batch-size").arg(settings.n_batch.to_string());

    // `--jinja` habilita la plantilla del modelo (rol `tool`, `tool_calls`), requisito del
    // tool-calling nativo. Solo se omite si el usuario fuerza la ruta GBNF universal, que usa
    // la plantilla legacy probada.
    if settings.tool_calling != "grammar" {
        cmd.arg("--jinja");
    }

    // Cuantización de la caché KV: reduce la memoria del contexto (requiere flash-attn).
    if !settings.cache_type_k.is_empty() && settings.cache_type_k != "f16" {
        cmd.arg("--cache-type-k").arg(&settings.cache_type_k);
    }
    if !settings.cache_type_v.is_empty() && settings.cache_type_v != "f16" {
        cmd.arg("--cache-type-v").arg(&settings.cache_type_v);
    }

    // Gestión de memoria del modelo.
    if !settings.use_mmap {
        cmd.arg("--no-mmap");
    }
    if settings.use_mlock {
        cmd.arg("--mlock");
    }
    if !settings.device.is_empty() && settings.device != "cpu" {
        if settings.device == "auto" {
            // dejar que llama-server elija
        } else {
            cmd.arg("--device").arg(&settings.device);
        }
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    cmd.kill_on_drop(true);

    tracing::info!("Lanzando llama-server en puerto {} con modelo {}", port, model);
    let mut child = cmd.spawn().map_err(|e| {
        let msg = format!("No se pudo lanzar llama-server: {e}");
        progress.fail(&msg);
        AppError::Inference(msg)
    })?;

    // Poller de progreso de carga (banda 15..82%). Combina dos señales y toma el
    // máximo, de modo que la barra siempre avanza:
    //   1. IO real: bytes leídos del .gguf vs su tamaño (/proc/<pid>/io). Fiable
    //      con --no-mmap o caché en frío, pero con mmap + caché caliente los
    //      contadores no se mueven (los modelos pesados se quedaban en 15%).
    //   2. Estimación temporal: curva asintótica que se acerca a la cima de la
    //      banda sin alcanzarla; garantiza avance aunque (1) no dé señal.
    if let (Some(pid), true) = (child.id(), model_size > 0) {
        let progress = progress.clone();
        tokio::spawn(async move {
            let start = std::time::Instant::now();
            loop {
                if progress.reached(85) {
                    break; // las fases de stderr tomaron el relevo
                }
                let io_frac = match proc_read_bytes(pid) {
                    Some(read) => read as f64 / model_size as f64,
                    None => break, // el proceso ya no existe
                };
                // tau=20s: ~63% de la banda a los 20s, ~95% a los 60s, nunca 100%.
                let time_frac = 1.0 - (-start.elapsed().as_secs_f64() / 20.0).exp();
                progress.set_band(io_frac.max(time_frac), 15, 82, "Cargando pesos del modelo");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
    }

    // Log stdout en background; stderr además emite fases de progreso.
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(log_pipe(stdout, "llama.stdout"));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(progress_pipe(stderr, progress.clone()));
    }

    // Esperar a que /health responda
    if let Err(e) = wait_for_ready(port, std::time::Duration::from_secs(120)).await {
        progress.fail(&e.to_string());
        let _ = child.kill().await;
        return Err(e);
    }

    progress.set(100, "Listo");

    Ok(ServerHandle {
        child,
        port,
        model: model.to_string(),
    })
}

/// Lee stderr de llama-server: lo registra (como `log_pipe`) y, además, traduce
/// los marcadores de fase a porcentajes de carga emitidos vía `model://loading`.
async fn progress_pipe<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    mut r: R,
    progress: ProgressEmitter,
) {
    use tokio::io::AsyncBufReadExt;
    let mut buf = tokio::io::BufReader::new(&mut r);
    let mut line = String::new();
    loop {
        line.clear();
        match buf.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                tracing::info!("[llama.stderr] {}", trimmed);
                // Marcadores conocidos de llama.cpp (orden de aparición).
                if trimmed.contains("loading model") {
                    progress.set(8, "Leyendo modelo");
                } else if trimmed.contains("fitting params to device memory") {
                    progress.set(15, "Ajustando a la memoria del dispositivo");
                } else if trimmed.contains("warming up the model") {
                    progress.set(85, "Calentando el modelo");
                } else if trimmed.contains("model loaded") {
                    progress.set(92, "Modelo cargado");
                } else if trimmed.contains("server is listening") {
                    progress.set(96, "Servidor escuchando");
                }
            }
            Err(_) => break,
        }
    }
}

async fn log_pipe<R: tokio::io::AsyncRead + Unpin + Send + 'static>(mut r: R, tag: &'static str) {
    use tokio::io::AsyncBufReadExt;
    let mut buf = tokio::io::BufReader::new(&mut r);
    let mut line = String::new();
    loop {
        line.clear();
        match buf.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    tracing::info!("[{}] {}", tag, trimmed);
                }
            }
            Err(_) => break,
        }
    }
}

pub async fn wait_for_ready(port: u16, timeout: std::time::Duration) -> AppResult<()> {
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let start = std::time::Instant::now();
    let mut last_err = String::new();
    while start.elapsed() < timeout {
        match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => return Ok(()),
            Ok(r) => last_err = format!("status {}", r.status()),
            Err(e) => last_err = e.to_string(),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    Err(AppError::Inference(format!(
        "llama-server no respondió en {}s (último: {})",
        timeout.as_secs(),
        last_err
    )))
}

pub async fn stop_server(handle: &mut ServerHandle) -> AppResult<()> {
    handle.kill().await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GpuDevice {
    pub id: String,
    pub name: String,
    pub total_mb: u64,
    pub free_mb: u64,
}

pub async fn list_gpu_devices() -> AppResult<Vec<GpuDevice>> {
    let bin = llama_binary_path()?;
    let lib_dir = llama_lib_dir()?;
    let mut cmd = Command::new(&bin);
    cmd.env("LD_LIBRARY_PATH", &lib_dir);
    cmd.arg("--list-devices");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    let output = cmd.output().await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    let mut devices = vec![];
    for line in combined.lines() {
        // Formato: "  Vulkan1: NVIDIA GeForce GTX 1070 (8438 MiB, 8251 MiB free)"
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Vulkan") {
            if let Some((id_str, after)) = rest.split_once(": ") {
                let id = format!("Vulkan{}", id_str);
                // parsear nombre y memoria
                let name = after.rsplit_once(" (").map(|(n, _)| n).unwrap_or(after);
                let total_mb = extract_mb(after, "");
                let free_mb = extract_mb(after, "free");
                devices.push(GpuDevice {
                    id,
                    name: name.to_string(),
                    total_mb,
                    free_mb,
                });
            }
        }
    }
    Ok(devices)
}

/// Memoria RAM del sistema (Linux, vía `/proc/meminfo`). `free_mb` usa
/// `MemAvailable` (memoria realmente utilizable sin swap agresivo).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemMemory {
    pub total_mb: u64,
    pub free_mb: u64,
}

pub fn read_system_memory() -> SystemMemory {
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    if let Ok(txt) = std::fs::read_to_string("/proc/meminfo") {
        for line in txt.lines() {
            if let Some(v) = line.strip_prefix("MemTotal:") {
                total_kb = parse_meminfo_kb(v);
            } else if let Some(v) = line.strip_prefix("MemAvailable:") {
                avail_kb = parse_meminfo_kb(v);
            }
        }
    }
    SystemMemory {
        total_mb: total_kb / 1024,
        free_mb: avail_kb / 1024,
    }
}

fn parse_meminfo_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn extract_mb(s: &str, marker: &str) -> u64 {
    // busca el primer "NNNN MiB" que venga seguido (o precedido por marker)
    let needle = if marker.is_empty() { "MiB" } else { "MiB free" };
    if let Some(idx) = s.find(needle) {
        let prefix = &s[..idx];
        let num: String = prefix
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("")
            .to_string();
        if let Ok(n) = num.parse::<u64>() {
            return n;
        }
    }
    0
}
