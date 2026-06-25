use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

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
    pub category: String,
    pub tags: Vec<String>,
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
        // ==================== Ultra-ligeros (CPU, cualquier hardware) ====================
        CatalogModel {
            id: "qwen3.5-0.8b".into(),
            name: "Qwen 3.5 0.8B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-0.8B-GGUF".into(),
            description: "El más pequeño de la línea Qwen 3.5. Cabe en cualquier máquina. Ideal para pruebas rápidas.".into(),
            params: "0.8B".into(),
            default_file: "Qwen3.5-0.8B-Q4_K_M.gguf".into(),
            size_gb: 0.53,
            category: "Ultra-ligero".into(),
            tags: vec!["qwen", "alibaba", "ultraligero", "multilingue", "cpu"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-2b".into(),
            name: "Qwen 3.5 2B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-2B-GGUF".into(),
            description: "Qwen 3.5 de 2B. Excelente multilingüe, rápido en CPU. Buena calidad para su tamaño.".into(),
            params: "2B".into(),
            default_file: "Qwen3.5-2B-Q4_K_M.gguf".into(),
            size_gb: 1.28,
            category: "Ultra-ligero".into(),
            tags: vec!["qwen", "alibaba", "ultraligero", "multilingue", "cpu"].into_iter().map(String::from).collect(),
        },

        // ==================== Ligeros (CPU básico, 8GB RAM) ====================
        CatalogModel {
            id: "phi-4-mini".into(),
            name: "Phi-4 Mini Instruct".into(),
            author: "unsloth".into(),
            repo: "unsloth/Phi-4-mini-instruct-GGUF".into(),
            description: "Microsoft Phi-4 Mini. Razonamiento superior en tamaño compacto. Excelente para código y lógica.".into(),
            params: "3.8B".into(),
            default_file: "Phi-4-mini-instruct-Q4_K_M.gguf".into(),
            size_gb: 2.49,
            category: "Ligero".into(),
            tags: vec!["microsoft", "phi", "ligero", "razonamiento", "codigo"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "phi-4-mini-reasoning".into(),
            name: "Phi-4 Mini Reasoning".into(),
            author: "unsloth".into(),
            repo: "unsloth/Phi-4-mini-reasoning-GGUF".into(),
            description: "Variante de Phi-4 Mini optimizada para razonamiento paso a paso (chain-of-thought).".into(),
            params: "3.8B".into(),
            default_file: "Phi-4-mini-reasoning-Q4_K_M.gguf".into(),
            size_gb: 2.49,
            category: "Ligero".into(),
            tags: vec!["microsoft", "phi", "ligero", "razonamiento", "thinking"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-4b".into(),
            name: "Qwen 3.5 4B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-4B-GGUF".into(),
            description: "Qwen 3.5 de 4B. Equilibrio ideal entre calidad y velocidad en CPU. Soporta español nativo.".into(),
            params: "4B".into(),
            default_file: "Qwen3.5-4B-Q4_K_M.gguf".into(),
            size_gb: 2.74,
            category: "Ligero".into(),
            tags: vec!["qwen", "alibaba", "ligero", "multilingue", "cpu"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-e2b".into(),
            name: "Gemma 4 E2B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-E2B-it-GGUF".into(),
            description: "Google Gemma 4 E2B. Eficiente, calidad alta en tamaño reducido. Buena para chat general.".into(),
            params: "2B".into(),
            default_file: "gemma-4-E2B-it-Q4_K_M.gguf".into(),
            size_gb: 3.11,
            category: "Ligero".into(),
            tags: vec!["google", "gemma", "ligero", "eficiente"].into_iter().map(String::from).collect(),
        },

        // ==================== Medianos (CPU 16GB / GPU 6GB+) ====================
        CatalogModel {
            id: "gemma-4-e4b".into(),
            name: "Gemma 4 E4B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-E4B-it-GGUF".into(),
            description: "Google Gemma 4 E4B. Modelo mediano con excelente relación calidad/recurso.".into(),
            params: "4B".into(),
            default_file: "gemma-4-E4B-it-Q4_K_M.gguf".into(),
            size_gb: 4.98,
            category: "Mediano".into(),
            tags: vec!["google", "gemma", "mediano"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-9b".into(),
            name: "Qwen 3.5 9B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-9B-GGUF".into(),
            description: "Qwen 3.5 9B. El más popular de la línea. Razonamiento sólido, excelente multilingüe. GPU recomendada.".into(),
            params: "9B".into(),
            default_file: "Qwen3.5-9B-Q4_K_M.gguf".into(),
            size_gb: 5.68,
            category: "Mediano".into(),
            tags: vec!["qwen", "alibaba", "mediano", "multilingue", "popular"].into_iter().map(String::from).collect(),
        },

        // ==================== Pesados (GPU 8-12GB) ====================
        CatalogModel {
            id: "gemma-4-12b".into(),
            name: "Gemma 4 12B IT".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-12b-it-GGUF".into(),
            description: "Google Gemma 4 12B. Modelo potente de Google con razonamiento avanzado. GPU 8GB+.".into(),
            params: "12B".into(),
            default_file: "gemma-4-12b-it-Q4_K_M.gguf".into(),
            size_gb: 7.12,
            category: "Pesado".into(),
            tags: vec!["google", "gemma", "pesado", "potente"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-12b-qat".into(),
            name: "Gemma 4 12B IT (QAT)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-12B-it-qat-GGUF".into(),
            description: "Variante QAT (entrenada para cuantización) de Gemma 4 12B. Más liviana y de mayor calidad que el Q4_K_M estándar al mismo nivel de bits.".into(),
            params: "12B".into(),
            default_file: "gemma-4-12B-it-qat-UD-Q4_K_XL.gguf".into(),
            size_gb: 6.72,
            category: "Pesado".into(),
            tags: vec!["google", "gemma", "pesado", "qat", "nuevo"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "phi-4-14b".into(),
            name: "Phi-4 (14B)".into(),
            author: "MaziyarPanahi".into(),
            repo: "MaziyarPanahi/phi-4-GGUF".into(),
            description: "Microsoft Phi-4 14B. Razonamiento de nivel superior, competitivo con modelos más grandes.".into(),
            params: "14B".into(),
            default_file: "phi-4.Q4_K_M.gguf".into(),
            size_gb: 9.05,
            category: "Pesado".into(),
            tags: vec!["microsoft", "phi", "pesado", "razonamiento", "codigo"].into_iter().map(String::from).collect(),
        },

        // ==================== MoE / Muy pesados (GPU 16-24GB+) ====================
        CatalogModel {
            id: "qwen3.5-27b".into(),
            name: "Qwen 3.5 27B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-27B-GGUF".into(),
            description: "Qwen 3.5 27B. Razonamiento profundo, calidad cercana a modelos comerciales. GPU 16GB+.".into(),
            params: "27B".into(),
            default_file: "Qwen3.5-27B-Q4_K_M.gguf".into(),
            size_gb: 16.74,
            category: "MoE / Pesado+".into(),
            tags: vec!["qwen", "alibaba", "moe", "pesado", "potente", "multilingue"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.6-27b".into(),
            name: "Qwen 3.6 27B".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.6-27B-GGUF".into(),
            description: "Qwen 3.6 27B. Última generación de Alibaba. Mejor razonamiento y contexto que 3.5.".into(),
            params: "27B".into(),
            default_file: "Qwen3.6-27B-Q4_K_M.gguf".into(),
            size_gb: 16.82,
            category: "MoE / Pesado+".into(),
            tags: vec!["qwen", "alibaba", "moe", "pesado", "nuevo", "2026", "multilingue"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-26b-a4b".into(),
            name: "Gemma 4 26B A4B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-26B-A4B-it-GGUF".into(),
            description: "Google Gemma 4 26B MoE: 26B totales pero solo 4B activos. Velocidad de modelo pequeño con calidad de grande.".into(),
            params: "26B (4B activo)".into(),
            default_file: "gemma-4-26B-A4B-it-UD-Q4_K_M.gguf".into(),
            size_gb: 16.95,
            category: "MoE / Pesado+".into(),
            tags: vec!["google", "gemma", "moe", "pesado", "eficiente", "nuevo", "2026"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "gemma-4-26b-a4b-qat".into(),
            name: "Gemma 4 26B A4B (QAT MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/gemma-4-26B-A4B-it-qat-GGUF".into(),
            description: "Variante QAT del MoE Gemma 4 26B A4B. Misma arquitectura (4B activos) con mejor calidad por bit y menos peso que la versión estándar.".into(),
            params: "26B (4B activo)".into(),
            default_file: "gemma-4-26B-A4B-it-qat-UD-Q4_K_XL.gguf".into(),
            size_gb: 14.25,
            category: "MoE / Pesado+".into(),
            tags: vec!["google", "gemma", "moe", "pesado", "eficiente", "qat", "nuevo"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3-30b-a3b".into(),
            name: "Qwen3 30B A3B Instruct 2507".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3-30B-A3B-Instruct-2507-GGUF".into(),
            description: "Qwen3 MoE: 30B totales, 3B activos. Rápido como un 3B, potente como un 30B. Versión 2507.".into(),
            params: "30B (3B activo)".into(),
            default_file: "Qwen3-30B-A3B-Instruct-2507-Q4_K_M.gguf".into(),
            size_gb: 18.56,
            category: "MoE / Pesado+".into(),
            tags: vec!["qwen", "alibaba", "moe", "pesado", "eficiente", "multilingue"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.5-35b-a3b".into(),
            name: "Qwen 3.5 35B A3B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.5-35B-A3B-GGUF".into(),
            description: "Qwen 3.5 MoE 35B: 35B totales, 3B activos. La generación actual de Qwen MoE. GPU 16GB+.".into(),
            params: "35B (3B activo)".into(),
            default_file: "Qwen3.5-35B-A3B-Q4_K_M.gguf".into(),
            size_gb: 22.02,
            category: "MoE / Pesado+".into(),
            tags: vec!["qwen", "alibaba", "moe", "pesado", "eficiente", "nuevo", "multilingue"].into_iter().map(String::from).collect(),
        },
        CatalogModel {
            id: "qwen3.6-35b-a3b".into(),
            name: "Qwen 3.6 35B A3B (MoE)".into(),
            author: "unsloth".into(),
            repo: "unsloth/Qwen3.6-35B-A3B-GGUF".into(),
            description: "Qwen 3.6 MoE 35B: última generación. 35B totales, 3B activos. Lo más moderno de Alibaba a junio 2026.".into(),
            params: "35B (3B activo)".into(),
            default_file: "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf".into(),
            size_gb: 22.13,
            category: "MoE / Pesado+".into(),
            tags: vec!["qwen", "alibaba", "moe", "pesado", "eficiente", "nuevo", "2026", "multilingue"].into_iter().map(String::from).collect(),
        },
    ]
}

pub fn list_catalog() -> Vec<CatalogModel> {
    curated()
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
