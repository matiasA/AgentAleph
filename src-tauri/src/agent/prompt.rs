use crate::agent::skills::SkillDoc;
use crate::agent::tools::{ParamType, ToolDoc};
use std::path::Path;

/// Repository instruction files injected into the system prompt, in priority order.
const PROJECT_DOC_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md"];
/// Character cap per project-context file to avoid overflowing local context.
const PROJECT_DOC_MAX: usize = 8000;

/// Reads project instructions from the working directory, if present.
/// Returns a formatted prompt block or `None`.
fn project_context(working_dir: &str) -> Option<String> {
    let root = Path::new(working_dir);
    let mut out = String::new();
    for name in PROJECT_DOC_FILES {
        let path = root.join(name);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        let clipped: String = text.chars().take(PROJECT_DOC_MAX).collect();
        let truncated = if clipped.len() < text.len() {
            "\n[... truncated ...]"
        } else {
            ""
        };
        out.push_str(&format!("\n### {name}\n{clipped}{truncated}\n"));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn type_name(t: ParamType) -> &'static str {
    match t {
        ParamType::Str => "string",
        ParamType::Int => "number",
        ParamType::Bool => "bool",
    }
}

/// Typed tool signature, for example `read_file(path: string, offset?: number)`.
fn signature(d: &ToolDoc) -> String {
    let ps: Vec<String> = d
        .params
        .iter()
        .map(|p| {
            let sep = if p.required { ": " } else { "?: " };
            format!("{}{sep}{}", p.name, type_name(p.ty))
        })
        .collect();
    format!("{}({})", d.name, ps.join(", "))
}

/// Agent system prompt. `native` selects the contract: native tool calls or GBNF JSON.
/// `memory_project`/`memory_user` are pre-formatted blocks from `agent::memory` (already
/// clipped to their budget), or `None` if empty/disabled.
pub fn system_prompt(
    working_dir: &str,
    docs: &[ToolDoc],
    native: bool,
    skills: &[SkillDoc],
    memory_project: Option<&str>,
    memory_user: Option<&str>,
) -> String {
    let mut tools_desc = String::new();
    for d in docs {
        tools_desc.push_str(&format!("- {} — {}\n", signature(d), d.description));
    }
    let project = match project_context(working_dir) {
        Some(ctx) => format!(
            "\n\nProject instructions from the repository. These override default \
             preferences when relevant:{ctx}"
        ),
        None => String::new(),
    };
    let skills_block = if skills.is_empty() {
        String::new()
    } else {
        let mut s = String::from(
            "\n\nSpecialized knowledge from active skills. Apply it when relevant:",
        );
        for sk in skills {
            s.push_str(&format!("\n\n### {}\n{}", sk.name, sk.body));
        }
        s
    };
    let memory_block = {
        let mut s = String::new();
        if let Some(p) = memory_project {
            s.push_str(
                "\n\nPersistent project memory (facts/decisions recorded in earlier \
                 sessions on this project):",
            );
            s.push_str(p);
        }
        if let Some(u) = memory_user {
            s.push_str(
                "\n\nPersistent user memory (the user's preferences, apply across all \
                 projects):",
            );
            s.push_str(u);
        }
        if !s.is_empty() {
            s.push_str(
                "\nMemory is accumulated knowledge, not the source of truth: safety/\
                 permission rules always win, then project instructions (AGENTS.md/\
                 CLAUDE.md) on technical conflicts, then project memory over user \
                 memory for this repo's conventions. Never store secrets, credentials, \
                 API keys, or tokens in memory.",
            );
        }
        s
    };
    let head = format!(
        "You are a coding agent running locally on the user's machine.\n\
         Project directory: {working_dir}{project}{skills_block}{memory_block}"
    );
    if native {
        format!(
            r#"{head}

You have tools for inspecting and modifying the project. Invoke them through the system's tool calls; do not write JSON manually. Once you have the answer or have completed the task, reply in plain text without calling more tools.

Available tools:
{tools_desc}
Rules:
- Do not invent file contents: read files with read_file before making claims about them.
- Do not repeat a tool with the same arguments: the result would be identical.
- As soon as you have the requested information, give your final answer in plain text. Do not keep exploring.
- Never announce a future step in your final answer ("next I will...", "then I will generate..."). If the task needs more steps, execute them with tools now; the final answer only describes what you already did.
- Report results honestly: if something failed or remains undone, say so explicitly; do not claim something is done or verified unless you checked it with a tool.
- When you learn a durable fact worth remembering for future sessions (a project convention, a decision, a user preference), record it with the memory tool. Don't record trivial or session-specific details."#
        )
    } else {
        format!(
            r#"{head}

You work in steps. On EACH turn, respond with EXACTLY ONE JSON object and no text before or after it, with this exact shape:
{{"tool": "<nombre>", "args": {{ ... }}}}

Respect each tool's required arguments and types. Optional arguments are marked with "?".

Available tools:
{tools_desc}- final(text: string) — finish the task and return the final answer to the user.

Rules:
- Use exactly one tool per step. After seeing its result, decide the next step.
- Do not invent file contents: read files with read_file before making claims about them.
- NEVER repeat a tool with the same arguments: the result would be identical.
- As soon as a tool result already contains the requested information, immediately respond with the "final" tool. Do not keep exploring if you already have the answer.
- "final" means the task is ALREADY complete. Never use it to announce future steps ("next I will generate...", "then I will convert..."): if steps remain, execute them first with the appropriate tools and call "final" only when the work is genuinely done.
- Report results honestly: if something failed or remains undone, say so explicitly in the "final" text; do not claim something is done or verified unless you checked it with a tool.
- When you learn a durable fact worth remembering for future sessions (a project convention, a decision, a user preference), record it with the memory tool. Don't record trivial or session-specific details."#
        )
    }
}
