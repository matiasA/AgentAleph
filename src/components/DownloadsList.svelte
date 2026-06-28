<script lang="ts">
  import type { DownloadState, UpdateStatus } from "../lib/types";
  import {
    api,
    humanSize,
    humanSpeed,
    isDone,
    isDownloading,
    statusError,
    statusLabel,
  } from "../lib/api";
  import { installAndRestart } from "../lib/updater";
  import { open } from "@tauri-apps/plugin-shell";

  let {
    downloads,
    updateStatus = null,
  }: {
    downloads: DownloadState[];
    updateStatus?: UpdateStatus | null;
  } = $props();

  function pct(d: DownloadState): number {
    if (d.total === 0) return 0;
    return Math.min(100, (d.downloaded / d.total) * 100);
  }

  async function cancel(id: string) {
    await api.cancelDownload(id);
  }
</script>

<div class="col" style="flex:1;overflow:hidden">
  <div class="row between" style="padding:8px 10px;border-bottom:1px solid var(--border)">
    <span class="small muted">Downloads</span>
    <span class="dim small">{downloads.length}</span>
  </div>
  {#if updateStatus && updateStatus.phase !== "idle" && updateStatus.phase !== "checking"}
    <div class="update-banner">
      <div class="update-info">
        <span class="update-title">
          {#if updateStatus.phase === "downloading"}
            Downloading update v{updateStatus.version}…
          {:else if updateStatus.phase === "ready"}
            Update v{updateStatus.version} ready to install
          {:else if updateStatus.phase === "available"}
            New version v{updateStatus.version} available
          {:else if updateStatus.phase === "error"}
            Update error
          {/if}
        </span>
        {#if updateStatus.phase === "downloading"}
          <div class="bar update-bar">
            <div class="fill" style="width:{updateStatus.percent}%"></div>
          </div>
        {:else if updateStatus.notes}
          <span class="update-notes">{updateStatus.notes}</span>
        {/if}
        {#if updateStatus.phase === "error" && updateStatus.error}
          <span class="update-error">{updateStatus.error}</span>
        {/if}
      </div>
      {#if updateStatus.phase === "ready"}
        <button class="update-btn" onclick={installAndRestart}>
          Install &amp; restart
        </button>
      {:else if updateStatus.phase === "available" && !updateStatus.isAppImage}
        <button
          class="update-btn secondary"
          onclick={() => open("https://github.com/matiasA/AgentAleph/releases/latest")}
        >
          Download
        </button>
      {/if}
    </div>
  {/if}

  <div class="scroll" style="padding:8px 10px">
    {#if downloads.length === 0}
      <div class="empty dim small">No downloads</div>
    {:else}
      {#each [...downloads].reverse() as d (d.id)}
        <div class="dl">
          <div class="row between">
            <div class="col" style="flex:1;min-width:0">
              <div class="name">{d.filename.split("/").pop()}</div>
              <div class="dim small">{d.repo}</div>
            </div>
            <span class="tag {statusLabel(d.status) === 'Completed' ? 'success' : statusLabel(d.status) === 'Failed' ? 'error' : statusLabel(d.status) === 'Cancelled' ? 'warn' : ''}">
              {statusLabel(d.status)}
            </span>
          </div>
          {#if isDownloading(d.status)}
            <div class="bar"><div class="fill" style="width:{pct(d)}%"></div></div>
            <div class="row between small dim" style="margin-top:4px">
              <span>{humanSize(d.downloaded)} / {humanSize(d.total)}</span>
              <span>{humanSpeed(d.speed_bps)} · {pct(d).toFixed(0)}%</span>
            </div>
            <div class="row" style="justify-content:flex-end;margin-top:4px">
              <button class="ghost small-btn" onclick={() => cancel(d.id)}>Cancel</button>
            </div>
          {:else if statusError(d.status)}
            <div class="small" style="color:var(--error);margin-top:4px">
              {statusError(d.status)}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .update-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 8px 10px 4px;
    padding: 10px 12px;
    background: var(--accent-bg);
    border: 1px solid var(--accent-border, color-mix(in srgb, var(--accent) 30%, transparent));
    border-radius: var(--radius-sm);
    flex: none;
  }
  .update-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .update-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-2);
  }
  .update-notes {
    font-size: 11px;
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .update-error {
    font-size: 11px;
    color: var(--error);
  }
  .update-bar {
    margin-top: 2px;
  }
  .update-btn {
    flex: none;
    padding: 5px 12px;
    font-size: 11px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-contrast);
    white-space: nowrap;
    align-self: center;
  }
  .update-btn:hover {
    background: var(--accent-2);
    border-color: var(--accent-2);
  }
  .update-btn.secondary {
    background: transparent;
    border-color: var(--accent);
    color: var(--accent-2);
  }
  .update-btn.secondary:hover {
    background: var(--accent-bg);
  }

  .empty {
    padding: 14px 10px;
    text-align: center;
  }
  .dl {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px;
    margin-bottom: 8px;
  }
  .name {
    font-weight: 500;
  }
  .bar {
    margin-top: 8px;
    height: 4px;
    background: var(--bg-3);
    border-radius: 2px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s;
  }
  .small-btn {
    padding: 3px 8px;
    font-size: 11px;
  }
</style>
