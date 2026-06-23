<script lang="ts">
  let {
    role,
    content,
    reasoning = "",
    streaming = false,
    inReasoning = false,
  }: {
    role: string;
    content: string;
    reasoning?: string;
    streaming?: boolean;
    inReasoning?: boolean;
  } = $props();

  let isUser = $derived(role === "user");
  let showReasoning = $state(false);
</script>

<div class="msg" class:user={isUser} class:assistant={!isUser}>
  <div class="role">
    {isUser ? "you" : "assistant"}
  </div>

  {#if reasoning}
    <div class="reasoning">
      <button class="reasoning-toggle" onclick={() => (showReasoning = !showReasoning)}>
        <span class="reasoning-icon">{showReasoning ? "▼" : "▶"}</span>
        <span class="reasoning-label">
          {inReasoning ? "pensando..." : "pensamiento"}
          {#if !inReasoning}
            <span class="dim">({reasoning.length} chars)</span>
          {/if}
        </span>
        {#if inReasoning}<span class="cursor">▋</span>{/if}
      </button>
      {#if showReasoning || inReasoning}
        <div class="reasoning-body">{reasoning}</div>
      {/if}
    </div>
  {/if}

  <div class="body">
    {content}
    {#if streaming && !inReasoning}<span class="cursor">▋</span>{/if}
  </div>
</div>

<style>
  .msg {
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-soft);
  }
  .role {
    font-size: 10px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }
  .body {
    color: var(--text-0);
    line-height: 1.6;
    white-space: pre-wrap;
    word-wrap: break-word;
    user-select: text;
  }
  .user {
    background: var(--bg-1);
  }
  .reasoning {
    margin-bottom: 8px;
    border-left: 2px solid var(--border);
    padding-left: 8px;
  }
  .reasoning-toggle {
    background: transparent;
    border: none;
    padding: 2px 0;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-2);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .reasoning-toggle:hover {
    color: var(--text-1);
  }
  .reasoning-icon {
    font-size: 8px;
    color: var(--text-3);
  }
  .reasoning-label {
    color: var(--text-2);
  }
  .reasoning-body {
    color: var(--text-2);
    font-size: 11px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
    margin-top: 4px;
    padding: 6px 8px;
    background: var(--bg-2);
    border-radius: var(--radius);
    user-select: text;
    max-height: 300px;
    overflow-y: auto;
  }
  .cursor {
    display: inline-block;
    color: var(--accent);
    animation: blink 1s steps(2) infinite;
    margin-left: 1px;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
</style>
