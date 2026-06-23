<script lang="ts">
  import { api, onDownloadProgress, onModelStatus } from "../lib/api";
  import type { DownloadState, LoadProgress, ModelStatus } from "../lib/types";
  import ModelBrowser from "./ModelBrowser.svelte";
  import LocalModels from "./LocalModels.svelte";
  import DownloadsList from "./DownloadsList.svelte";
  import SettingsView from "./SettingsView.svelte";

  let {
    status,
    downloads,
    loadProgress = null,
    onDownloadsChange,
    onStatusChange,
  }: {
    status: ModelStatus;
    downloads: DownloadState[];
    loadProgress?: LoadProgress | null;
    onDownloadsChange: () => void;
    onStatusChange: () => void;
  } = $props();

  type Tab = "models" | "catalog" | "downloads" | "settings";
  let tab = $state<Tab>("models");

  let localRefreshKey = $state(0);

  async function handleDownload(repo: string, filename: string, _displayName: string) {
    try {
      await api.download(repo, filename);
      onDownloadsChange();
      tab = "downloads";
    } catch (e: any) {
      alert("Error al iniciar descarga: " + String(e));
    }
  }

  function refreshLocal() {
    localRefreshKey += 1;
    onStatusChange();
  }
</script>

<div class="sidebar">
  <div class="brand">
    <div class="brand-name">AGENT ALEPH</div>
    <div class="brand-sub dim">agente · modelos locales</div>
  </div>

  <nav class="tabs">
    <button class="tab" class:active={tab === "models"} onclick={() => (tab = "models")}>
      Modelos
    </button>
    <button class="tab" class:active={tab === "catalog"} onclick={() => (tab = "catalog")}>
      Catálogo
    </button>
    <button class="tab" class:active={tab === "downloads"} onclick={() => (tab = "downloads")}>
      Descargas
      {#if downloads.filter((d) => d.status === "Downloading" || d.status === "Pending").length > 0}
        <span class="dot"></span>
      {/if}
    </button>
    <button class="tab" class:active={tab === "settings"} onclick={() => (tab = "settings")}>
      Ajustes
    </button>
  </nav>

  <div class="content">
    {#if tab === "models"}
      <LocalModels {status} {loadProgress} onRefresh={refreshLocal} />
    {:else if tab === "catalog"}
      <ModelBrowser onDownload={handleDownload} />
    {:else if tab === "downloads"}
      <DownloadsList {downloads} />
    {:else if tab === "settings"}
      <SettingsView onSaved={onStatusChange} />
    {/if}
  </div>
</div>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
  .brand {
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border);
  }
  .brand-name {
    font-weight: 700;
    letter-spacing: 1px;
    color: var(--text-0);
    font-size: 13px;
  }
  .brand-sub {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 1px;
  }
  .tabs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px;
    background: var(--border);
    border-bottom: 1px solid var(--border);
  }
  .tab {
    background: var(--bg-1);
    border: none;
    border-radius: 0;
    padding: 8px 6px;
    font-size: 11px;
    color: var(--text-2);
    position: relative;
  }
  .tab:hover {
    background: var(--bg-2);
    color: var(--text-0);
  }
  .tab.active {
    background: var(--bg-2);
    color: var(--accent);
    border-bottom: 2px solid var(--accent);
  }
  .dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
