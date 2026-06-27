use super::{resolve_in_root, ParamSpec, ParamType, Risk, Tool, ToolCtx, SKIP_DIRS};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct ListDir;

#[async_trait]
impl Tool for ListDir {
    fn name(&self) -> &'static str {
        "list"
    }

    fn description(&self) -> &'static str {
        "List the contents of a project directory. path is relative, default \".\"."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![ParamSpec::opt("path", ParamType::Str)]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let rel = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let dir = resolve_in_root(&ctx.working_dir, rel, true)?;
        if !dir.is_dir() {
            return Err(AppError::Other(format!("not a directory: {rel}")));
        }

        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| AppError::Other(format!("could not list {rel}: {e}")))?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.path().is_dir() {
                let marker = if SKIP_DIRS.contains(&name.as_str()) { "/ (skipped)" } else { "/" };
                dirs.push(format!("{name}{marker}"));
            } else {
                files.push(name);
            }
        }
        dirs.sort();
        files.sort();

        let mut out = String::new();
        for d in &dirs {
            out.push_str(&format!("📁 {d}\n"));
        }
        for f in &files {
            out.push_str(&format!("   {f}\n"));
        }
        if out.is_empty() {
            out = "[empty directory]".into();
        }
        Ok(out)
    }
}
