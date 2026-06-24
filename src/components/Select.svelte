<script lang="ts">
  import Icon from "./Icon.svelte";

  type Option = { value: string; label: string };

  let {
    value = $bindable(),
    options,
    placeholder = "Seleccionar…",
    disabled = false,
    onChange,
  }: {
    value: string;
    options: Option[];
    placeholder?: string;
    disabled?: boolean;
    onChange?: (v: string) => void;
  } = $props();

  let open = $state(false);
  let current = $derived(options.find((o) => o.value === value));

  function choose(v: string) {
    value = v;
    open = false;
    onChange?.(v);
  }

  function toggle() {
    if (!disabled) open = !open;
  }
</script>

<svelte:window onclick={() => (open = false)} />

<div class="sel" class:open class:disabled role="presentation" onclick={(e) => e.stopPropagation()}>
  <button class="sel-trigger" {disabled} onclick={toggle} type="button">
    <span class="sel-value" class:placeholder={!current}>
      {current ? current.label : placeholder}
    </span>
    <span class="sel-chev" class:up={open}><Icon name="chevron-down" size="sm" /></span>
  </button>

  {#if open}
    <div class="sel-menu">
      {#each options as o (o.value)}
        <button
          class="sel-opt"
          class:active={o.value === value}
          type="button"
          onclick={() => choose(o.value)}>
          <span class="sel-opt-label">{o.label}</span>
          {#if o.value === value}<span class="sel-check"><Icon name="check" size="sm" /></span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sel {
    position: relative;
    width: 100%;
    min-width: 0;
  }
  .sel-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    color: var(--text-0);
    font-weight: 400;
    text-align: left;
  }
  .sel-trigger:hover:not(:disabled) {
    background: var(--bg-input);
    border-color: var(--border-strong);
  }
  .sel.open .sel-trigger {
    border-color: var(--accent-border);
    box-shadow: 0 0 0 3px var(--accent-bg);
  }
  .sel-value {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-value.placeholder {
    color: var(--text-3);
  }
  .sel-chev {
    display: inline-flex;
    color: var(--text-2);
    transition: transform var(--t-fast);
    flex: none;
  }
  .sel-chev.up {
    transform: rotate(180deg);
  }

  .sel-menu {
    position: absolute;
    top: calc(100% + 5px);
    left: 0;
    right: 0;
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    z-index: 60;
    max-height: 280px;
    overflow-y: auto;
    animation: fade-in 0.12s var(--ease);
  }
  .sel-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    padding: 7px 9px;
    color: var(--text-1);
    text-align: left;
    font-weight: 400;
  }
  .sel-opt:hover {
    background: var(--bg-hover);
    color: var(--text-0);
  }
  .sel-opt.active {
    color: var(--accent-2);
    background: var(--accent-bg);
  }
  .sel-opt-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sel-check {
    display: inline-flex;
    color: var(--accent);
    flex: none;
  }
</style>
