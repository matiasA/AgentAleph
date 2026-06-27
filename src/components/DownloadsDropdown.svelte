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
  import Icon from "./Icon.svelte";

  let {
    downloads,
    onClose,
  }: {
    downloads: DownloadState[];
    onClose: () => void;
  } = $props();

  let clearedIds = $state<Set<string>>(new Set());
  let filterText = $state("");

  let ongoing = $derived(
    downloads.filter((d) => isDownloading(d.status))
  );

  let completed = $derived(
    downloads.filter((d) => isDone(d.status) && !clearedIds.has(d.id))
  );

  let filtered = $derived({
    ongoing: ongoing.filter((d) => matchesFilter(d)),
    completed: completed.filter((d) => matchesFilter(d)),
  });

  function matchesFilter(d: DownloadState): boolean {
    if (!filterText) return true;
    const q = filterText.toLowerCase();
    return (
      d.filename.toLowerCase().includes(q) ||
      d.repo.toLowerCase().includes(q)
    );
  }

  function pct(d: DownloadState): number {
    if (d.total === 0) return 0;
    return Math.min(100, (d.downloaded / d.total) * 100);
  }

  function eta(d: DownloadState): string {
    if (d.speed_bps <= 0 || d.total === 0) return "";
    const remaining = d.total - d.downloaded;
    const secs = Math.ceil(remaining / d.speed_bps);
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.ceil(secs / 60)} mins`;
    return `${(secs / 3600).toFixed(1)} h`;
  }

  function shortName(d: DownloadState): string {
    return d.filename.split("/").pop() ?? d.filename;
  }

  async function cancel(id: string) {
    await api.cancelDownload(id);
  }

  function clearCompleted() {
    const next = new Set(clearedIds);
    for (const d of completed) next.add(d.id);
    clearedIds = next;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}></div>

<div class="panel" role="dialog" aria-label="Downloads">
  <div class="header">
    <span class="title">Downloads</span>
    <button class="close-btn" onclick={onClose} aria-label="Close">
      <Icon name="x" size="sm" />
    </button>
  </div>

  <div class="filter-wrap">
    <input
      class="filter-input"
      placeholder="Filter downloads..."
      bind:value={filterText}
      type="search"
    />
  </div>

  <div class="body">
    {#if filtered.ongoing.length > 0}
      <div class="section-label">Active</div>
      {#each filtered.ongoing as d (d.id)}
        <div class="dl-item">
          <div class="dl-row">
            <span class="dl-icon"><Icon name="download" size="sm" /></span>
            <span class="dl-name" title={shortName(d)}>{shortName(d)}</span>
            <button class="icon-mini" title="Cancel" onclick={() => cancel(d.id)} aria-label="Cancel download">
              <Icon name="x" size="sm" />
            </button>
          </div>
          <div class="bar">
            <div class="bar-fill" style="width:{pct(d)}%"></div>
          </div>
          <div class="dl-meta">
            <span>{humanSize(d.downloaded)} of {humanSize(d.total)}</span>
            <span>
              {#if d.speed_bps > 0}
                {humanSpeed(d.speed_bps)} · {eta(d)} left
              {:else}
                {statusLabel(d.status)}
              {/if}
            </span>
          </div>
        </div>
      {/each}
    {/if}

    {#if filtered.completed.length > 0}
      <div class="section-label between">
        <span>Completed</span>
        <button class="link-btn" onclick={clearCompleted}>Clear</button>
      </div>
      {#each filtered.completed as d (d.id)}
        {@const err = statusError(d.status)}
        <div class="dl-item done">
          <div class="dl-row">
            <span class="dl-icon muted"><Icon name="download" size="sm" /></span>
            <span class="dl-name" title={shortName(d)}>{shortName(d)}</span>
            <span class="status-chip" class:success={d.status === "Completed"} class:error={!!err} class:warn={d.status === "Cancelled"}>
              {statusLabel(d.status)}
            </span>
          </div>
          {#if err}
            <div class="err-text">{err}</div>
          {/if}
          <div class="dl-meta">
            <span class="muted">{d.repo}</span>
            {#if d.total > 0}
              <span class="muted">{humanSize(d.total)}</span>
            {/if}
          </div>
        </div>
      {/each}
    {/if}

    {#if filtered.ongoing.length === 0 && filtered.completed.length === 0}
      <div class="empty">No downloads{filterText ? " match" : ""}</div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 49;
  }

  .panel {
    position: fixed;
    top: 40px;
    right: 40px;
    width: 420px;
    max-height: 540px;
    background: var(--bg-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 8px 32px rgba(0,0,0,0.35);
    display: flex;
    flex-direction: column;
    z-index: 50;
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-0);
  }
  .close-btn {
    background: transparent;
    border: none;
    padding: 3px;
    border-radius: var(--radius-sm);
    color: var(--text-2);
    display: inline-flex;
  }
  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-0);
  }

  .filter-wrap {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .filter-input {
    width: 100%;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 5px 10px;
    font-size: 12px;
    color: var(--text-0);
    box-sizing: border-box;
  }
  .filter-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
    padding: 6px 0 2px;
  }
  .section-label.between {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .link-btn {
    background: transparent;
    border: none;
    padding: 0;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
    cursor: pointer;
    text-transform: none;
    letter-spacing: normal;
  }
  .link-btn:hover {
    color: var(--accent-2);
  }

  .dl-item {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 9px 10px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .dl-item.done {
    opacity: 0.8;
  }

  .dl-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .dl-name {
    flex: 1;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-mini {
    background: transparent;
    border: none;
    padding: 2px;
    border-radius: var(--radius-sm);
    color: var(--text-3);
    display: inline-flex;
    flex: none;
    cursor: pointer;
  }
  .icon-mini:hover {
    background: var(--bg-hover);
    color: var(--error, #e24b4a);
  }

  .bar {
    height: 3px;
    background: var(--bg-3);
    border-radius: 2px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s;
  }

  .dl-meta {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-2);
  }

  .status-chip {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    background: var(--bg-3);
    color: var(--text-2);
    flex: none;
  }
  .status-chip.success {
    background: color-mix(in srgb, var(--success, #3fb950) 15%, transparent);
    color: var(--success, #3fb950);
  }
  .status-chip.error {
    background: color-mix(in srgb, var(--error, #e24b4a) 15%, transparent);
    color: var(--error, #e24b4a);
  }
  .status-chip.warn {
    background: color-mix(in srgb, var(--warn, #d29922) 15%, transparent);
    color: var(--warn, #d29922);
  }

  .err-text {
    font-size: 11px;
    color: var(--error, #e24b4a);
    word-break: break-all;
  }

  .muted {
    color: var(--text-3);
  }

  .empty {
    padding: 24px 0;
    text-align: center;
    font-size: 12px;
    color: var(--text-3);
  }

  .dl-icon {
    display: inline-flex;
    color: var(--text-2);
    flex: none;
  }
  .dl-icon.muted {
    color: var(--text-3);
  }
</style>
