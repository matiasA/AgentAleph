pub mod bash;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod list;
pub mod read_file;
pub mod write_file;

use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Nivel de riesgo de una herramienta; determina la política de permisos por modo.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    /// Solo lectura (archivos, búsquedas). Siempre permitida.
    Read,
    /// Escribe/modifica archivos del proyecto.
    Write,
    /// Ejecuta comandos arbitrarios.
    Exec,
}

/// Tipo de un parámetro de herramienta (gobierna la gramática y la validación).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Str,
    Int,
    Bool,
}

/// Especificación de un parámetro de los args de una herramienta.
#[derive(Clone)]
pub struct ParamSpec {
    pub name: &'static str,
    pub ty: ParamType,
    pub required: bool,
}

impl ParamSpec {
    pub fn req(name: &'static str, ty: ParamType) -> Self {
        Self { name, ty, required: true }
    }
    pub fn opt(name: &'static str, ty: ParamType) -> Self {
        Self { name, ty, required: false }
    }
}

/// Documentación completa de una herramienta para prompt, gramática y validación.
#[derive(Clone)]
pub struct ToolDoc {
    pub name: &'static str,
    pub description: &'static str,
    pub params: Vec<ParamSpec>,
}

/// Contexto de ejecución compartido por todas las herramientas.
pub struct ToolCtx {
    pub working_dir: PathBuf,
}

/// Una herramienta invocable por el agente.
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn risk(&self) -> Risk;
    /// Esquema de argumentos: tipos y obligatoriedad. Vacío = sin argumentos.
    fn params(&self) -> Vec<ParamSpec>;
    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String>;
}

/// Registro de herramientas disponibles para un turno de agente.
pub struct Registry {
    tools: Vec<Box<dyn Tool>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Box::new(read_file::ReadFile),
                Box::new(list::ListDir),
                Box::new(glob::Glob),
                Box::new(grep::Grep),
                Box::new(write_file::WriteFile),
                Box::new(edit::Edit),
                Box::new(bash::Bash),
            ],
        }
    }

    /// Documentación de todas las herramientas (para prompt, gramática y validación).
    pub fn docs(&self) -> Vec<ToolDoc> {
        self.tools
            .iter()
            .map(|t| ToolDoc {
                name: t.name(),
                description: t.description(),
                params: t.params(),
            })
            .collect()
    }

    pub fn risk(&self, name: &str) -> Option<Risk> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.risk())
    }

    /// Valida los args contra el esquema de la herramienta antes de ejecutar.
    /// Devuelve un error legible para reinyectar al modelo si algo no cuadra.
    pub fn validate(&self, name: &str, args: &Value) -> Result<(), String> {
        let tool = match self.tools.iter().find(|t| t.name() == name) {
            Some(t) => t,
            None => return Ok(()), // desconocida: execute() devolverá el error
        };
        let obj = args
            .as_object()
            .ok_or_else(|| "args debe ser un objeto JSON".to_string())?;
        for p in tool.params() {
            match obj.get(p.name) {
                None if p.required => {
                    return Err(format!("falta el parámetro requerido '{}'", p.name))
                }
                None => {}
                Some(v) => {
                    let ok = match p.ty {
                        ParamType::Str => v.is_string(),
                        ParamType::Int => v.is_i64() || v.is_u64(),
                        ParamType::Bool => v.is_boolean(),
                    };
                    if !ok {
                        return Err(format!(
                            "el parámetro '{}' tiene un tipo inválido",
                            p.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn execute(&self, name: &str, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        match self.tools.iter().find(|t| t.name() == name) {
            Some(t) => t.execute(args, ctx).await,
            None => Err(AppError::NotFound(format!("herramienta desconocida: {name}"))),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------- Utilidades compartidas ----------

/// Resuelve `rel` dentro de `root`. Si `must_exist` es true, el destino debe existir;
/// si es false (para escritura), basta con que exista el directorio padre. En ambos casos
/// se rechaza cualquier ruta que escape del directorio del proyecto.
pub fn resolve_in_root(root: &Path, rel: &str, must_exist: bool) -> AppResult<PathBuf> {
    let canon_root = std::fs::canonicalize(root)
        .map_err(|e| AppError::Other(format!("directorio del proyecto inválido: {e}")))?;
    let joined = root.join(rel);

    if must_exist {
        return match std::fs::canonicalize(&joined) {
            Ok(c) if c.starts_with(&canon_root) => Ok(c),
            Ok(_) => Err(deny_escape()),
            Err(_) => Err(AppError::NotFound(format!("ruta no encontrada: {rel}"))),
        };
    }

    // Para escritura: canonicalizamos el padre (debe existir) y reanexamos el nombre.
    let parent = joined
        .parent()
        .ok_or_else(|| AppError::Other("ruta inválida".into()))?;
    let name = joined
        .file_name()
        .ok_or_else(|| AppError::Other("ruta sin nombre de archivo".into()))?;
    let canon_parent = std::fs::canonicalize(parent).map_err(|_| {
        AppError::NotFound(format!("el directorio destino no existe para: {rel}"))
    })?;
    if !canon_parent.starts_with(&canon_root) {
        return Err(deny_escape());
    }
    Ok(canon_parent.join(name))
}

fn deny_escape() -> AppError {
    AppError::Other("acceso denegado: la ruta queda fuera del directorio del proyecto".into())
}

/// Directorios que se omiten al recorrer el árbol (ruido / volumen).
const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target", "dist", ".svelte-kit"];

/// Recorre `dir` recursivamente (hasta `max_depth` niveles) y devuelve rutas de archivos
/// relativas a `root`. Omite directorios pesados habituales.
pub fn walk_files(root: &Path, dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    if out.len() >= 5000 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') && name != "." {
            continue;
        }
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_ref()) || max_depth == 0 {
                continue;
            }
            walk_files(root, &path, max_depth - 1, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_path_buf());
        }
    }
}
