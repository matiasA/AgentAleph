use super::{walk_files, ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct Glob;

#[async_trait]
impl Tool for Glob {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files by name with * and ? wildcards, for example \"*.rs\" or \"src/*.svelte\"."
    }

    fn risk(&self) -> Risk {
        Risk::Read
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![ParamSpec::req("pattern", ParamType::Str)]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("glob requires the 'pattern' argument".into()))?;

        let mut files = Vec::new();
        walk_files(&ctx.working_dir, &ctx.working_dir, 12, &mut files);

        let mut matches: Vec<String> = files
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .filter(|p| glob_match(pattern, p))
            .collect();
        matches.sort();
        matches.truncate(200);

        if matches.is_empty() {
            return Ok(format!("[no matches for '{pattern}']"));
        }
        Ok(matches.join("\n"))
    }
}

/// Glob matching with `*` and `?`.
fn glob_match(pattern: &str, text: &str) -> bool {
    // If the pattern has no separator, compare against the filename only.
    let target = if pattern.contains('/') {
        text
    } else {
        text.rsplit('/').next().unwrap_or(text)
    };
    wildcard(pattern.as_bytes(), target.as_bytes())
}

fn wildcard(pat: &[u8], txt: &[u8]) -> bool {
    let (mut p, mut t) = (0usize, 0usize);
    let (mut star_p, mut star_t): (Option<usize>, usize) = (None, 0);
    while t < txt.len() {
        if p < pat.len() && (pat[p] == b'?' || pat[p] == txt[t]) {
            p += 1;
            t += 1;
        } else if p < pat.len() && pat[p] == b'*' {
            star_p = Some(p);
            star_t = t;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_t += 1;
            t = star_t;
        } else {
            return false;
        }
    }
    while p < pat.len() && pat[p] == b'*' {
        p += 1;
    }
    p == pat.len()
}
