<script lang="ts">
  import { api, onChatToken, onChatDone } from "../lib/api";
  import type { ChatMsg, ModelStatus } from "../lib/types";
  import Message from "./Message.svelte";

  let {
    status,
    sessionId,
  }: {
    status: ModelStatus;
    sessionId: string;
  } = $props();

  type LocalMsg = { role: string; content: string; reasoning?: string; streaming?: boolean };

  let messages = $state<LocalMsg[]>([]);
  let input = $state("");
  let sending = $state(false);
  let scrollEl: HTMLDivElement | null = $state(null);
  let streamingIdx = $state<number | null>(null);
  let inReasoning = $state(false);

  let unlistenToken: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;

  $effect(() => {
    (async () => {
      unlistenToken = await onChatToken((e) => {
        if (e.session_id !== sessionId) return;
        if (streamingIdx === null) return;
        const m = messages[streamingIdx];
        if (!m) return;
        if (e.is_reasoning) {
          m.reasoning = (m.reasoning || "") + e.token;
          inReasoning = true;
        } else {
          m.content += e.token;
          inReasoning = false;
        }
        scrollToBottom();
      });
      unlistenDone = await onChatDone((e) => {
        if (e.session_id !== sessionId) return;
        sending = false;
        inReasoning = false;
        if (streamingIdx !== null && messages[streamingIdx]) {
          messages[streamingIdx].streaming = false;
          if (e.error && e.error !== "cancelled" && messages[streamingIdx].content === "") {
            messages[streamingIdx].content = `⚠ Error: ${e.error}`;
          }
        }
        streamingIdx = null;
        scrollToBottom();
      });
    })();
    return () => {
      unlistenToken?.();
      unlistenDone?.();
    };
  });

  function scrollToBottom() {
    queueMicrotask(() => {
      if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
    });
  }

  async function send() {
    const text = input.trim();
    if (!text || sending) return;
    if (!status.loaded) {
      alert("Carga un modelo primero desde la pestaña 'Modelos'");
      return;
    }
    input = "";
    messages.push({ role: "user", content: text });
    messages.push({ role: "assistant", content: "", streaming: true });
    streamingIdx = messages.length - 1;
    sending = true;
    scrollToBottom();

    const history: ChatMsg[] = messages
      .slice(0, -1)
      .map((m) => ({ role: m.role, content: m.content }));

    try {
      await api.sendChat(sessionId, history);
    } catch (e: any) {
      sending = false;
      if (streamingIdx !== null) {
        messages[streamingIdx].streaming = false;
        messages[streamingIdx].content = `⚠ Error: ${String(e)}`;
      }
      streamingIdx = null;
    }
  }

  async function stop() {
    await api.stopChat(sessionId);
  }

  function clearChat() {
    if (sending) return;
    messages = [];
    streamingIdx = null;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="col" style="flex:1;overflow:hidden">
  <div class="chat-header row between">
    <div class="row" style="gap:8px">
      <span class="small muted">Chat</span>
      {#if status.loaded}
        <span class="tag accent">{status.model_name}</span>
      {:else}
        <span class="tag warn">sin modelo</span>
      {/if}
    </div>
    <button class="ghost small-btn" onclick={clearChat} disabled={sending || messages.length === 0}>
      Limpiar
    </button>
  </div>

  <div class="chat-scroll" bind:this={scrollEl}>
    {#if messages.length === 0}
      <div class="empty">
        <div class="dim" style="font-size:14px">Sin mensajes</div>
        <div class="dim small" style="margin-top:6px">
          {#if status.loaded}
            Escribe abajo para chatear con <strong>{status.model_name}</strong>
          {:else}
            Carga un modelo desde la pestaña <strong>Modelos</strong>.
          {/if}
        </div>
      </div>
    {:else}
      {#each messages as m, i (i)}
        <Message role={m.role} content={m.content} reasoning={m.reasoning} streaming={m.streaming && i === streamingIdx} inReasoning={inReasoning && i === streamingIdx} />
      {/each}
    {/if}
  </div>

  <div class="composer">
    <textarea
      placeholder={status.loaded ? "Escribe un mensaje... (Enter para enviar, Shift+Enter para salto)" : "Carga un modelo para empezar a chatear"}
      bind:value={input}
      onkeydown={onKeydown}
      rows="2"
      disabled={!status.loaded}
    ></textarea>
    <div class="row between" style="margin-top:6px">
      <span class="dim small">↵ enviar · ⇧↵ salto de línea</span>
      {#if sending}
        <button class="danger" onclick={stop}>■ Detener</button>
      {:else}
        <button class="primary" onclick={send} disabled={!status.loaded || !input.trim()}>
          Enviar
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .chat-header {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-1);
  }
  .chat-scroll {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-0);
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
  .composer {
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-1);
  }
  .small-btn {
    padding: 3px 10px;
    font-size: 11px;
  }
</style>
