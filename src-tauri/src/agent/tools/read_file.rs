use super::{resolve_in_root, ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

const DEFAULT_LIMIT: usize = 400;

pub struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read a project text file. offset is the 0-based starting line; limit is the maximum number of lines, default 400."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("path", ParamType::Str),
            ParamSpec::opt("offset", ParamType::Int),
            ParamSpec::opt("limit", ParamType::Int),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("read_file requires the 'path' argument".into()))?;
        let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_LIMIT as u64) as usize;

        let resolved = resolve_in_root(&ctx.working_dir, path, true)?;

        let content = tokio::fs::read_to_string(&resolved)
            .await
            .map_err(|e| AppError::Other(format!("could not read {}: {e}", resolved.display())))?;

        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let end = offset.saturating_add(limit).min(total);

        let mut out = String::new();
        if offset < total {
            for (i, line) in lines[offset..end].iter().enumerate() {
                out.push_str(&format!("{:>6}\t{}\n", offset + i + 1, line));
            }
        }
        if end < total {
            out.push_str(&format!(
                "\n... [{} more lines; use offset={} to continue]\n",
                total - end,
                end
            ));
        }
        if out.is_empty() {
            out = "[empty file or range out of bounds]".into();
        }
        Ok(out)
    }
}
