<script lang="ts">
  // Custom title bar replacing the native OS chrome. Zed-like: a slim 34px bar
  // with navigation, ephemeral activity, and window controls.
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Icon from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import DownloadsDropdown from "./DownloadsDropdown.svelte";
  import type { DownloadState, LoadProgress, ModelStatus } from "../lib/types";

  let {
    status,
    loadProgress,
    pendingDownloads,
    downloads = [],
    onToggleSidebar,
  }: {
    status: ModelStatus;
    loadProgress: LoadProgress | null;
    pendingDownloads: number;
    downloads?: DownloadState[];
    onToggleSidebar: () => void;
  } = $props();

  let downloadsOpen = $state(false);

  const win = getCurrentWindow();
  let maximized = $state(false);

  // Ephemeral text for background work such as loading and downloads.
  let activity = $derived(
    loadProgress
      ? `Loading ${loadProgress.model_name || "model"}... ${Math.round(loadProgress.percent)}%`
      : pendingDownloads > 0
        ? `Downloading ${pendingDownloads} model${pendingDownloads > 1 ? "s" : ""}...`
        : null
  );

  onMount(() => {
    let unlisten: (() => void) | undefined;
    win.isMaximized().then((m) => (maximized = m));
    win.onResized(() => win.isMaximized().then((m) => (maximized = m))).then((u) => (unlisten = u));
    return () => unlisten?.();
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <!-- Left navigation area -->
  <div class="zone left">
    <button class="tb-icon" title="Menu" onclick={onToggleSidebar} aria-label="Toggle sidebar">
      <Icon name="menu" size="sm" />
    </button>
    {#if status.loaded && status.model_name}
      <span class="project" title={status.model_name}>{status.model_name}</span>
    {:else}
      <span class="project dim">No model loaded</span>
    {/if}
  </div>

  <!-- Flexible drag region -->
  <div class="zone fill" data-tauri-drag-region></div>

  <!-- Right activity area -->
  {#if activity}
    <div class="zone activity">
      <Icon name="download" size="sm" />
      <span class="activity-text">{activity}</span>
    </div>
  {/if}

  <!-- Downloads button with badge -->
  <div class="zone">
    <button
      class="dl-btn"
      class:active={downloadsOpen}
      class:pulsing={pendingDownloads > 0}
      title="Downloads"
      aria-label="Open downloads panel"
      onclick={() => (downloadsOpen = !downloadsOpen)}
    >
      <Logo size={24} />
      {#if pendingDownloads > 0}
        <span class="dl-badge">{pendingDownloads}</span>
      {/if}
    </button>
  </div>

  <!-- Window controls -->
  <div class="zone controls">
    <button class="winctl" title="Minimize" aria-label="Minimize" onclick={() => win.minimize()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
    </button>
    <button
      class="winctl"
      title={maximized ? "Restore" : "Maximize"}
      aria-label={maximized ? "Restore" : "Maximize"}
      onclick={() => win.toggleMaximize()}>
      {#if maximized}
        <svg viewBox="0 0 10 10" aria-hidden="true">
          <rect x="1" y="2.5" width="6" height="6" rx="0.5" />
          <path d="M3.5 2.5V1.5h5v5h-1" />
        </svg>
      {:else}
        <svg viewBox="0 0 10 10" aria-hidden="true"><rect x="1.5" y="1.5" width="7" height="7" rx="0.5" /></svg>
      {/if}
    </button>
    <button class="winctl close" title="Close" aria-label="Close" onclick={() => win.close()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
    </button>
  </div>
</header>

{#if downloadsOpen}
  <DownloadsDropdown {downloads} onClose={() => (downloadsOpen = false)} />
{/if}

<style>
  .titlebar {
    height: 34px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 8px;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    color: var(--text-0);
    user-select: none;
  }

  .zone {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .zone.fill {
    flex: 1;
    height: 100%;
  }

  /* — navegación izquierda — */
  .project {
    font-size: 12px;
    color: var(--text-1);
    font-weight: 500;
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-left: 2px;
  }
  .project.dim {
    color: var(--text-3);
    font-weight: 400;
  }

  /* icono de acción (hamburguesa) — monocromático, hover ilumina */
  .tb-icon {
    background: transparent;
    border: 1px solid transparent;
    padding: 4px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    display: inline-flex;
  }
  .tb-icon:hover {
    background: var(--bg-hover);
    border-color: transparent;
    color: var(--icon-active);
  }

  /* — botón de descargas — */
  .dl-btn {
    position: relative;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 3px;
    display: inline-flex;
    cursor: pointer;
    transition: background var(--t-fast);
  }
  .dl-btn:hover {
    background: var(--bg-hover);
  }
  .dl-btn.active {
    background: var(--accent-bg);
  }
  .dl-badge {
    position: absolute;
    top: -3px;
    right: -3px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 9px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  @keyframes pulse-badge {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }
  .dl-btn.pulsing .dl-badge {
    animation: pulse-badge 1.5s ease-in-out infinite;
  }
  @keyframes logo-glow-pulse {
    0%, 100% { filter: brightness(1); }
    50% { filter: brightness(1.35) drop-shadow(0 0 6px var(--accent-glow, #3dcfcf88)); }
  }
  .dl-btn.pulsing :global(.logo) {
    animation: logo-glow-pulse 1.5s ease-in-out infinite;
  }

  /* — estado efímero — */
  .activity {
    color: var(--text-2);
    font-size: 12px;
    gap: 6px;
  }
  .activity :global(svg.ico) {
    color: var(--text-2);
    width: 13px;
    height: 13px;
  }
  .activity-text {
    white-space: nowrap;
  }

  /* — controles de ventana — */
  .controls {
    gap: 2px;
    margin-left: 4px;
  }
  .winctl {
    width: 28px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--icon-default);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background var(--t-fast), color var(--t-fast);
  }
  .winctl svg {
    width: 11px;
    height: 11px;
    stroke: currentColor;
    stroke-width: 1.1;
    stroke-linecap: round;
    stroke-linejoin: round;
    fill: none;
  }
  .winctl:hover {
    background: var(--bg-active);
    color: var(--text-0);
    border-color: transparent;
  }
  .winctl.close:hover {
    background: #e24b4a;
    color: #fff;
  }
</style>
