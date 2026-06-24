<script lang="ts">
  import { api } from "../lib/api";
  import type { DownloadState, LoadProgress, ModelStatus } from "../lib/types";
  import ModelBrowser from "./ModelBrowser.svelte";
  import LocalModels from "./LocalModels.svelte";
  import DownloadsList from "./DownloadsList.svelte";
  import SettingsView from "./SettingsView.svelte";
  import Icon from "./Icon.svelte";
  import ThemePicker from "./ThemePicker.svelte";
  import Logo from "./Logo.svelte";

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

  const nav: { id: Tab; label: string; icon: string }[] = [
    { id: "models", label: "Modelos", icon: "box" },
    { id: "catalog", label: "Catálogo", icon: "catalog" },
    { id: "downloads", label: "Descargas", icon: "download" },
    { id: "settings", label: "Ajustes", icon: "settings" },
  ];

  let pendingCount = $derived(
    downloads.filter((d) => d.status === "Downloading" || d.status === "Pending").length
  );

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
    <Logo size={40} />
    <div class="brand-text">
      <div class="brand-name">Agent Aleph</div>
      <div class="brand-sub">agente · modelos locales</div>
    </div>
  </div>

  <nav class="nav">
    {#each nav as item (item.id)}
      <button class="nav-item" class:active={tab === item.id} onclick={() => (tab = item.id)}>
        <Icon name={item.icon} />
        <span class="nav-label">{item.label}</span>
        {#if item.id === "downloads" && pendingCount > 0}
          <span class="badge">{pendingCount}</span>
        {/if}
      </button>
    {/each}
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

  <div class="foot">
    <ThemePicker />
    <button class="icon-btn" title="Refrescar" onclick={refreshLocal}>
      <Icon name="refresh" size="sm" />
    </button>
    <button class="icon-btn" title="Vista de cuadrícula">
      <Icon name="grid" size="sm" />
    </button>
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
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 16px 16px 14px;
  }
  .brand-name {
    font-size: 15px;
    font-weight: 700;
    letter-spacing: -0.2px;
    color: var(--text-0);
  }
  .brand-sub {
    font-size: 10.5px;
    color: var(--text-2);
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px 10px 10px;
    border-bottom: 1px solid var(--border-soft);
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 11px;
    width: 100%;
    justify-content: flex-start;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 9px 11px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-1);
  }
  .nav-item :global(svg.ico) {
    color: var(--text-2);
    transition: color var(--t-fast);
  }
  .nav-item:hover {
    background: var(--bg-hover);
    border-color: transparent;
    color: var(--text-0);
  }
  .nav-item:hover :global(svg.ico) {
    color: var(--text-1);
  }
  .nav-item.active {
    background: var(--accent-bg);
    color: var(--accent-2);
  }
  .nav-item.active :global(svg.ico) {
    color: var(--accent);
  }
  .nav-label {
    flex: 1;
    text-align: left;
  }
  .badge {
    font-size: 10px;
    font-weight: 700;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-contrast);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--border-soft);
  }
</style>
