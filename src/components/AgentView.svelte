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
  import type { ChatMsg, ModelStatus, SessionMeta } from "../lib/types";
  import Icon from "./Icon.svelte";
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
      : [{ value: sid, label: "· sesión actual (sin guardar)" }]),
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
          if (modelIdx !== null && items[modelIdx]?.kind === "model") {
            (items[modelIdx] as any).streaming = false;
          }
          if (e.reason === "done") {
            items.push({ kind: "final", text: e.text });
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
      alert("Carga un modelo primero desde la pestaña 'Modelos'");
      return;
    }
    if (!workingDir) {
      alert("Elige primero la carpeta del proyecto");
      return;
    }
    input = "";
    items.push({ kind: "user", text });
    running = true;
    scrollToBottom();
    try {
      await api.agentSend(sid, workingDir, mode, text);
    } catch (e: any) {
      running = false;
      items.push({ kind: "error", text: String(e) });
    }
  }

  async function respond(approved: boolean) {
    if (!pending) return;
    const req = pending.request_id;
    pending = null;
    await api.respondPermission(req, approved);
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

  // ---------- Sesiones ----------

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
    if (!confirm("¿Eliminar esta sesión?")) return;
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

  /** Reconstruye el timeline visible a partir de la conversación guardada. */
  function reconstruct(messages: ChatMsg[]): Item[] {
    const out: Item[] = [];
    let lastTool = "?";
    let lastArgs = "";
    for (const m of messages) {
      if (m.role === "system") continue;
      if (m.role === "assistant") {
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
      } else if (m.role === "user") {
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
          // mensajes internos del harness: no se muestran
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
      <span class="small muted">Agente</span>
      {#if status.loaded}
        <span class="tag accent">{status.model_name}</span>
      {:else}
        <span class="tag warn">sin modelo</span>
      {/if}
    </div>
    <div class="row" style="gap:8px">
      <div class="mode-pill">
        <button class:active={mode === "build"} disabled={running} onclick={() => (mode = "build")}>build</button>
        <button class:active={mode === "plan"} disabled={running} onclick={() => (mode = "plan")}>plan</button>
      </div>
      <button class="ghost small-btn" onclick={pickDir} title={workingDir}>
        <Icon name="folder" size="sm" />
        {workingDir ? workingDir.split("/").pop() : "Elegir carpeta"}
      </button>
      <button class="ghost small-btn" onclick={clearAll} disabled={running || items.length === 0}>
        Limpiar
      </button>
    </div>
  </div>

  <div class="session-bar">
    <button class="ghost small-btn" onclick={newSession} disabled={running}>+ Nueva</button>
    <div class="session-select">
      <Select
        value={sid}
        options={sessionOptions}
        disabled={running}
        onChange={(v) => openSession(v)}
      />
    </div>
    {#if sessions.find((s) => s.id === sid)}
      <button class="ghost small-btn danger" onclick={() => deleteSession(sid)} disabled={running} title="Eliminar sesión">
        ✕
      </button>
    {/if}
  </div>

  <div class="agent-scroll" bind:this={scrollEl}>
    {#if items.length === 0}
      <div class="empty">
        <div class="empty-orb">
          <span class="orb-halo"></span>
          <span class="orb-core"><Icon name="agent" size="lg" /></span>
        </div>
        <div class="empty-title">Agente listo</div>
        <div class="empty-sub">
          Elige una carpeta de proyecto y describe una tarea. El agente usará herramientas
          paso a paso.
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
            <div class="label dim">decisión {it.streaming ? "·…" : ""}</div>
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
        <span class="perm-tag">permiso</span>
        El agente quiere: <strong>{pending.summary}</strong>
      </div>
      <div class="row" style="gap:8px">
        <button class="danger small-btn" onclick={() => respond(false)}>Rechazar</button>
        <button class="solid small-btn" onclick={() => respond(true)}>Permitir</button>
      </div>
    </div>
  {/if}

  <div class="composer-wrap">
    <div class="composer" class:disabled={!status.loaded}>
      <textarea
        placeholder={status.loaded ? "Describe una tarea para el agente…" : "Carga un modelo para empezar"}
        bind:value={input}
        onkeydown={onKeydown}
        rows="1"
        disabled={!status.loaded}
      ></textarea>
      <div class="composer-bar">
        <span class="hint-inline">
          <span class="kbd">↵</span> ejecutar · <span class="kbd">⇧↵</span> nueva línea
        </span>
        {#if running}
          <button class="send stop" onclick={stop} title="Detener"><Icon name="stop" size="sm" /></button>
        {:else}
          <button class="send" onclick={send} disabled={!status.loaded || !input.trim()} title="Ejecutar">
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
    width: 120px;
    height: 120px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 22px;
  }
  .orb-halo {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: radial-gradient(circle, var(--accent-bg) 0%, transparent 68%);
    -webkit-mask: radial-gradient(circle, transparent 38%, #000 39%);
    mask: radial-gradient(circle, transparent 38%, #000 39%);
  }
  .orb-core {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--bg-2);
    border: 1px solid var(--border);
    color: var(--accent-2);
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
