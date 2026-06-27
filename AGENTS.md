# AGENTS.md

> Project notes for coding agents working in this repository. This document records the
> local-agent evaluation harness, the bugs found while testing with real GGUF models, the
> fixes that landed, and how to reproduce the evaluation runs.

---

## Testing The Agent Harness With Local Models

### Motivation

The agent harness (`src-tauri/src/agent/`) was originally invoked only from the Tauri UI
through `agent_send` IPC and manual permissions. That made it impractical to run a large,
repeatable task set against a real model. Without a headless mode, changes to the loop,
tools, or context handling could only be verified by manually testing the app. The project
needed an automated evaluation harness that exercises the **same `run_inner` used by the
production app**.

### What Was Built

#### 1. `agent_loop.rs` Refactor

The loop body was tightly coupled to `app.emit(...)` and `AppState` for permissions.
The refactor introduced:

- **`trait LoopSink`** (`agent_loop.rs`): abstracts loop events (`emit_step`,
  `emit_token`, `emit_tool`, `emit_permission`, `emit_done`). The Tauri path uses
  `TauriSink` to re-emit to the webview; a headless harness can use an in-memory sink.
- **Parameterized `run_inner`**: accepts `sink: &dyn LoopSink`,
  `state: Option<&Arc<AppState>>`, `auto_allow: bool`, and `persist: bool`. In Tauri mode,
  `state=Some`, `auto_allow=false`, and `persist=true`, preserving the user-facing behavior.
  In headless mode, `state=None`, `auto_allow=true`, and `persist=false`, so `Ask` becomes
  `Allow` without touching the UI and sessions are not written to disk.
- **Public headless `run_turn`**: one-call entry point to run a turn against an already
  running `llama-server` on a given port, with permissions auto-approved. It returns
  `(final_text, reason)` where `reason` is one of `done`, `max_steps`, `loop`, `cancelled`,
  or `error`.
- `use_native_tools` and `parse_tool_call` were made public so the harness can inspect or
  force the tool-calling route.

Verification: the original `cargo check` continued to pass after the refactor. The Tauri app
behavior did not change; the code was reorganized so it could be tested.

#### 2. `agent_eval` Binary

`src-tauri/src/bin/agent_eval.rs` is a standalone evaluation harness:

1. Loads user settings from `~/.config/agent-aleph/settings.json` and overrides them with CLI
   flags such as `--context-size`, `--max-tokens`, `--threads`, and `--temperature`.
2. Builds a reproducible fixture project in `/tmp/opencode/agent-eval/fixture`, including
   `package.json`, a README with a described bug, `AGENTS.md`, source files, tests, docs,
   numeric data, an empty file, a binary blob, and type declarations. Each task receives a
   fresh working-directory copy of that pristine fixture.
3. Starts `llama-server` with `spawn_server`, mirroring `inference::server::start_server`
   without an `AppHandle`. It uses `--jinja` only for the native route and preserves KV-cache,
   mmap/mlock, device, and related settings. It waits for `/health` with a generous timeout
   because larger models load slowly.
4. Runs 54 tasks against `run_turn`, which uses the same `run_inner` as the app. A
   `RecordSink` stores every tool call and final-token stream in memory.
5. Checks each task with a function that receives `(working_dir, final_text, calls)` and
   returns pass/fail. Checkers combine expected tool usage, final-text content, and file-system
   state for write/edit tasks.
6. Logs progress to stderr and writes `route-<route>.{json,md}` transcripts plus a global
   `report.json`. `--limit N` runs a smoke test, `--only id1,id2` filters tasks, and
   `AGENT_EVAL_DEBUG=1` dumps requests when context overflows.

The 54 tasks cover all 7 tools: `read_file`, `list`, `glob`, `grep`, `write_file`, `edit`,
and `bash`, plus mixed realistic tasks such as fixing a bug and running tests. They also
exercise failure paths: missing files, ambiguous edits, listing a file, and empty grep
matches.

#### 3. `[[bin]] agent_eval` And `default-run`

```toml
default-run = "agent-aleph"

[[bin]]
name = "agent_eval"
path = "src/bin/agent_eval.rs"
```

Without `default-run`, `cargo run` failed because the workspace now has both `agent-aleph`
and `agent_eval` binaries.

### Reproducing The Evaluation

```bash
# Run from clon-codex/src-tauri/ so binaries/llama-linux-x64/ resolves correctly.
cargo build --bin agent_eval --release

# Full run: 54 tasks x 2 routes = 108 turns, roughly 10-15 minutes with a 4B model.
./target/release/agent_eval \
  --model /mnt/disco_d/MODELOS/lmstudio-community/Qwen3.5-4B-GGUF/Qwen3.5-4B-Q4_K_M.gguf \
  --routes grammar,native \
  --out /tmp/opencode/eval-out \
  --context-size 8192 --max-tokens 1024

# Smoke test: 3 tasks, 1 route.
./target/release/agent_eval --model <gguf> --routes grammar --limit 3 --out /tmp/opencode/smoke

# Debug a context overflow.
AGENT_EVAL_DEBUG=1 ./target/release/agent_eval --model <gguf> --only r3 --routes grammar --out /tmp/opencode/dbg

# Run selected tasks.
./target/release/agent_eval --model <gguf> --routes grammar --only r3,w6,e6 --out /tmp/opencode/spot
```

Outputs: `report.json`, `route-grammar.{json,md}`, and `route-native.{json,md}`. The Markdown
files include a summary table and the tool-call transcript plus final text for each task.

### Tested Model

**Qwen3.5-4B-Q4_K_M** (`/mnt/disco_d/MODELOS/lmstudio-community/Qwen3.5-4B-GGUF/`, 2.7 GB),
context size 8192, max_tokens 1024, temperature 0.4. Both routes were tested:

- `grammar`: GBNF, no `--jinja`
- `native`: `--jinja` plus `delta.tool_calls`

The bundled `llama-server` binary is the Vulkan x64 build 9754.

### Harness Bugs Found And Fixed

#### Bug 1: Context Overflow From Dense Tool Results

**Symptom:** task `r3`, reading `data.txt` with `offset=10`, failed with `reason=error` and
an HTTP 400: the request exceeded the 8192-token context size. A single dense numeric tool
result could collapse a session.

**Diagnosis:** the `data.txt` tool result was 10760 bytes and 246 lines. The real tokenizer
counted it as roughly 7100 tokens, but `approx_tokens` estimated around 3840. The heuristic
underestimated dense numeric/code output by about 1.85x because each number, operator,
underscore, and tab can become separate tokens.

**Fixes** (`src-tauri/src/agent/agent_loop.rs`, `agent/message.rs`):

1. `approx_tokens` now accounts for line density: `bytes/3 + lines + 4`.
2. `ctx_budget` reserves roughly 25% of the context (`ctx*3/4 - step_max_tokens`) instead of
   using the earlier narrow margin.
3. Tool results are capped to `RESULT_CTX_CAP = 6000` characters before entering the model
   context through `cap_for_context`. The UI still receives the full result via
   `sink.emit_tool`; only the model-context copy is truncated.

In the GBNF route, invalid raw JSON caused by `max_tokens` truncation is also capped to
400 characters before being stored in the conversation.

**Verification:** two full evaluation runs, 108 tasks each, produced zero `reason=error`
context-overflow failures.

#### Evaluation Checker Fixes

Some checkers were too strict or encoded impossible tasks, causing false failures:

- `w6`: now accepts trailing newlines and allows `bash` setup such as `mkdir -p`.
- `b3`: accepts both English and Spanish fail stems (`fail` / `fall`) because models may
  answer in either language.
- `p3`: now searches for `function` instead of the Spanish word `funcion`, which was absent
  from the fixture.
- `e6`: now accepts either reporting the ambiguity or resolving it with multiple precise
  edits, which is valid agent behavior.

### Results

| Route | Run 1 (ok/54) | Run 2 (ok/54) |
|-------|---------------|---------------|
| grammar | 49 | 47 |
| native | 45 | 49 |

There were zero context-overflow errors. Remaining failures were `reason=done`, meaning the
model completed but answered incorrectly: 4B limitations rather than harness failures. The
model is also somewhat nondeterministic at temperature 0.4, so run-to-run variance of about
two tasks is expected.

Conclusion: GBNF and native tool calling are comparable on a 4B model. GBNF is slightly more
stable at conserving steps; native works better when the model benefits from the structured
tool format. The refactor did not regress the Tauri app.

### Modified Files

- `src-tauri/src/agent/agent_loop.rs` — `LoopSink`, `TauriSink`, headless `run_turn`, overflow
  fixes, context caps, debug opt-in.
- `src-tauri/src/agent/message.rs` — improved `approx_tokens`.
- `src-tauri/src/agent/mod.rs` — re-exports `run_turn` and `LoopSink`.
- `src-tauri/Cargo.toml` — `default-run = "agent-aleph"` and `[[bin]] agent_eval`.
- `src-tauri/src/bin/agent_eval.rs` — evaluation harness.
- `AGENTS.md` — this documentation.

### Evaluation Backlog

- Persist inference settings into the report for run comparability.
- Resample nondeterministic tasks with multiple temperatures to separate model capability from
  one-off failures.
- Add GPU/port cooldown between routes to reduce the risk of a second `llama-server` OOM.
- Validate tool-call arguments in checkers, not only tool names and final text.
- Grow to a 100+ task suite, including context-compaction stress cases.
- Run `agent_eval` in GitHub Actions with a small 0.8B model for fast PR regression coverage.
