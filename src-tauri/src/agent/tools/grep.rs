use super::{walk_files, ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

const MAX_HITS: usize = 100;

pub struct Grep;

#[async_trait]
impl Tool for Grep {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for literal text inside project files."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("query", ParamType::Str),
            ParamSpec::opt("ignore_case", ParamType::Bool),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("grep requires the 'query' argument".into()))?;
        let ignore_case = args.get("ignore_case").and_then(|v| v.as_bool()).unwrap_or(false);
        let needle = if ignore_case { query.to_lowercase() } else { query.to_string() };

        let mut files = Vec::new();
        walk_files(&ctx.working_dir, &ctx.working_dir, 12, &mut files);

        let mut out = String::new();
        let mut hits = 0usize;
        'outer: for rel in &files {
            let abs = ctx.working_dir.join(rel);
            let content = match std::fs::read_to_string(&abs) {
                Ok(c) => c,
                Err(_) => continue, // binary or unreadable
            };
            for (i, line) in content.lines().enumerate() {
                let hay = if ignore_case { line.to_lowercase() } else { line.to_string() };
                if hay.contains(&needle) {
                    let trimmed: String = line.trim().chars().take(200).collect();
                    out.push_str(&format!(
                        "{}:{}: {}\n",
                        rel.to_string_lossy().replace('\\', "/"),
                        i + 1,
                        trimmed
                    ));
                    hits += 1;
                    if hits >= MAX_HITS {
                        out.push_str(&format!("\n... [limit of {MAX_HITS} matches]\n"));
                        break 'outer;
                    }
                }
            }
        }
        if out.is_empty() {
            out = format!("[no matches for '{query}']");
        }
        Ok(out)
    }
}
