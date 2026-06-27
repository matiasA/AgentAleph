<script lang="ts">
  import Icon from "./Icon.svelte";
  import {
    ACCENTS,
    getAccent,
    applyAccent,
    getMode,
    applyMode,
    type AccentId,
    type Mode,
  } from "../lib/theme";

  let open = $state(false);
  let current = $state<AccentId>(getAccent());
  let mode = $state<Mode>(getMode());

  function pick(id: AccentId) {
    current = id;
    applyAccent(id);
  }

  function setMode(m: Mode) {
    mode = m;
    applyMode(m);
  }
</script>

<svelte:window onclick={() => (open = false)} />

<div class="wrap" role="presentation" onclick={(e) => e.stopPropagation()}>
  <button class="icon-btn" title="Change accent" onclick={() => (open = !open)}>
    <Icon name="palette" size="sm" />
  </button>

  {#if open}
    <div class="pop">
      <div class="pop-label">Modo</div>
      <div class="mode-toggle">
        <button class="mode-opt" class:active={mode === "dark"} onclick={() => setMode("dark")}>
          Oscuro
        </button>
        <button class="mode-opt" class:active={mode === "light"} onclick={() => setMode("light")}>
          Claro
        </button>
      </div>

      <div class="pop-label" style="margin-top:12px">Tono</div>
      <div class="swatches">
        {#each ACCENTS as a (a.id)}
          <button
            class="swatch"
            class:active={current === a.id}
            title={a.label}
            style="--sw:{a.swatch}"
            onclick={() => pick(a.id)}>
            <span class="dot"></span>
            <span class="sw-label">{a.label}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    margin-right: auto;
  }
  .pop {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 180px;
    background: var(--bg-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 10px;
    z-index: 50;
    animation: fade-in 0.14s var(--ease);
  }
  .pop-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 1.2px;
    color: var(--text-2);
    margin-bottom: 8px;
  }
  .mode-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    padding: 3px;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .mode-opt {
    border: none;
    background: transparent;
    padding: 6px 8px;
    font-size: 11.5px;
    color: var(--text-1);
    border-radius: 6px;
  }
  .mode-opt:hover {
    background: var(--bg-hover);
  }
  .mode-opt.active {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .swatches {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .swatch {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-1);
    color: var(--text-1);
    font-size: 11px;
  }
  .swatch:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
  }
  .swatch.active {
    border-color: var(--sw);
    color: var(--text-0);
  }
  .dot {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    background: var(--sw);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--sw) 22%, transparent);
    flex: none;
  }
  .sw-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
