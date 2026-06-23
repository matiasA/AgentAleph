<script lang="ts">
  import type { AppInfo, LoadProgress, ModelStatus } from "../lib/types";

  let {
    status,
    info,
    pendingDownloads,
    loadProgress = null,
  }: {
    status: ModelStatus;
    info: AppInfo | null;
    pendingDownloads: number;
    loadProgress?: LoadProgress | null;
  } = $props();
</script>

<div class="statusbar">
  <span class="status-led {loadProgress ? 'loading' : status.loaded ? 'on' : 'off'}"></span>
  {#if loadProgress}
    <span class="loading-wrap">
      <span class="muted">cargando:</span>
      <span class="ldname" title={loadProgress.model_name}>{loadProgress.model_name}</span>
      <span class="progress-track">
        <span class="progress-fill" style="width:{loadProgress.percent}%"></span>
      </span>
      <span class="pct">{loadProgress.percent}%</span>
      <span class="dim phase">{loadProgress.phase}</span>
    </span>
  {:else}
    <span>
      {#if status.loaded}
        <span class="muted">model:</span> {status.model_name}
        <span class="dim"> · :{status.port}</span>
      {:else}
        <span class="dim">sin modelo cargado</span>
      {/if}
    </span>
  {/if}
  {#if pendingDownloads > 0}
    <span class="tag accent">↓ {pendingDownloads}</span>
  {/if}
  <span style="flex:1"></span>
  {#if info}
    <span class="dim">v{info.version}</span>
    <span class="dim">{info.os}/{info.arch}</span>
  {/if}
</div>

<style>
  .status-led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .status-led.on {
    background: var(--success);
    box-shadow: 0 0 6px var(--success);
  }
  .status-led.off {
    background: var(--text-3);
  }
  .status-led.loading {
    background: var(--accent);
    box-shadow: 0 0 6px var(--accent);
    animation: pulse 1s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
  .loading-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .ldname {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .progress-track {
    width: 120px;
    height: 5px;
    border-radius: 3px;
    background: var(--bg-2, rgba(255, 255, 255, 0.08));
    overflow: hidden;
    flex: none;
  }
  .progress-fill {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .pct {
    font-variant-numeric: tabular-nums;
    min-width: 34px;
  }
  .phase {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
