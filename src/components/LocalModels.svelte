<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import type { LoadProgress, LocalModel, ModelStatus } from "../lib/types";
  import Icon from "./Icon.svelte";

  let {
    status,
    loadProgress = null,
    onRefresh,
  }: {
    status: ModelStatus;
    loadProgress?: LoadProgress | null;
    onRefresh: () => void;
  } = $props();

  let local = $state<LocalModel[]>([]);
  let dirs = $state<string[]>([]);
  let loading = $state(true);

  let loadingId = $state<string | null>(null);
  let menuOpen = $state<string | null>(null);

  async function refresh() {
    loading = true;
    try {
      local = await api.listLocal();
      dirs = await api.listModelDirs();
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    refresh();
  });

  async function addFolder() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir !== "string") return;
    try {
      dirs = await api.addModelDir(dir);
      await refresh();
    } catch (e: any) {
      alert("Error añadiendo carpeta: " + String(e));
    }
  }

  async function removeFolder(path: string) {
    try {
      dirs = await api.removeModelDir(path);
      await refresh();
    } catch (e: any) {
      alert("Error: " + String(e));
    }
  }

  async function load(m: LocalModel) {
    loadingId = m.path;
    try {
      await api.loadModel(m.path);
      onRefresh();
    } catch (e: any) {
      alert("Error cargando modelo: " + String(e));
    } finally {
      loadingId = null;
    }
  }

  async function remove(m: LocalModel) {
    if (!confirm(`¿Eliminar ${m.name}?\n(${m.size_human})`)) return;
    try {
      await api.deleteModel(m.path);
      await refresh();
      onRefresh();
    } catch (e: any) {
      alert("Error: " + String(e));
    }
  }

  function isActive(m: LocalModel): boolean {
    return status.model === m.path;
  }
</script>

<div class="col" style="flex:1;overflow:hidden">
  <div class="lm-head">
    <span class="section-label">Modelos locales</span>
    <button class="add-folder" onclick={addFolder} title="Añadir carpeta de modelos">
      <Icon name="folder-plus" size="sm" /> Carpeta
    </button>
  </div>
  <div class="scroll" style="padding:6px 10px 10px">
    {#if loading}
      <div class="muted small" style="padding:10px 4px">Cargando…</div>
    {:else if local.length === 0}
      <div class="empty">
        <span class="empty-ico"><Icon name="box" size="lg" /></span>
        <div class="muted">Sin modelos</div>
        <div class="dim small" style="margin-top:4px">
          Descarga uno en "Catálogo" o pulsa <strong>Carpeta</strong> para añadir tus GGUF.
        </div>
      </div>
    {:else}
      {#each local as m (m.path)}
        <div class="local-row" class:active={isActive(m)}>
          <div class="col" style="flex:1;min-width:0">
            <div class="row" style="gap:6px">
              <span class="name" title={m.name}>{m.name}</span>
              {#if isActive(m)}
                <span class="tag accent">activo</span>
              {/if}
            </div>
            <div class="dim small">{m.size_human}</div>
          </div>
          <div class="row" style="gap:4px;position:relative">
            {#if isActive(m)}
              <button class="ghost small-btn" onclick={() => api.unloadModel().then(onRefresh)}>
                Expulsar
              </button>
            {:else}
              <button
                class="primary small-btn"
                onclick={() => load(m)}
                disabled={loadingId === m.path}
              >
                {#if loadingId === m.path}
                  {loadProgress && loadProgress.model === m.path
                    ? `${loadProgress.percent}%`
                    : "…"}
                {:else}
                  Cargar
                {/if}
              </button>
            {/if}
            <button
              class="icon-btn"
              title="Más"
              onclick={() => (menuOpen = menuOpen === m.path ? null : m.path)}
            >
              <Icon name="dots" size="sm" />
            </button>
            {#if menuOpen === m.path}
              <div class="menu" role="presentation">
                <button class="menu-item danger" onclick={() => { menuOpen = null; remove(m); }}>
                  <Icon name="x" size="sm" /> Eliminar
                </button>
              </div>
            {/if}
          </div>
        </div>
      {/each}
    {/if}

    {#if dirs.length > 0}
      <div class="folders">
        <div class="section-label" style="margin-bottom:8px">Carpetas externas</div>
        {#each dirs as d (d)}
          <div class="folder-row">
            <Icon name="folder" size="sm" />
            <span class="folder-path" title={d}>{d}</span>
            <button class="icon-btn" onclick={() => removeFolder(d)} title="Dejar de escanear">
              <Icon name="x" size="sm" />
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .lm-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 6px;
  }
  .add-folder {
    border: none;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--text-1);
    border-radius: var(--radius-sm);
  }
  .add-folder:hover {
    background: var(--bg-hover);
    color: var(--text-0);
  }
  .empty {
    padding: 28px 14px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .empty-ico {
    color: var(--text-3);
    margin-bottom: 10px;
  }
  .local-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 7px;
    background: var(--bg-2);
    transition: border-color var(--t-fast), background var(--t-fast);
  }
  .local-row.active {
    border-color: var(--accent-border);
    background: var(--accent-bg);
  }
  .local-row:hover {
    border-color: var(--border-strong);
  }
  .name {
    font-weight: 500;
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small-btn {
    padding: 5px 11px;
    font-size: 11px;
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 130px;
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    z-index: 20;
    animation: fade-in 0.12s var(--ease);
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    justify-content: flex-start;
    border: none;
    background: transparent;
    padding: 7px 9px;
    font-size: 12px;
    color: var(--text-1);
    border-radius: var(--radius-sm);
  }
  .menu-item:hover {
    background: var(--bg-hover);
  }
  .menu-item.danger:hover {
    color: var(--error);
    background: var(--error-bg);
  }
  .folders {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border-soft);
  }
  .folder-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 2px;
    color: var(--text-2);
  }
  .folder-path {
    flex: 1;
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
