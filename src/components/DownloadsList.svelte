<script lang="ts">
  import type { DownloadState } from "../lib/types";
  import {
    api,
    humanSize,
    humanSpeed,
    isDone,
    isDownloading,
    statusError,
    statusLabel,
  } from "../lib/api";

  let {
    downloads,
  }: {
    downloads: DownloadState[];
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
