use super::{ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::agent::memory::{self, Scope};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct Memory;

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn description(&self) -> &'static str {
        "Record durable facts across sessions. scope=\"project\" for facts about this \
         codebase (conventions, decisions, gotchas) — stored in the repo, visible to \
         teammates. scope=\"user\" for the user's personal preferences (communication \
         style, tooling habits) — applies to all projects. action=\"add\" appends a new \
         single-line entry (content). action=\"replace\" finds an existing entry \
         containing 'match' and replaces it with 'content'. action=\"remove\" deletes \
         the entry containing 'match'. Each file has a strict character budget; \
         consolidate with replace/remove instead of accumulating redundant entries. \
         Never store secrets, credentials, API keys, or tokens in memory — project \
         memory (MEMORY.md) may be committed to git and shared with a team."
    }

    fn risk(&self) -> Risk {
        Risk::Write
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::req("scope", ParamType::Str),
            ParamSpec::req("action", ParamType::Str),
            ParamSpec::opt("content", ParamType::Str),
            ParamSpec::opt("match", ParamType::Str),
        ]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        if !ctx.memory_enabled {
            return Err(AppError::Other("memory is disabled in Settings".into()));
        }

        let scope_str = args
            .get("scope")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("memory requires 'scope'".into()))?;
        let scope = match scope_str {
            "project" => Scope::Project,
            "user" => Scope::User,
            other => {
                return Err(AppError::Other(format!(
                    "invalid 'scope': '{other}' (expected \"project\" or \"user\")"
                )))
            }
        };
        let budget = match scope {
            Scope::Project => ctx.memory_project_budget,
            Scope::User => ctx.memory_user_budget,
        };
        let file_name = match scope {
            Scope::Project => "MEMORY.md",
            Scope::User => "USER.md",
        };

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("memory requires 'action'".into()))?;

        let updated = match action {
            "add" => {
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Other("action=\"add\" requires 'content'".into()))?;
                memory::add_entry(scope, &ctx.working_dir, budget, content)?
            }
            "replace" => {
                let m = args
                    .get("match")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Other("action=\"replace\" requires 'match'".into()))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AppError::Other("action=\"replace\" requires 'content'".into())
                    })?;
                memory::replace_entry(scope, &ctx.working_dir, budget, m, content)?
            }
            "remove" => {
                let m = args
                    .get("match")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Other("action=\"remove\" requires 'match'".into()))?;
                memory::remove_entry(scope, &ctx.working_dir, m)?
            }
            other => {
                return Err(AppError::Other(format!(
                    "invalid 'action': '{other}' (expected \"add\", \"replace\", or \"remove\")"
                )))
            }
        };

        let len = updated.chars().count();
        Ok(format!(
            "Memory updated ({scope_str}). {file_name} is now {len}/{budget} chars:\n{updated}"
        ))
    }
}
