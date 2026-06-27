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
        "Replace an exact text span in a file. 'old_string' must appear EXACTLY ONCE."
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
            .ok_or_else(|| AppError::Other("edit requires 'path'".into()))?;
        let old = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("edit requires 'old_string'".into()))?;
        let new = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("edit requires 'new_string'".into()))?;

        if old.is_empty() {
            return Err(AppError::Other("'old_string' cannot be empty".into()));
        }

        let resolved = resolve_in_root(&ctx.working_dir, path, true)?;
        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| AppError::Other(format!("could not read {path}: {e}")))?;

        let count = content.matches(old).count();
        if count == 0 {
            return Err(AppError::Other(
                "'old_string' was not found in the file".into(),
            ));
        }
        if count > 1 {
            return Err(AppError::Other(format!(
                "'old_string' appears {count} times; it must be unique (add more context)"
            )));
        }

        let updated = content.replacen(old, new, 1);
        tokio::fs::write(&resolved, &updated)
            .await
            .map_err(|e| AppError::Other(format!("could not write {path}: {e}")))?;

        Ok(format!("Edited {path}: 1 replacement applied"))
    }
}
