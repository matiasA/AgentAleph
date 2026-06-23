use crate::error::AppResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub system_prompt: String,
    pub context_size: u32,
    pub n_gpu_layers: u32,
    pub threads: u32,
    pub device: String,
    pub enable_thinking: bool,
    /// Si es true, NO se pasa --n-gpu-layers y llama.cpp reparte capas automáticamente (-fit).
    #[serde(default = "default_true")]
    pub gpu_layers_auto: bool,
    /// Cuantización de la caché KV (clave): "f16" | "q8_0" | "q4_0". Ahorra memoria de contexto.
    #[serde(default = "default_cache_type")]
    pub cache_type_k: String,
    /// Cuantización de la caché KV (valor): "f16" | "q8_0" | "q4_0".
    #[serde(default = "default_cache_type")]
    pub cache_type_v: String,
    /// Tamaño de batch para el prompt (throughput de prefill).
    #[serde(default = "default_batch")]
    pub n_batch: u32,
    /// Usar mmap para cargar el modelo (true = menos RAM; false = carga completa a RAM).
    #[serde(default = "default_true")]
    pub use_mmap: bool,
    /// Fijar el modelo en RAM con mlock (evita swap; requiere RAM suficiente).
    #[serde(default)]
    pub use_mlock: bool,
    /// Carpetas externas (fuera de models_dir) donde también buscar GGUF.
    #[serde(default)]
    pub extra_model_dirs: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 0.9,
            repeat_penalty: 1.1,
            system_prompt: "You are a helpful assistant.".into(),
            context_size: 4096,
            n_gpu_layers: 99,
            threads: num_threads(),
            device: "auto".into(),
            enable_thinking: false,
            gpu_layers_auto: true,
            cache_type_k: default_cache_type(),
            cache_type_v: default_cache_type(),
            n_batch: default_batch(),
            use_mmap: default_true(),
            use_mlock: false,
            extra_model_dirs: Vec::new(),
        }
    }
}

fn num_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

fn default_cache_type() -> String {
    "f16".into()
}

fn default_batch() -> u32 {
    2048
}

fn default_true() -> bool {
    true
}

impl Settings {
    pub fn config_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("agent-aleph");
        p.push("settings.json");
        p
    }

    pub fn load() -> Self {
        match std::fs::read_to_string(Self::config_path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> AppResult<()> {
        let p = Self::config_path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(p, s)?;
        Ok(())
    }
}
