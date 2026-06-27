# Roadmap: Professional Local Coding Agent

> Goal: turn Agent Aleph from a local model manager into a professional-grade **coding agent**
> comparable to opencode or Claude Code, while staying **100% local** on top of llama.cpp.

## Progress

- ✅ **Phase 1** — Agent loop, GBNF grammar, `read_file`, and step UI. Validated with a 0.8B model.
- ✅ **Phase 2** — `trait Tool` + registry; `list`, `glob`, `grep`, `write_file`, `edit`,
  and `bash`; `allow` / `ask` / `deny` permissions with a UI round trip; build/plan modes.
- ✅ **Phase 3** — Loop detection, context management, per-step token caps, and hardened prompts.
  Validated: it stops loops and avoids context overflow.
- 🚧 **Phase 4** — Done: runtime optimization (quantized KV cache, batch, mmap/mlock), external
  model folders, and persistent sessions with memory across turns. Pending: subagents, metrics,
  and speculative decoding.
- 🚧 **Phase 5** — Harness professionalization after review.

## Phase 5: Harness Professionalization

Gaps were identified in an external review and checked against the code on 2026-06-24. Work
was ordered by structural dependency: rich messages unlock native tool calling, compaction,
and cleaner UI reconstruction.

### Done

- ✅ **Per-tool schema, per-tool GBNF, and argument validation.** `params()` on the `Tool`
  trait, grammar tied to the exact schema, and pre-execution validation are in place.
- ✅ **Priority 1: rich messages (`agent/message.rs`).** `Role::{System, User, Assistant,
  Tool}`, `AgentMsg`, semantic constructors, tool metadata, and a `to_wire()` layer isolate
  internal messages from transport formatting. Legacy sessions remain compatible.
- ✅ **Priority 2: summarizing compaction (`agent/context.rs`).** `maybe_compact` preserves the
  system/task header and recent tail, summarizes the middle with a dedicated low-temperature
  model call, and replaces it with one summary message.
- ✅ **Priority 3: native tool calling.** `--jinja`, OpenAI-style `tools`, streaming
  `delta.tool_calls`, multiple calls per step, native `tool` role wire format, and automatic
  route selection are implemented. `Settings.tool_calling` supports `auto`, `native`, and
  `grammar`.

### Cheap Safety And Quality Work

- ✅ `bash`: destructive-command blacklist, 60s timeout, and output truncation.
- ✅ Project-context injection: `prompt.rs` reads `AGENTS.md` and `CLAUDE.md` from the working
  directory and injects them with priority.
- ✅ State-aware loop detection: successful writes/commands reset repetition signatures because
  a world mutation invalidates previous loop evidence.
- Pending: make `grep` use regex/ripgrep and honor `.gitignore` instead of scanning with a
  literal `contains` walk.

### Agent Panel

- ✅ **Skills** — local instruction/resource packs in
  `data_dir/agent-aleph/skills/<slug>/SKILL.md`, with active state in `enabled.json`.
  Commands support list/create/import/delete/toggle/read. Active skills are injected into the
  system prompt.
- ✅ **Attached context** — `read_context_file` plus shared `agentContext.svelte.ts` store.
  The agent panel can attach files/text, and `AgentView.send` prepends that block to the turn.
- ⏳ **Connections** — GitHub/Google are visible as placeholders. Real integration requires
  network access and OAuth/token handling, outside the current local-first scope.

### Larger Work Toward Local opencode

- Subagents (`Task` / `@general`)
- MCP
- Slash commands (`/commit`, `/review`, `/compact`)
- Real diffs in the permission UI
- Runtime metrics: tokens/sec, steps, and context usage

### Code Quality Notes

- ✅ `run_inner` is lighter: per-call logic now lives in `process_call`, shared by grammar and
  native routes.
- `budget_view` previously used repeated removal in a loop; it should stay simple now that
  compaction exists, but it can still be cleaned up.
- `walk_files` rereads up to 5000 files at depth 12 on every grep/glob without cache or
  `.gitignore`.

---

## Vision

Agent Aleph joins two halves:

- **The LM Studio half:** catalog, Hugging Face downloads, llama-server, GPU offload, and
  inference settings.
- **The Codex/opencode half:** an agentic harness with a tool loop, permissions, sessions,
  context management, and a step-based UI.

The product is the harness. The model runtime enables it. Final quality depends on two
parallel tracks:

1. A local model that is capable enough and fast enough.
2. A robust harness that gets the most out of imperfect local models.

### Design Principle For Weaker Local Models

Compared with frontier hosted models, local models fail more often: they get lost, repeat
tools, and hallucinate formatting. The harness compensates by design:

- **Grammar-constrained output (GBNF)** for syntactically valid tool calls.
- **Defensive loop** with step caps, repetition detection, and argument validation.
- **Aggressive context management** because local context is smaller and more expensive.
- **High-signal prompts and tools** to reduce ambiguity.

---

## Baseline

| Layer | File | Status |
|-------|------|--------|
| Model runtime | `inference/server.rs` | ✅ Solid. Runtime controls have been added over time. |
| Streaming chat | `chat/session.rs` | ⚠️ Simple chat path; agent mode uses the harness. |
| State | `state.rs` | ⚠️ App state is centralized; agent sessions add richer state. |
| IPC | `commands.rs` / `api.ts` | ✅ Clean pattern. New backend operations should be wrapped here. |
| Frontend | `AgentView.svelte` | ✅ Step-based agent UI exists; diff/metrics polish remains. |

---

## Target Architecture

### Backend Modules

```text
src-tauri/src/
├── agent/
│   ├── mod.rs
│   ├── agent_loop.rs       # agent loop orchestrator
│   ├── message.rs          # rich message types
│   ├── tools/
│   │   ├── mod.rs          # Tool trait + registry
│   │   ├── read_file.rs
│   │   ├── write_file.rs
│   │   ├── edit.rs
│   │   ├── bash.rs
│   │   ├── grep.rs
│   │   ├── glob.rs
│   │   └── list.rs
│   ├── grammar.rs          # GBNF from JSON schemas
│   ├── permissions.rs      # allow/ask/deny policy
│   ├── context.rs          # token estimation, truncation, compaction
│   ├── prompt.rs           # agent system prompt
│   └── session_store.rs    # persisted agent sessions
├── inference/
├── chat/
├── models/
└── ...
```

### Conceptual Layers

1. **Provider** — abstraction over inference. Today: local llama-server.
2. **Session** — rich message history, mode, active model, working directory, and context budget.
3. **Agent / Mode** — build mode for full access, plan mode for read-only work.
4. **Tool** — executable capability with JSON schema, validation, and executor.
5. **Permission** — cross-cutting `allow` / `ask` / `deny` layer.
6. **Loop** — the engine that binds everything together.

---

## Agent Loop

```text
run_agent_turn(session, user_input):
    push user message
    for step in MAX_STEPS:
        build request from messages + tools + grammar/native settings
        stream inference and emit tokens to UI
        parse tool calls

        if no tool calls:
            push final assistant message
            return done

        push assistant message with tool calls
        for call in tool calls:
            validate arguments
            check permissions
            ask the UI if needed
            execute the tool
            push tool result

        detect loops
        compact context if needed

    return max_steps
```

Key decisions:

- Streaming and the loop coexist: each step emits tokens while the backend keeps the response
  needed to parse tool calls.
- Tool calling has two routes: native OpenAI-style tools for capable models and GBNF as the
  universal fallback.
- Cancellation reuses the existing `CancellationToken` infrastructure.

---

## Tool Set

| Tool | Description | Default permission | Plan mode |
|------|-------------|--------------------|-----------|
| `read_file` | Read a file with offset/limit pagination | allow | allow |
| `list` / `glob` | List directories / find by pattern | allow | allow |
| `grep` | Search file contents | allow | allow |
| `write_file` | Create or overwrite a file | ask | deny |
| `edit` | Exact string replacement | ask | deny |
| `bash` | Run a shell command | ask | ask |

Design notes:

- Each tool implements a shared trait with name, description, JSON schema, and async executor.
- `edit` uses exact unique-string semantics, which is more reliable for local models than
  free-form diffs.
- `read_file` is paginated and line-numbered to control context cost.
- `bash` has timeout, stdout/stderr capture, working-directory scoping, and output truncation.
- Tool outputs are truncated/summarized before re-entering model context.

---

## Security And Permissions

This app can write files and execute shell commands, so the dangerous parts are explicit:

- The user chooses a project working directory; file tools are restricted to that root.
- Per-tool policy supports `allow`, `ask`, and `deny`, with mode-specific overrides.
- When a decision is `ask`, the backend emits `agent://permission-request`, the UI shows the
  action, and the user approves or denies it.
- Network isolation for `bash` is documented as future work.
- Destructive commands are blacklisted or require stronger confirmation.

---

## Context Management

Local models usually have smaller contexts, and every token costs speed:

- Approximate token counting per message.
- Tool-output truncation before model-context insertion.
- Conversation compaction when approaching `context_size`.
- Paginated file reads instead of dumping full files.
- UI context usage indicators are planned.

---

## Runtime Optimization

The runtime track exists so larger and more capable local models can fit:

- Exposed settings for KV-cache quantization, mmap/mlock, batch size, GPU layers, and related
  knobs.
- Automatic GPU-layer suggestions from detected free VRAM.
- Model and quantization guidance based on detected RAM/VRAM, prioritizing IQ3/IQ4 and MoE
  models that activate fewer parameters per token.

---

## IPC And Types

Agent commands:

- `agent_send(session_id, working_dir, mode, input)`
- `agent_stop(session_id)`
- `respond_permission(request_id, decision)`
- `set_agent_mode(session_id, mode)`
- `list_tools()`

Agent events:

- `agent://token`
- `agent://step`
- `agent://tool-result`
- `agent://permission-request`
- `agent://done`

Rich message types in Rust and TypeScript include `role`, `content`, `tool_calls`,
`tool_call_id`, and `tool_name`.

---

## Frontend

- `AgentView.svelte`: step rendering, assistant messages, tool calls, collapsible results,
  and permission flow.
- Build/plan mode selector.
- Permission UI for commands and file writes.
- Project-folder selector through the Tauri dialog plugin.
- Context and step indicators.
- The simple chat path remains available as Ask mode without tools.

---

## Phased Roadmap

### Phase 0: Foundations

- Runtime controls in `Settings` / `server.rs`.
- Automatic `n_gpu_layers`.
- Rich message types mirrored in TypeScript.

### Phase 1: Minimal End-To-End Loop

- Agent loop with step cap.
- `read_file`.
- GBNF tool calling.
- `agent_send` and `agent://*` events.
- Basic `AgentView`.

### Phase 2: Tool Set And Permissions

- `list`, `glob`, `grep`, `write_file`, `edit`, and `bash`.
- Permission policy and UI confirmation.
- Working-directory scoping.
- Build/plan modes.

### Phase 3: Robustness

- Context counting, truncation, compaction, and paginated reading.
- Loop detection and argument validation.
- Native tool calling alongside grammar.
- Model-family-aware prompts.

### Phase 4: Professional Polish

- Speculative decoding.
- Hardware-based model and quantization suggestions.
- Persistent sessions.
- Subagents.
- Metrics: tokens/sec, steps, and context usage.

---

## Risks And Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Local model is too weak for agentic work | High | GBNF, high-signal prompts, Coder/MoE focus, runtime optimization |
| Loops or nontermination | Medium | Step cap, repetition detection, system notes |
| Context overflow | Medium | Truncation, compaction, paginated reads |
| Dangerous execution | High | Ask/deny permissions, scoped working dir, blacklist, plan mode |
| Malformed tool calls | Medium | Grammar, schema validation, error feedback |
| Slow multi-step loop | Medium | KV-cache reuse, speculative decoding, MoE models |

---

## Next Proposed Step

The original proof-of-concept path was Phase 1: a minimal loop, `read_file`, GBNF, and a
basic `AgentView`. That path is complete. The next high-leverage items are real permission
diffs, metrics, subagents, and connector work once the project is ready to move beyond the
strict local-first boundary.
