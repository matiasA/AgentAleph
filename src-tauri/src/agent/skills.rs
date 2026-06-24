//! Skills: paquetes de instrucciones + recursos que dan conocimiento especializado al agente.
//!
//! Cada skill es una carpeta `~/.local/share/agent-aleph/skills/<slug>/` con un `SKILL.md`
//! (frontmatter `name`/`description` + cuerpo de instrucciones) y recursos opcionales. Las
//! skills *activas* (listadas en `skills/enabled.json`) inyectan su cuerpo en el system prompt
//! del agente. Es 100% local: no hay red.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Tope de caracteres del cuerpo de una skill que se inyecta al prompt (contexto local chico).
const SKILL_BODY_MAX: usize = 6000;

/// Metadatos de una skill para la UI.
#[derive(Serialize, Clone)]
pub struct Skill {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Skill resuelta para inyección en el prompt (nombre + cuerpo).
pub struct SkillDoc {
    pub name: String,
    pub body: String,
}

pub fn skills_dir() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("agent-aleph");
    p.push("skills");
    p
}

fn enabled_path() -> PathBuf {
    skills_dir().join("enabled.json")
}

fn skill_md(slug: &str) -> PathBuf {
    skills_dir().join(slug).join("SKILL.md")
}

fn load_enabled() -> HashSet<String> {
    std::fs::read_to_string(enabled_path())
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .map(|v| v.into_iter().collect())
        .unwrap_or_default()
}

fn save_enabled(set: &HashSet<String>) -> AppResult<()> {
    std::fs::create_dir_all(skills_dir())?;
    let list: Vec<&String> = set.iter().collect();
    std::fs::write(enabled_path(), serde_json::to_string_pretty(&list)?)?;
    Ok(())
}

/// Parsea un `SKILL.md`: frontmatter `---\nname: …\ndescription: …\n---` + cuerpo.
/// Tolerante: si no hay frontmatter, `name` cae al slug y `description` queda vacía.
fn parse_front(content: &str, slug: &str) -> (String, String, String) {
    let mut name = slug.to_string();
    let mut description = String::new();

    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let front = &rest[..end];
            let body_start = end + "\n---".len();
            let body = rest[body_start..].trim_start_matches('\n').to_string();
            for line in front.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    match k.trim().to_lowercase().as_str() {
                        "name" => name = v.trim().to_string(),
                        "description" => description = v.trim().to_string(),
                        _ => {}
                    }
                }
            }
            return (name, description, body);
        }
    }
    (name, description, content.trim().to_string())
}

/// Convierte un nombre en un slug de carpeta seguro y único dentro del directorio de skills.
fn slugify_unique(name: &str) -> String {
    let base: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-').replace("--", "-");
    let base = if base.is_empty() { "skill".to_string() } else { base };
    let mut slug = base.clone();
    let mut n = 2;
    while skills_dir().join(&slug).exists() {
        slug = format!("{base}-{n}");
        n += 1;
    }
    slug
}

/// Lista todas las skills disponibles con su estado de activación.
pub fn list() -> Vec<Skill> {
    let enabled = load_enabled();
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(skills_dir()) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let md = path.join("SKILL.md");
        let Ok(content) = std::fs::read_to_string(&md) else {
            continue;
        };
        let (name, description, _) = parse_front(&content, &slug);
        out.push(Skill {
            slug: slug.clone(),
            name,
            description,
            enabled: enabled.contains(&slug),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Activa o desactiva una skill por slug.
pub fn set_enabled(slug: &str, enabled: bool) -> AppResult<()> {
    if !skill_md(slug).exists() {
        return Err(AppError::NotFound(format!("skill desconocida: {slug}")));
    }
    let mut set = load_enabled();
    if enabled {
        set.insert(slug.to_string());
    } else {
        set.remove(slug);
    }
    save_enabled(&set)
}

/// Crea una skill nueva desde la UI.
pub fn create(name: &str, description: &str, body: &str) -> AppResult<Skill> {
    if name.trim().is_empty() {
        return Err(AppError::Other("el nombre de la skill no puede estar vacío".into()));
    }
    let slug = slugify_unique(name);
    let dir = skills_dir().join(&slug);
    std::fs::create_dir_all(&dir)?;
    let content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        name.trim(),
        description.trim(),
        body.trim()
    );
    std::fs::write(dir.join("SKILL.md"), content)?;
    Ok(Skill {
        slug,
        name: name.trim().to_string(),
        description: description.trim().to_string(),
        enabled: false,
    })
}

/// Importa una skill copiando una carpeta que contenga `SKILL.md`.
pub fn import(folder: &str) -> AppResult<Skill> {
    let src = PathBuf::from(folder);
    if !src.join("SKILL.md").exists() {
        return Err(AppError::Other("la carpeta no contiene un SKILL.md".into()));
    }
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "skill".into());
    let slug = slugify_unique(&name);
    let dst = skills_dir().join(&slug);
    copy_dir(&src, &dst)?;
    let content = std::fs::read_to_string(dst.join("SKILL.md")).unwrap_or_default();
    let (name, description, _) = parse_front(&content, &slug);
    Ok(Skill {
        slug,
        name,
        description,
        enabled: false,
    })
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Borra una skill (carpeta) y la quita de las activas.
pub fn delete(slug: &str) -> AppResult<()> {
    let dir = skills_dir().join(slug);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    let mut set = load_enabled();
    if set.remove(slug) {
        let _ = save_enabled(&set);
    }
    Ok(())
}

/// Devuelve el contenido crudo del `SKILL.md` (para ver/editar en la UI).
pub fn read_full(slug: &str) -> AppResult<String> {
    std::fs::read_to_string(skill_md(slug))
        .map_err(|_| AppError::NotFound(format!("skill desconocida: {slug}")))
}

/// Resuelve las skills activas a (nombre, cuerpo) para inyectar en el system prompt.
pub fn enabled_docs() -> Vec<SkillDoc> {
    let enabled = load_enabled();
    let mut docs = Vec::new();
    for slug in &enabled {
        let Ok(content) = std::fs::read_to_string(skill_md(slug)) else {
            continue;
        };
        let (name, _, body) = parse_front(&content, slug);
        if body.trim().is_empty() {
            continue;
        }
        let body: String = body.chars().take(SKILL_BODY_MAX).collect();
        docs.push(SkillDoc { name, body });
    }
    docs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    docs
}
