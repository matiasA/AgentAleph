<script lang="ts">
  import { api, onChatToken, onChatDone } from "../lib/api";
  import type { ChatMsg, ModelStatus } from "../lib/types";
  import Message from "./Message.svelte";
  import Icon from "./Icon.svelte";

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
      alert("Load a model first from the Models tab");
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
  {#if messages.length > 0}
    <div class="chat-header row between">
      <div class="row" style="gap:8px">
        {#if status.loaded}
          <span class="tag accent">{status.model_name}</span>
        {:else}
          <span class="tag warn">no model</span>
        {/if}
      </div>
      <button class="ghost small-btn" onclick={clearChat} disabled={sending}>
        Clear
      </button>
    </div>
  {/if}

  <div class="chat-scroll" bind:this={scrollEl}>
    {#if messages.length === 0}
      <div class="empty">
        <div class="empty-orb">
          <span class="orb-halo"></span>
          <Icon name="chat" size="xxl" />
        </div>
        <div class="empty-title">Start a Conversation</div>
        <div class="empty-sub">
          {#if status.loaded}
            Write your first message to <strong>{status.model_name}</strong>.
          {:else}
            Select or load a model and write your first message.
          {/if}
        </div>
      </div>
    {:else}
      {#each messages as m, i (i)}
        <Message role={m.role} content={m.content} reasoning={m.reasoning} streaming={m.streaming && i === streamingIdx} inReasoning={inReasoning && i === streamingIdx} />
      {/each}
    {/if}
  </div>

  <div class="composer-wrap">
    <div class="composer" class:disabled={!status.loaded}>
      <textarea
        placeholder={status.loaded ? "Write your message..." : "Load a model to start chatting"}
        bind:value={input}
        onkeydown={onKeydown}
        rows="1"
        disabled={!status.loaded}
      ></textarea>
      <div class="composer-bar">
        <div class="tools">
          <button class="icon-btn" title="Attach context"><Icon name="plus" size="sm" /></button>
          <button class="icon-btn" title="Parameters"><Icon name="sliders" size="sm" /></button>
          <button class="icon-btn" title="Attach file"><Icon name="paperclip" size="sm" /></button>
        </div>
        {#if sending}
          <button class="send stop" onclick={stop} title="Stop"><Icon name="stop" size="sm" /></button>
        {:else}
          <button
            class="send"
            onclick={send}
            disabled={!status.loaded || !input.trim()}
            title="Send">
            <Icon name="send" size="sm" />
          </button>
        {/if}
      </div>
    </div>
    <div class="hint">
      <span><span class="kbd">↵</span> Enter to send</span>
      <span><span class="kbd">⇧ ↵</span> Enter for new line</span>
    </div>
  </div>
</div>

<style>
  .chat-header {
    padding: 10px 18px;
    border-bottom: 1px solid var(--border-soft);
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
    max-width: 340px;
  }
  .empty-sub strong {
    color: var(--text-1);
    font-weight: 600;
  }

  .composer-wrap {
    padding: 12px 18px 14px;
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
  .tools {
    display: flex;
    gap: 2px;
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
  .hint {
    display: flex;
    gap: 16px;
    justify-content: flex-end;
    margin-top: 9px;
    font-size: 11px;
    color: var(--text-3);
  }
  .hint span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .small-btn {
    padding: 4px 11px;
    font-size: 11px;
  }
</style>
