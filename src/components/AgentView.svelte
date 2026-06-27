<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    api,
    onAgentToken,
    onAgentStep,
    onAgentTool,
    onAgentDone,
    onAgentPermission,
  } from "../lib/api";
  import type { AgentMsg, ModelStatus, SessionMeta } from "../lib/types";
  import { buildContextBlock } from "../lib/agentContext.svelte";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import Select from "./Select.svelte";

  let {
    status,
    sessionId,
  }: {
    status: ModelStatus;
    sessionId: string;
  } = $props();

  type Item =
    | { kind: "user"; text: string }
    | { kind: "model"; text: string; reasoning: string; streaming: boolean }
    | { kind: "tool"; tool: string; args: string; result: string; isError: boolean }
    | { kind: "final"; text: string }
    | { kind: "error"; text: string };

  type Pending = { request_id: string; tool: string; summary: string };

  let sid = $state(sessionId);
  let sessions = $state<SessionMeta[]>([]);
  let items = $state<Item[]>([]);
  let input = $state("");
  let running = $state(false);
  let workingDir = $state("");
  let mode = $state<"build" | "plan">("build");
  let pending = $state<Pending | null>(null);
  let scrollEl: HTMLDivElement | null = $state(null);
  let modelIdx = $state<number | null>(null);

  let sessionOptions = $derived([
    ...(sessions.find((s) => s.id === sid)
      ? []
      : [{ value: sid, label: "· current session (unsaved)" }]),
    ...sessions.map((s) => ({ value: s.id, label: s.title })),
  ]);

  let unlisten: Array<() => void> = [];

  $effect(() => {
    refreshSessions();
    (async () => {
      unlisten.push(
        await onAgentStep((e) => {
          if (e.session_id !== sid) return;
          if (e.phase === "model_start") {
            items.push({ kind: "model", text: "", reasoning: "", streaming: true });
            modelIdx = items.length - 1;
            scrollToBottom();
          }
        })
      );
      unlisten.push(
        await onAgentToken((e) => {
          if (e.session_id !== sid || modelIdx === null) return;
          const m = items[modelIdx];
          if (m?.kind !== "model") return;
          if (e.is_reasoning) m.reasoning += e.token;
          else m.text += e.token;
          scrollToBottom();
        })
      );
      unlisten.push(
        await onAgentTool((e) => {
          if (e.session_id !== sid) return;
          if (modelIdx !== null && items[modelIdx]?.kind === "model") {
            (items[modelIdx] as any).streaming = false;
          }
          items.push({
            kind: "tool",
            tool: e.tool,
            args: e.args,
            result: e.result,
            isError: e.is_error,
          });
          modelIdx = null;
          scrollToBottom();
        })
      );
      unlisten.push(
        await onAgentDone((e) => {
          if (e.session_id !== sid) return;
          // Close the streaming model bubble; discard it if it stayed empty.
          if (modelIdx !== null && items[modelIdx]?.kind === "model") {
            const mb = items[modelIdx] as any;
            mb.streaming = false;
            if (!mb.text.trim() && !(mb.reasoning ?? "").trim()) {
              items.splice(modelIdx, 1);
              modelIdx = null;
            }
          }
          if (e.reason === "done") {
            // In the native route, the final text was already streamed into the bubble: do not duplicate it.
            const last = items[items.length - 1] as any;
            if (last?.kind === "model" && (e.text ?? "").trim() && last.text.trim() === e.text.trim()) {
              items[items.length - 1] = { kind: "final", text: e.text };
            } else {
              items.push({ kind: "final", text: e.text });
            }
          } else if (e.reason === "max_steps" || e.reason === "loop") {
            items.push({ kind: "error", text: e.text });
          } else if (e.error && e.error !== "cancelled") {
            items.push({ kind: "error", text: e.error });
          }
          running = false;
          modelIdx = null;
          pending = null;
          refreshSessions();
          scrollToBottom();
        })
      );
      unlisten.push(
        await onAgentPermission((e) => {
          if (e.session_id !== sid) return;
          pending = { request_id: e.request_id, tool: e.tool, summary: e.summary };
          scrollToBottom();
        })
      );
    })();
    return () => {
      unlisten.forEach((u) => u());
      unlisten = [];
    };
  });

  function scrollToBottom() {
    queueMicrotask(() => {
      if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
  }

  async function pickDir() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") workingDir = dir;
  }

  async function send() {
    const text = input.trim();
    if (!text || running) return;
    if (!status.loaded) {
      alert("Load a model first from the Models tab");
      return;
    }
    if (!workingDir) {
      alert("Choose the project folder first");
      return;
    }
    input = "";
    items.push({ kind: "user", text });
    running = true;
    scrollToBottom();
    try {
      // Attached context from the right panel is prepended to the input sent to the model;
      // the user bubble shows only the user's text.
      const ctx = buildContextBlock();
      await api.agentSend(sid, workingDir, mode, ctx ? `${ctx}\n${text}` : text);
    } catch (e: any) {
      running = false;
      items.push({ kind: "error", text: String(e) });
    }
  }

  async function respond(approved: boolean, remember = false) {
    if (!pending) return;
    const req = pending.request_id;
    pending = null;
    await api.respondPermission(req, approved, remember);
  }

  async function stop() {
    pending = null;
    await api.agentStop(sid);
  }

  function clearAll() {
    if (running) return;
    items = [];
    modelIdx = null;
  }

  // ---------- Sessions ----------

  async function refreshSessions() {
    try {
      sessions = await api.listAgentSessions();
    } catch {
      sessions = [];
    }
  }

  function newSession() {
    if (running) return;
    sid = "agent-" + crypto.randomUUID();
    items = [];
    modelIdx = null;
    pending = null;
  }

  async function openSession(id: string) {
    if (running || id === sid) return;
    const s = await api.loadAgentSession(id);
    if (!s) return;
    sid = s.id;
    workingDir = s.working_dir;
    mode = s.mode === "plan" ? "plan" : "build";
    items = reconstruct(s.messages);
    modelIdx = null;
    pending = null;
    scrollToBottom();
  }

  async function deleteSession(id: string) {
    if (!confirm("Delete this session?")) return;
    await api.deleteAgentSession(id);
    if (id === sid) newSession();
    await refreshSessions();
  }

  function tryParse(s: string): { tool?: string; args?: any } | null {
    try {
      const v = JSON.parse(s);
      return v && typeof v === "object" ? v : null;
    } catch {
      return null;
    }
  }

  /** Rebuilds the visible timeline from the saved conversation. */
  function reconstruct(messages: AgentMsg[]): Item[] {
    const out: Item[] = [];
    let lastTool = "?";
    let lastArgs = "";
    for (const m of messages) {
      // Harness notes/errors and the system prompt stay hidden.
      if (m.role === "system") continue;
      if (m.role === "assistant") {
        // Native route: calls arrive in tool_calls; content may be empty.
        if (m.tool_calls && m.tool_calls.length) {
          const c = m.tool_calls[0];
          lastTool = c.name;
          lastArgs = JSON.stringify(c.args ?? {});
          if (m.content.trim()) {
            out.push({ kind: "model", text: m.content, reasoning: "", streaming: false });
          }
        } else {
          // GBNF route: the call, or "final", is JSON in content.
          const p = tryParse(m.content);
          if (p?.tool === "final") {
            out.push({ kind: "final", text: p.args?.text ?? m.content });
          } else {
            out.push({ kind: "model", text: m.content, reasoning: "", streaming: false });
            if (p?.tool) {
              lastTool = p.tool;
              lastArgs = JSON.stringify(p.args ?? {});
            }
          }
        }
      } else if (m.role === "tool") {
        // Tool result in the rich message model: content is raw output.
        out.push({
          kind: "tool",
          tool: m.tool_name ?? lastTool,
          args: lastArgs,
          result: m.content,
          isError: m.is_error ?? false,
        });
      } else if (m.role === "user") {
        // Backward compatibility with old sessions: tool results/notes were reinjected as user messages.
        if (m.content.startsWith("Resultado de ")) {
          const nl = m.content.indexOf("\n");
          const header = nl >= 0 ? m.content.slice(0, nl) : m.content;
          const result = nl >= 0 ? m.content.slice(nl + 1) : "";
          out.push({
            kind: "tool",
            tool: lastTool,
            args: lastArgs,
            result,
            isError: header.includes("(ERROR)"),
          });
        } else if (
          m.content.startsWith("Ya ejecutaste") ||
          m.content.startsWith("Error: tu respuesta") ||
          m.content.startsWith("[…")
        ) {
          // Internal harness messages from old sessions are hidden.
          continue;
        } else {
          out.push({ kind: "user", text: m.content });
        }
      }
    }
    return out;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="col" style="flex:1;overflow:hidden">
  <div class="agent-header row between">
    <div class="row" style="gap:8px">
      <span class="small muted">Agent</span>
      {#if status.loaded}
        <span class="tag accent">{status.model_name}</span>
      {:else}
        <span class="tag warn">no model</span>
      {/if}
    </div>
    <div class="row" style="gap:8px">
      <div class="mode-pill">
        <button class:active={mode === "build"} disabled={running} onclick={() => (mode = "build")}>build</button>
        <button class:active={mode === "plan"} disabled={running} onclick={() => (mode = "plan")}>plan</button>
      </div>
      <button class="ghost small-btn" onclick={pickDir} title={workingDir}>
        <Icon name="folder" size="sm" />
        {workingDir ? workingDir.split("/").pop() : "Choose folder"}
      </button>
      <button class="ghost small-btn" onclick={clearAll} disabled={running || items.length === 0}>
        Clear
      </button>
    </div>
  </div>

  <div class="session-bar">
    <button class="ghost small-btn" onclick={newSession} disabled={running}>+ New</button>
    <div class="session-select">
      <Select
        value={sid}
        options={sessionOptions}
        disabled={running}
        onChange={(v) => openSession(v)}
      />
    </div>
    {#if sessions.find((s) => s.id === sid)}
      <button class="ghost small-btn danger" onclick={() => deleteSession(sid)} disabled={running} title="Delete session">
        ✕
      </button>
    {/if}
  </div>

  <div class="agent-scroll" bind:this={scrollEl}>
    {#if items.length === 0}
      <div class="empty">
        <div class="empty-orb">
          <span class="orb-halo"></span>
          <Logo size={80} />
        </div>
        <div class="empty-title">Agent Ready</div>
        <div class="empty-sub">
          Choose a project folder and describe a task. The agent will use tools step by step.
        </div>
      </div>
    {:else}
      {#each items as it, i (i)}
        {#if it.kind === "user"}
          <div class="bubble user">{it.text}</div>
        {:else if it.kind === "model"}
          <div class="step model">
            {#if it.reasoning}
              <div class="reasoning">{it.reasoning}</div>
            {/if}
            <div class="label dim">decision {it.streaming ? "·…" : ""}</div>
            <pre class="json">{it.text}</pre>
          </div>
        {:else if it.kind === "tool"}
          <div class="step tool" class:err={it.isError}>
            <div class="label"><Icon name="terminal" size="sm" /> <strong>{it.tool}</strong> <span class="dim mono">{it.args}</span></div>
            <pre class="result">{it.result}</pre>
          </div>
        {:else if it.kind === "final"}
          <div class="bubble final">{it.text}</div>
        {:else if it.kind === "error"}
          <div class="bubble err">⚠ {it.text}</div>
        {/if}
      {/each}
    {/if}
  </div>

  {#if pending}
    <div class="permission">
      <div class="perm-text">
        <span class="perm-tag">permission</span>
        The agent wants to: <strong>{pending.summary}</strong>
        <div class="dim small" style="margin-top:2px">
          "Always allow" will not ask again for <strong>{pending.tool}</strong> during this session.
        </div>
      </div>
      <div class="row" style="gap:8px">
        <button class="danger small-btn" onclick={() => respond(false)}>Reject</button>
        <button class="ghost small-btn" onclick={() => respond(true, true)}>
          Always allow
        </button>
        <button class="solid small-btn" onclick={() => respond(true)}>Allow</button>
      </div>
    </div>
  {/if}

  <div class="composer-wrap">
    <div class="composer" class:disabled={!status.loaded}>
      <textarea
        placeholder={status.loaded ? "Describe a task for the agent..." : "Load a model to start"}
        bind:value={input}
        onkeydown={onKeydown}
        rows="1"
        disabled={!status.loaded}
      ></textarea>
      <div class="composer-bar">
        <span class="hint-inline">
          <span class="kbd">↵</span> run · <span class="kbd">⇧↵</span> new line
        </span>
        {#if running}
          <button class="send stop" onclick={stop} title="Stop"><Icon name="stop" size="sm" /></button>
        {:else}
          <button class="send" onclick={send} disabled={!status.loaded || !input.trim()} title="Run">
            <Icon name="send" size="sm" />
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .agent-header {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-1);
  }
  .session-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-1);
  }
  .session-select {
    flex: 1;
    min-width: 0;
    font-size: 12px;
  }
  .agent-scroll {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-0);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 20px;
  }
  .empty-orb {
    position: relative;
    width: 140px;
    height: 140px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 22px;
  }
  .orb-halo {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: radial-gradient(circle, color-mix(in srgb, var(--accent) 12%, transparent) 0%, transparent 70%);
  }
  .empty-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-0);
  }
  .empty-sub {
    font-size: 13px;
    color: var(--text-2);
    margin-top: 7px;
    max-width: 360px;
  }
  .bubble {
    padding: 8px 12px;
    border-radius: 8px;
    white-space: pre-wrap;
    font-size: 13px;
    max-width: 90%;
  }
  .bubble.user {
    align-self: flex-end;
    background: var(--bg-2);
    color: var(--text-0);
  }
  .bubble.final {
    align-self: flex-start;
    background: var(--bg-1);
    border: 1px solid var(--border);
    color: var(--text-0);
  }
  .bubble.err {
    background: rgba(220, 60, 60, 0.12);
    border: 1px solid rgba(220, 60, 60, 0.4);
    color: var(--text-0);
  }
  .step {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-1);
    padding: 6px 10px;
  }
  .step.tool.err {
    border-color: rgba(220, 60, 60, 0.4);
  }
  .label {
    font-size: 11px;
    margin-bottom: 4px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .json,
  .result {
    margin: 0;
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-1);
    max-height: 220px;
    overflow-y: auto;
  }
  .reasoning {
    font-size: 11px;
    font-style: italic;
    color: var(--text-2);
    margin-bottom: 6px;
    white-space: pre-wrap;
  }
  .mode-pill {
    display: flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }
  .mode-pill button {
    background: var(--bg-1);
    border: none;
    border-radius: 0;
    padding: 3px 10px;
    font-size: 11px;
    color: var(--text-2);
  }
  .mode-pill button.active {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .permission {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    background: rgba(220, 160, 40, 0.1);
    border-top: 1px solid rgba(220, 160, 40, 0.4);
  }
  .perm-text {
    font-size: 12px;
    color: var(--text-0);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .perm-tag {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: #d9a028;
    margin-right: 6px;
  }
  .composer-wrap {
    padding: 12px 14px 14px;
  }
  .composer {
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    padding: 12px 14px 10px;
    transition: border-color var(--t-fast), box-shadow var(--t-fast);
  }
  .composer:focus-within {
    border-color: var(--accent-border);
    box-shadow: 0 0 0 3px var(--accent-bg);
  }
  .composer.disabled {
    opacity: 0.7;
  }
  .composer textarea {
    background: transparent;
    border: none;
    padding: 0;
    max-height: 200px;
    min-height: 24px;
    field-sizing: content;
  }
  .composer textarea:focus {
    box-shadow: none;
  }
  .composer-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 8px;
  }
  .hint-inline {
    font-size: 11px;
    color: var(--text-3);
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .send {
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent-2);
    flex: none;
  }
  .send:hover:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
  }
  .send:disabled {
    color: var(--text-3);
    border-color: var(--border);
    background: transparent;
  }
  .send.stop {
    border-color: color-mix(in srgb, var(--error) 50%, transparent);
    background: var(--error-bg);
    color: var(--error);
  }
  .send.stop:hover {
    background: var(--error);
    color: #fff;
    border-color: var(--error);
  }
  .small-btn {
    padding: 4px 11px;
    font-size: 11px;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
