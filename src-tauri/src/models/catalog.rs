use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    pub author: String,
    pub repo: String,
    pub description: String,
    pub params: String,
    pub default_file: String,
    pub size_gb: f64,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Tema/uso para navegar el Hub por intención (Código, Legal, Sin censura…).
/// Cada tema dispara búsquedas curadas contra HF y, para temas de pool delgado,
/// destaca primero generalistas fuertes del catálogo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: String,
    /// "rich" (pool grande: el chip busca directo en HF) | "niche" (pool delgado:
    /// primero generalistas recomendados, luego especializados de HF).
    pub tier: String,
    #[serde(default)]
    pub blurb: String,
    /// Términos lanzados contra `/api/models?filter=gguf&search=`.
    pub hf_queries: Vec<String>,
    /// Filtro opcional de idioma del Hub (ej. "es"). Vacío = sin filtro.
    #[serde(default)]
    pub hf_lang: String,
    /// ids de modelos del catálogo a destacar como "generalistas recomendados".
    #[serde(default)]
    pub recommended_model_ids: Vec<String>,
    /// Aviso/caption bajo el tema (disclaimers, contenido adulto…).
    #[serde(default)]
    pub note: Option<String>,
}

/// Estructura del `catalog.json` (local para test; en el futuro, remoto cacheado).
/// Cada lista, si viene vacía o el archivo falta, cae al equivalente embebido.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CatalogFile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub models: Vec<CatalogModel>,
    #[serde(default)]
    pub topics: Vec<Topic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfModel {
    pub repo: String,
    pub author: String,
    pub name: String,
    pub downloads: u64,
    pub likes: u64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfFile {
    pub r#ref: String,
    pub path: String,
    pub size: u64,
}

fn curated() -> Vec<CatalogModel> {
    vec![
        // ==================== Tiny (CPU, any hardware) ====================
        CatalogModel {
            id: "qwen3.5-0.8b".into(),
            name: "Qwen 3.5 0.8B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-0.8B-GGUF".into(),
            description: "The smallest Qwen 3.5 model. Fits on almost any machine. Ideal for quick tests.".into(),
            params: "0.8B".into(),
            default_file: "Qwen3.5-0.8B-Q4_K_M.gguf".into(),
            size_gb: 0.53,
            category: "Tiny".into(),
            tags: vec!["qwen", "alibaba", "tiny", "multilingual", "cpu"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-2b".into(),
            name: "Qwen 3.5 2B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-2B-GGUF".into(),
            description: "Qwen 3.5 2B. Excellent multilingual model, fast on CPU, with strong quality for its size.".into(),
            params: "2B".into(),
            default_file: "Qwen3.5-2B-Q4_K_M.gguf".into(),
            size_gb: 1.28,
            category: "Tiny".into(),
            tags: vec!["qwen", "alibaba", "tiny", "multilingual", "cpu"].into_iter().map(String::from).collect(),
        },

        // ==================== Light (basic CPU, 8GB RAM) ====================
        CatalogModel {
            id: "phi-4-mini".into(),
            name: "Phi-4 Mini Instruct".into(),
            author: "unsloth".into(),
            repo: "unsloth/Phi-4-mini-instruct-GGUF".into(),
            description: "Microsoft Phi-4 Mini. Strong reasoning in a compact size. Excellent for code and logic.".into(),
            params: "3.8B".into(),
            default_file: "Phi-4-mini-instruct-Q4_K_M.gguf".into(),
            size_gb: 2.49,
            category: "Light".into(),
            tags: vec!["microsoft", "phi", "light", "reasoning", "code"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "phi-4-mini-reasoning".into(),
            name: "Phi-4 Mini Reasoning".into(),
            author: "unsloth".into(),
            repo: "unsloth/Phi-4-mini-reasoning-GGUF".into(),
            description: "Phi-4 Mini variant optimized for step-by-step reasoning.".into(),
            params: "3.8B".into(),
            default_file: "Phi-4-mini-reasoning-Q4_K_M.gguf".into(),
            size_gb: 2.49,
            category: "Light".into(),
            tags: vec!["microsoft", "phi", "light", "reasoning", "thinking"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-4b".into(),
            name: "Qwen 3.5 4B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-4B-GGUF".into(),
            description: "Qwen 3.5 4B. Excellent balance of quality and CPU speed, with strong multilingual support.".into(),
            params: "4B".into(),
            default_file: "Qwen3.5-4B-Q4_K_M.gguf".into(),
            size_gb: 2.74,
            category: "Light".into(),
            tags: vec!["qwen", "alibaba", "light", "multilingual", "cpu"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-e2b".into(),
            name: "Gemma 4 E2B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-E2B-it-GGUF".into(),
            description: "Google Gemma 4 E2B. Efficient, high quality for its size, and good for general chat.".into(),
            params: "2B".into(),
            default_file: "gemma-4-E2B-it-Q4_K_M.gguf".into(),
            size_gb: 3.11,
            category: "Light".into(),
            tags: vec!["google", "gemma", "light", "efficient"].into_iter().map(String::from).collect(),
        },

        // ==================== Medium (16GB CPU / 6GB+ GPU) ====================
        CatalogModel {
            id: "gemma-4-e4b".into(),
            name: "Gemma 4 E4B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-E4B-it-GGUF".into(),
            description: "Google Gemma 4 E4B. Medium-sized model with an excellent quality-to-resource ratio.".into(),
            params: "4B".into(),
            default_file: "gemma-4-E4B-it-Q4_K_M.gguf".into(),
            size_gb: 4.98,
            category: "Medium".into(),
            tags: vec!["google", "gemma", "medium"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-9b".into(),
            name: "Qwen 3.5 9B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-9B-GGUF".into(),
            description: "Qwen 3.5 9B. The most popular size in the line. Solid reasoning, excellent multilingual quality. GPU recommended.".into(),
            params: "9B".into(),
            default_file: "Qwen3.5-9B-Q4_K_M.gguf".into(),
            size_gb: 5.68,
            category: "Medium".into(),
            tags: vec!["qwen", "alibaba", "medium", "multilingual", "popular"].into_iter().map(String::from).collect(),
        },

        // ==================== Heavy (8-12GB GPU) ====================
        CatalogModel {
            id: "gemma-4-12b".into(),
            name: "Gemma 4 12B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-12b-it-GGUF".into(),
            description: "Google Gemma 4 12B. Powerful Google model with advanced reasoning. 8GB+ GPU recommended.".into(),
            params: "12B".into(),
            default_file: "gemma-4-12b-it-Q4_K_M.gguf".into(),
            size_gb: 7.12,
            category: "Heavy".into(),
            tags: vec!["google", "gemma", "heavy", "powerful"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-12b-qat".into(),
            name: "Gemma 4 12B IT (QAT)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-12B-it-qat-GGUF".into(),
            description: "QAT quantization-trained variant of Gemma 4 12B. Lighter and higher quality than standard Q4_K_M at the same bit level.".into(),
            params: "12B".into(),
            default_file: "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf".into(),
            size_gb: 6.72,
            category: "Heavy".into(),
            tags: vec!["google", "gemma", "heavy", "qat", "new"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "phi-4-14b".into(),
            name: "Phi-4 (14B)".into(),
            author: "MaziyarPanahi".into(),
            repo: "MaziyarPanahi/phi-4-GGUF".into(),
            description: "Microsoft Phi-4 14B. High-end reasoning, competitive with larger models.".into(),
            params: "14B".into(),
            default_file: "phi-4.Q4_K_M.gguf".into(),
            size_gb: 9.05,
            category: "Heavy".into(),
            tags: vec!["microsoft", "phi", "heavy", "reasoning", "code"].into_iter().map(String::from).collect(),
        },

        // ==================== MoE / Very heavy (16-24GB+ GPU) ====================
        CatalogModel {
            id: "qwen3.5-27b".into(),
            name: "Qwen 3.5 27B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-27B-GGUF".into(),
            description: "Qwen 3.5 27B. Deep reasoning and quality close to commercial models. 16GB+ GPU recommended.".into(),
            params: "27B".into(),
            default_file: "Qwen3.5-27B-Q4_K_M.gguf".into(),
            size_gb: 16.74,
            category: "MoE / Heavy+".into(),
            tags: vec!["qwen", "alibaba", "moe", "heavy", "powerful", "multilingual"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.6-27b".into(),
            name: "Qwen 3.6 27B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.6-27B-GGUF".into(),
            description: "Qwen 3.6 27B. Latest Alibaba generation, with better reasoning and context than 3.5.".into(),
            params: "27B".into(),
            default_file: "Qwen3.6-27B-Q4_K_M.gguf".into(),
            size_gb: 16.82,
            category: "MoE / Heavy+".into(),
            tags: vec!["qwen", "alibaba", "moe", "heavy", "new", "2026", "multilingual"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-26b-a4b".into(),
            name: "Gemma 4 26B A4B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".into(),
            description: "Google Gemma 4 26B MoE: 26B total parameters but only 4B active. Small-model speed with large-model quality.".into(),
            params: "26B (4B active)".into(),
            default_file: "gemma-4-26B-A4B-it-UD-Q4_K_M.gguf".into(),
            size_gb: 16.95,
            category: "MoE / Heavy+".into(),
            tags: vec!["google", "gemma", "moe", "heavy", "efficient", "new", "2026"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-26b-a4b-qat".into(),
            name: "Gemma 4 26B A4B (QAT MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-26B-A4B-it-qat-GGUF".into(),
            description: "QAT variant of Gemma 4 26B A4B MoE. Same architecture, 4B active parameters, better quality per bit and lower weight than the standard version.".into(),
            params: "26B (4B active)".into(),
            default_file: "gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf".into(),
            size_gb: 14.25,
            category: "MoE / Heavy+".into(),
            tags: vec!["google", "gemma", "moe", "heavy", "efficient", "qat", "new"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3-30b-a3b".into(),
            name: "Qwen3 30B A3B Instruct 2507".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3-30B-A3B-Instruct-2507-GGUF".into(),
            description: "Qwen3 MoE: 30B total parameters, 3B active. Fast like a 3B, powerful like a 30B. 2507 version.".into(),
            params: "30B (3B active)".into(),
            default_file: "Qwen3-30B-A3B-Instruct-2507-Q4_K_M.gguf".into(),
            size_gb: 18.56,
            category: "MoE / Heavy+".into(),
            tags: vec!["qwen", "alibaba", "moe", "heavy", "efficient", "multilingual"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-35b-a3b".into(),
            name: "Qwen 3.5 35B A3B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-35B-A3B-GGUF".into(),
            description: "Qwen 3.5 MoE 35B: 35B total parameters, 3B active. Current Qwen MoE generation. 16GB+ GPU recommended.".into(),
            params: "35B (3B active)".into(),
            default_file: "Qwen3.5-35B-A3B-Q4_K_M.gguf".into(),
            size_gb: 22.02,
            category: "MoE / Heavy+".into(),
            tags: vec!["qwen", "alibaba", "moe", "heavy", "efficient", "new", "multilingual"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.6-35b-a3b".into(),
            name: "Qwen 3.6 35B A3B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.6-35B-A3B-GGUF".into(),
            description: "Qwen 3.6 MoE 35B: latest generation. 35B total parameters, 3B active. Alibaba's newest MoE line as of June 2026.".into(),
            params: "35B (3B active)".into(),
            default_file: "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf".into(),
            size_gb: 22.13,
            category: "MoE / Heavy+".into(),
            tags: vec!["qwen", "alibaba", "moe", "heavy", "efficient", "new", "2026", "multilingual"].into_iter().map(String::from).collect(),
        },
    ]
}

/// Embedded topics used as fallback when `catalog.json` is missing.
/// Search terms are curated for the Hub; niche topics surface generalists first.
fn curated_topics() -> Vec<Topic> {
    fn t(
        id: &str,
        icon: &str,
        label: &str,
        tier: &str,
        blurb: &str,
        queries: &[&str],
        rec: &[&str],
        note: Option<&str>,
    ) -> Topic {
        Topic {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            tier: tier.into(),
            blurb: blurb.into(),
            hf_queries: queries.iter().map(|s| s.to_string()).collect(),
            hf_lang: String::new(),
            recommended_model_ids: rec.iter().map(|s| s.to_string()).collect(),
            note: note.map(String::from),
        }
    }
    vec![
        // ---- Rich tier: large pool, the chip searches HF directly. ----
        t("code", "💻", "Code", "rich", "Program, debug, and explain code",
            &["coder", "code"], &["qwen3.5-9b"], None),
        t("reasoning", "🧠", "Reasoning", "rich", "Logic, math, and step-by-step work",
            &["reasoning", "thinking"], &["phi-4-mini-reasoning", "qwen3.5-9b"], None),
        t("uncensored", "🔓", "Uncensored", "rich", "Without safety filters (abliterated)",
            &["abliterated", "uncensored"], &[],
            Some("Models without safety filters. Use them thoughtfully and responsibly.")),
        t("agent", "🛠️", "Agent / Tools", "rich", "Tool calling and agentic workflows",
            &["hermes"], &["qwen3.5-9b"], None),
        t("roleplay", "🎭", "Roleplay / Writing", "rich", "Characters, narrative, and creative writing",
            &["roleplay", "rp"], &[],
            Some("Results may include adult/NSFW material.")),
        // ---- Niche tier: thin pool, recommended generalists first. ----
        t("legal", "⚖️", "Legal", "niche", "Law and legal documents",
            &["legal", "law"], &["qwen3.5-9b", "gemma-4-12b"],
            Some("Few legal models are available in GGUF, and they are often country-specific. A strong generalist usually works better.")),
        t("medical", "🩺", "Medical", "niche", "Health and biomedical domain",
            &["medical", "bio-medical"], &["qwen3.5-9b", "gemma-4-12b"],
            Some("Not medical advice. The GGUF pool is small; a strong generalist often works better.")),
        t("finance", "💰", "Finance", "niche", "Accounting and finance",
            &["finance"], &["qwen3.5-9b", "gemma-4-12b"],
            Some("Very few finance models are available in GGUF; consider a strong generalist.")),
    ]
}

/// Resuelve la ruta del `catalog.json` local. Prioriza el override editable del
/// usuario en el dir de config (recarga en caliente para test) y luego el cwd
/// (en dev, `cargo tauri dev` corre desde `src-tauri/`). Devuelve `None` si no
/// hay archivo y se debe usar el catálogo embebido.
fn catalog_json_path() -> Option<PathBuf> {
    if let Some(cfg) = dirs::config_dir() {
        let p = cfg.join("agent-aleph/catalog.json");
        if p.exists() {
            return Some(p);
        }
    }
    let cwd = std::env::current_dir().ok()?.join("catalog.json");
    if cwd.exists() {
        return Some(cwd);
    }
    None
}

/// Lee y parsea el `catalog.json` local. Cualquier fallo (ausente/ inválido)
/// devuelve un `CatalogFile` vacío, que hace caer a los embebidos.
fn load_catalog_file() -> CatalogFile {
    let Some(path) = catalog_json_path() else {
        return CatalogFile::default();
    };
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<CatalogFile>(&s).ok())
    {
        Some(f) => f,
        None => {
            tracing::warn!("catalog.json inválido o ilegible en {:?}; uso embebido", path);
            CatalogFile::default()
        }
    }
}

pub fn list_catalog() -> Vec<CatalogModel> {
    let f = load_catalog_file();
    if f.models.is_empty() {
        curated()
    } else {
        f.models
    }
}

pub fn list_topics() -> Vec<Topic> {
    let f = load_catalog_file();
    if f.topics.is_empty() {
        curated_topics()
    } else {
        f.topics
    }
}

pub async fn search_hf(query: &str) -> AppResult<Vec<HfModel>> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let url = format!(
        "https://huggingface.co/api/models?filter=gguf&search={}&limit=30&sort=downloads&direction=-1",
        urlencoding::encode(query)
    );
    let client = reqwest::Client::builder()
        .user_agent("agent-aleph/0.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let resp = client.get(&url).send().await?;
    let raw: Vec<serde_json::Value> = resp.json().await?;
    Ok(raw.into_iter().filter_map(parse_hf_model).collect())
}

/// Explora TODOS los modelos GGUF del Hub (sin query), ordenados según `sort`.
/// `sort` admite: "downloads" | "likes" | "trendingScore" | "lastModified".
/// `limit` se acota a [1, 200]. Para "paginar" se vuelve a pedir con más límite.
pub async fn browse_hf(sort: &str, limit: u32) -> AppResult<Vec<HfModel>> {
    let sort = match sort {
        "likes" => "likes",
        "trendingScore" | "trending" => "trendingScore",
        "lastModified" | "recent" => "lastModified",
        _ => "downloads",
    };
    let limit = limit.clamp(1, 200);
    let url = format!(
        "https://huggingface.co/api/models?filter=gguf&limit={}&sort={}&direction=-1",
        limit, sort
    );
    let client = reqwest::Client::builder()
        .user_agent("agent-aleph/0.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let resp = client.get(&url).send().await?;
    let raw: Vec<serde_json::Value> = resp.json().await?;
    Ok(raw.into_iter().filter_map(parse_hf_model).collect())
}

/// Convierte un objeto JSON del Hub en `HfModel`.
fn parse_hf_model(v: serde_json::Value) -> Option<HfModel> {
    let id = v.get("id")?.as_str()?.to_string();
    let (author, name) = match id.split_once('/') {
        Some((a, n)) => (a.to_string(), n.to_string()),
        None => ("".into(), id.clone()),
    };
    Some(HfModel {
        repo: id,
        author,
        name,
        downloads: v.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
        likes: v.get("likes").and_then(|d| d.as_u64()).unwrap_or(0),
        tags: v
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default(),
    })
}

pub async fn list_model_files(repo: &str) -> AppResult<Vec<HfFile>> {
    let url = format!("https://huggingface.co/api/models/{}/tree/main", repo);
    let client = reqwest::Client::builder()
        .user_agent("agent-aleph/0.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(AppError::NotFound(format!(
            "No se pudo listar el repo {} (HTTP {})",
            repo,
            resp.status()
        )));
    }
    let raw: Vec<serde_json::Value> = resp.json().await?;
    let out = raw
        .into_iter()
        .filter_map(|v| {
            let path = v.get("path")?.as_str()?.to_string();
            if !path.ends_with(".gguf") {
                return None;
            }
            let size = v
                .get("size")
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            let r#ref = v
                .get("r#ref")
                .and_then(|r| r.as_str())
                .unwrap_or("main")
                .to_string();
            Some(HfFile { r#ref, path, size })
        })
        .collect();
    Ok(out)
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                    c.to_string()
                } else {
                    format!("%{:02X}", c as u32)
                }
            })
            .collect()
    }
}
