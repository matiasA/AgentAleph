use super::{resolve_in_root, ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn description(&self) -> &'static str {
        "Crea o sobrescribe un archivo del proyecto."
    }

    fn risk(&self) -> Risk {
        Risk::Write
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("path", ParamType::Str),
            ParamSpec::req("content", ParamType::Str),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("write_file requiere 'path'".into()))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("write_file requiere 'content'".into()))?;

        let resolved = resolve_in_root(&ctx.working_dir, path, false)?;
        let existed = resolved.exists();
        tokio::fs::write(&resolved, content)
            .await
            .map_err(|e| AppError::Other(format!("no se pudo escribir {path}: {e}")))?;

        let bytes = content.len();
        let verb = if existed { "sobrescrito" } else { "creado" };
        Ok(format!("Archivo {verb}: {path} ({bytes} bytes)"))
    }
}
