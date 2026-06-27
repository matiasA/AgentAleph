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
  <div class="role">{isUser ? "You" : "Assistant"}</div>

  <div class="bubble">
    {#if reasoning}
      <div class="reasoning">
        <button class="reasoning-toggle" onclick={() => (showReasoning = !showReasoning)}>
          <span class="reasoning-icon" class:open={showReasoning}>▸</span>
          <span class="reasoning-label">
            {inReasoning ? "thinking..." : "reasoning"}
            {#if !inReasoning}
              <span class="dim">· {reasoning.length} chars</span>
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
</div>

<style>
  .msg {
    padding: 14px 18px;
    display: flex;
    flex-direction: column;
    animation: fade-in 0.18s var(--ease);
  }
  .msg.user {
    align-items: flex-end;
  }
  .role {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.6px;
    margin-bottom: 5px;
  }
  .bubble {
    max-width: 82%;
  }
  .msg.assistant .bubble {
    max-width: 100%;
  }
  .msg.user .bubble {
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-lg);
    padding: 10px 13px;
  }
  .body {
    color: var(--text-0);
    line-height: 1.65;
    white-space: pre-wrap;
    word-wrap: break-word;
    user-select: text;
  }
  .reasoning {
    margin-bottom: 8px;
    border-left: 2px solid var(--border-strong);
    padding-left: 10px;
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
    gap: 5px;
  }
  .reasoning-toggle:hover {
    background: transparent;
    color: var(--text-1);
  }
  .reasoning-icon {
    font-size: 10px;
    color: var(--text-3);
    transition: transform var(--t-fast);
  }
  .reasoning-icon.open {
    transform: rotate(90deg);
  }
  .reasoning-label {
    color: var(--text-2);
  }
  .reasoning-body {
    color: var(--text-2);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-wrap: break-word;
    margin-top: 6px;
    padding: 8px 10px;
    background: var(--bg-2);
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-sm);
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
