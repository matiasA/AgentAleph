<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../lib/api";
  import type { LoadProgress, LocalModel, ModelStatus } from "../lib/types";
  import Icon from "./Icon.svelte";
  import ModelFamilyBadge from "./ModelFamilyBadge.svelte";

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
  let confirmDelete = $state<LocalModel | null>(null);
  let deleting = $state(false);

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

  async function confirmRemove() {
    const m = confirmDelete;
    if (!m) return;
    deleting = true;
    try {
      await api.deleteModel(m.path);
      confirmDelete = null;
      await refresh();
      onRefresh();
    } catch (e: any) {
      alert("Error: " + String(e));
    } finally {
      deleting = false;
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
          <ModelFamilyBadge name={m.name} />
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
                <button class="menu-item danger" onclick={() => { menuOpen = null; confirmDelete = m; }}>
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

{#if confirmDelete}
  <div
    class="overlay"
    role="presentation"
    onclick={() => !deleting && (confirmDelete = null)}
    onkeydown={(e) => e.key === "Escape" && !deleting && (confirmDelete = null)}
  >
    <div
      class="confirm-box"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="row" style="gap:8px;align-items:flex-start">
        <span class="confirm-ico"><Icon name="alert" size="lg" /></span>
        <div class="col" style="gap:4px">
          <div class="confirm-title">Eliminar {confirmDelete.name}</div>
          <div class="dim small">
            Esto borra el archivo del disco ({confirmDelete.size_human}) de forma permanente. No
            hay papelera: si lo necesitas de nuevo, tendrás que volver a descargarlo.
          </div>
        </div>
      </div>
      <div class="row confirm-actions">
        <button class="ghost" disabled={deleting} onclick={() => (confirmDelete = null)}>
          Mantener archivo
        </button>
        <button class="danger-btn" disabled={deleting} onclick={confirmRemove}>
          {deleting ? "Eliminando…" : "Eliminar definitivamente"}
        </button>
      </div>
    </div>
  </div>
{/if}

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
    padding: 10px 11px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    margin-bottom: 6px;
    background: var(--bg-2);
    opacity: 0.6;
    transition: border-color var(--t-fast), background var(--t-fast), opacity var(--t-fast);
  }
  .local-row:hover {
    background: var(--bg-3);
    opacity: 1;
  }
  .local-row.active {
    border-color: var(--accent-border);
    background: var(--accent-bg);
    opacity: 1;
  }
  .local-row.active:hover {
    background: var(--accent-bg-strong);
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
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fade-in 0.12s var(--ease);
  }
  .confirm-box {
    width: 360px;
    background: var(--bg-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .confirm-ico {
    color: var(--error);
    flex-shrink: 0;
    margin-top: 1px;
  }
  .confirm-title {
    font-weight: 600;
    font-size: 13.5px;
    color: var(--text-0);
  }
  .confirm-actions {
    justify-content: flex-end;
    gap: 8px;
  }
  .danger-btn {
    border: 1px solid var(--error);
    color: var(--error);
    background: var(--error-bg);
    border-radius: var(--radius-sm);
    padding: 7px 12px;
    font-size: 12px;
    font-weight: 500;
  }
  .danger-btn:hover {
    background: var(--error);
    color: #fff;
  }
  .danger-btn:disabled,
  .confirm-actions .ghost:disabled {
    opacity: 0.6;
    pointer-events: none;
  }
</style>
