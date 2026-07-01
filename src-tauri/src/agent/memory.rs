//! Persistent cross-session memory: `MEMORY.md` (project-scoped, lives in the project's
//! working directory) and `USER.md` (global, lives in the app-data directory). Both are
//! line-based (one entry per line, `- ...`), bounded by a hard character budget, and
//! injected into the system prompt at session start by `prompt.rs`. Writes go through the
//! `memory` tool (`agent/tools/memory.rs`); manual edits go through the Tauri commands used
//! by the Memory panel in the UI. 100% local: no network.

use crate::agent::tools::resolve_in_root;
use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

/// Default character budget for project memory (`MEMORY.md`).
pub const PROJECT_BUDGET_DEFAULT: usize = 2200;
/// Default character budget for user memory (`USER.md`).
pub const USER_BUDGET_DEFAULT: usize = 1375;

const PROJECT_FILE_NAME: &str = "MEMORY.md";
const USER_FILE_NAME: &str = "USER.md";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Project,
    User,
}

impl Scope {
    fn file_name(self) -> &'static str {
        match self {
            Scope::Project => PROJECT_FILE_NAME,
            Scope::User => USER_FILE_NAME,
        }
    }
}

fn project_path(working_dir: &Path) -> PathBuf {
    working_dir.join(".agent-aleph").join(PROJECT_FILE_NAME)
}

fn user_dir() -> PathBuf {
    let mut p = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("agent-aleph");
    p.push("memory");
    p
}

fn user_path() -> PathBuf {
    user_dir().join(USER_FILE_NAME)
}

fn read_raw(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Raw content of the project memory file. Never fails; empty string if missing.
pub fn read_project(working_dir: &Path) -> String {
    read_raw(&project_path(working_dir))
}

/// Raw content of the global user memory file. Never fails; empty string if missing.
pub fn read_user() -> String {
    read_raw(&user_path())
}

/// Overwrite the project memory file, creating `.agent-aleph/` if needed. Used both for
/// manual UI edits and internally after add/replace/remove. Resolves through
/// `resolve_in_root` so a write can never escape the project directory.
pub fn write_project_raw(working_dir: &Path, content: &str) -> AppResult<()> {
    std::fs::create_dir_all(working_dir.join(".agent-aleph"))?;
    let resolved = resolve_in_root(working_dir, ".agent-aleph/MEMORY.md", false)?;
    std::fs::write(resolved, content)?;
    Ok(())
}

/// Overwrite the global user memory file, creating its directory if needed.
pub fn write_user_raw(content: &str) -> AppResult<()> {
    std::fs::create_dir_all(user_dir())?;
    std::fs::write(user_path(), content)?;
    Ok(())
}

pub fn clear_project(working_dir: &Path) -> AppResult<()> {
    write_project_raw(working_dir, "")
}

pub fn clear_user() -> AppResult<()> {
    write_user_raw("")
}

fn read_for(scope: Scope, working_dir: &Path) -> String {
    match scope {
        Scope::Project => read_project(working_dir),
        Scope::User => read_user(),
    }
}

fn write_for(scope: Scope, working_dir: &Path, content: &str) -> AppResult<()> {
    match scope {
        Scope::Project => write_project_raw(working_dir, content),
        Scope::User => write_user_raw(content),
    }
}

/// Splits file content into non-empty, trimmed lines (one memory entry per line).
fn split_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

/// Validates and formats a single entry: no embedded newlines, non-empty, `- ` prefix.
fn normalize_entry(raw: &str) -> AppResult<String> {
    if raw.contains('\n') {
        return Err(AppError::Other(
            "'content' must be a single-line entry; call memory again for each additional fact."
                .into(),
        ));
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Other("'content' cannot be empty".into()));
    }
    if let Some(rest) = trimmed.strip_prefix("- ") {
        Ok(format!("- {rest}"))
    } else {
        Ok(format!("- {trimmed}"))
    }
}

/// Line indices whose content contains `needle` as a substring.
fn find_matches(lines: &[String], needle: &str) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(needle))
        .map(|(i, _)| i)
        .collect()
}

fn budget_error(scope: Scope, new_len: usize, budget: usize) -> AppError {
    AppError::Other(format!(
        "memory budget exceeded: {} would be {new_len}/{budget} chars. Consolidate existing \
         entries first with action=\"replace\" or action=\"remove\", then retry.",
        scope.file_name()
    ))
}

fn no_match_error(m: &str) -> AppError {
    AppError::Other(format!(
        "no memory entry contains '{m}'. Check the current content in the system prompt and \
         use an exact substring from an existing line."
    ))
}

fn ambiguous_match_error(m: &str, lines: &[String], indices: &[usize]) -> AppError {
    let list = indices
        .iter()
        .map(|&i| lines[i].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    AppError::Other(format!(
        "'{m}' matches {} entries; be more specific. Matching lines:\n{list}",
        indices.len()
    ))
}

/// Appends a new entry. Exact-duplicate lines are a no-op (idempotent). Errors if the
/// resulting file would exceed `budget` chars — callers must consolidate first via
/// `replace_entry`/`remove_entry` rather than relying on silent truncation.
pub fn add_entry(scope: Scope, working_dir: &Path, budget: usize, entry: &str) -> AppResult<String> {
    let normalized = normalize_entry(entry)?;
    let current = read_for(scope, working_dir);
    let mut lines = split_lines(&current);
    if lines.iter().any(|l| l == &normalized) {
        return Ok(join_lines(&lines));
    }
    lines.push(normalized);
    let updated = join_lines(&lines);
    if updated.chars().count() > budget {
        return Err(budget_error(scope, updated.chars().count(), budget));
    }
    write_for(scope, working_dir, &updated)?;
    Ok(updated)
}

/// Replaces the single line containing `m` with `new_content`. Errors if `m` matches zero
/// or more than one line, or if the result would exceed `budget`.
pub fn replace_entry(
    scope: Scope,
    working_dir: &Path,
    budget: usize,
    m: &str,
    new_content: &str,
) -> AppResult<String> {
    let normalized = normalize_entry(new_content)?;
    let current = read_for(scope, working_dir);
    let mut lines = split_lines(&current);
    let matches = find_matches(&lines, m);
    match matches.len() {
        0 => return Err(no_match_error(m)),
        1 => {}
        _ => return Err(ambiguous_match_error(m, &lines, &matches)),
    }
    lines[matches[0]] = normalized;
    let updated = join_lines(&lines);
    if updated.chars().count() > budget {
        return Err(budget_error(scope, updated.chars().count(), budget));
    }
    write_for(scope, working_dir, &updated)?;
    Ok(updated)
}

/// Removes the single line containing `m`. Errors if `m` matches zero or more than one line.
pub fn remove_entry(scope: Scope, working_dir: &Path, m: &str) -> AppResult<String> {
    let current = read_for(scope, working_dir);
    let mut lines = split_lines(&current);
    let matches = find_matches(&lines, m);
    match matches.len() {
        0 => return Err(no_match_error(m)),
        1 => {}
        _ => return Err(ambiguous_match_error(m, &lines, &matches)),
    }
    lines.remove(matches[0]);
    let updated = join_lines(&lines);
    write_for(scope, working_dir, &updated)?;
    Ok(updated)
}

/// Clips content to `budget` chars for prompt injection, marking truncation. Mirrors
/// `prompt.rs::project_context()`'s clip-and-mark approach for `AGENTS.md`/`CLAUDE.md`.
fn clip(content: &str, budget: usize) -> String {
    let trimmed = content.trim();
    let clipped: String = trimmed.chars().take(budget).collect();
    if clipped.len() < trimmed.len() {
        format!("{clipped}\n[... truncated ...]")
    } else {
        clipped
    }
}

/// Formatted block for system-prompt injection, or `None` if the project memory is empty.
pub fn project_prompt_block(working_dir: &Path, budget: usize) -> Option<String> {
    let content = read_project(working_dir);
    if content.trim().is_empty() {
        None
    } else {
        Some(format!("\n{}\n", clip(&content, budget)))
    }
}

/// Formatted block for system-prompt injection, or `None` if the user memory is empty.
pub fn user_prompt_block(budget: usize) -> Option<String> {
    let content = read_user();
    if content.trim().is_empty() {
        None
    } else {
        Some(format!("\n{}\n", clip(&content, budget)))
    }
}
