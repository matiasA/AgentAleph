use super::{resolve_in_root, ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct Edit;

#[async_trait]
impl Tool for Edit {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Reemplaza una porción exacta de texto en un archivo; 'old_string' debe aparecer EXACTAMENTE UNA VEZ."
    }

    fn risk(&self) -> Risk {
        Risk::Write
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("path", ParamType::Str),
            ParamSpec::req("old_string", ParamType::Str),
            ParamSpec::req("new_string", ParamType::Str),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("edit requiere 'path'".into()))?;
        let old = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("edit requiere 'old_string'".into()))?;
        let new = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("edit requiere 'new_string'".into()))?;

        if old.is_empty() {
            return Err(AppError::Other("'old_string' no puede estar vacío".into()));
        }

        let resolved = resolve_in_root(&ctx.working_dir, path, true)?;
        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| AppError::Other(format!("no se pudo leer {path}: {e}")))?;

        let count = content.matches(old).count();
        if count == 0 {
            return Err(AppError::Other(
                "'old_string' no se encontró en el archivo".into(),
            ));
        }
        if count > 1 {
            return Err(AppError::Other(format!(
                "'old_string' aparece {count} veces; debe ser único (añade más contexto)"
            )));
        }

        let updated = content.replacen(old, new, 1);
        tokio::fs::write(&resolved, &updated)
            .await
            .map_err(|e| AppError::Other(format!("no se pudo escribir {path}: {e}")))?;

        Ok(format!("Editado {path}: 1 reemplazo aplicado"))
    }
}
