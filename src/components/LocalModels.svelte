<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import type { LoadProgress, LocalModel, ModelStatus } from "../lib/types";

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
  <div class="row between" style="padding:8px 10px;border-bottom:1px solid var(--border)">
    <span class="small muted">Modelos locales</span>
    <div class="row" style="gap:6px">
      <button class="ghost small-btn" onclick={addFolder} title="Añadir carpeta de modelos">
        ➕ Carpeta
      </button>
      <button class="ghost" onclick={refresh} title="Refrescar">⟳</button>
    </div>
  </div>
  <div class="scroll" style="padding:8px 10px">
    {#if loading}
      <div class="muted small">Cargando...</div>
    {:else if local.length === 0}
      <div class="empty">
        <div class="dim">Sin modelos</div>
        <div class="dim small" style="margin-top:4px">
          Descarga uno en "Catálogo" o pulsa <strong>➕ Carpeta</strong> para añadir una
          carpeta con tus GGUF.
        </div>
      </div>
    {:else}
      {#each local as m (m.path)}
        <div class="local-row" class:active={isActive(m)}>
          <div class="col" style="flex:1;min-width:0">
            <div class="row" style="gap:6px">
              <span class="name">{m.name}</span>
              {#if isActive(m)}
                <span class="tag accent">ACTIVO</span>
              {/if}
            </div>
            <div class="dim small">{m.size_human}</div>
          </div>
          <div class="row" style="gap:4px">
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
                    : "..."}
                {:else}
                  Cargar
                {/if}
              </button>
            {/if}
            <button class="ghost small-btn danger" onclick={() => remove(m)} title="Eliminar">
              ✕
            </button>
          </div>
        </div>
      {/each}
    {/if}

    {#if dirs.length > 0}
      <div class="folders">
        <div class="dim small" style="margin-bottom:6px">Carpetas externas</div>
        {#each dirs as d (d)}
          <div class="folder-row">
            <span class="folder-path" title={d}>📁 {d}</span>
            <button class="ghost small-btn" onclick={() => removeFolder(d)} title="Dejar de escanear">
              ✕
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .empty {
    padding: 20px 10px;
    text-align: center;
  }
  .local-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 6px;
    background: var(--bg-2);
  }
  .local-row.active {
    border-color: var(--accent-dim);
    background: var(--accent-bg);
  }
  .local-row:hover {
    border-color: var(--text-3);
  }
  .name {
    font-weight: 500;
  }
  .small-btn {
    padding: 3px 8px;
    font-size: 11px;
  }
  .folders {
    margin-top: 14px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .folder-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 4px 0;
  }
  .folder-path {
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
