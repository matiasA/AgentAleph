<script lang="ts">
  import { onMount } from "svelte";
  import { api, onDownloadProgress, onModelStatus, onModelLoading } from "./lib/api";
  import type { AppInfo, DownloadState, LoadProgress, ModelStatus } from "./lib/types";
  import Sidebar from "./components/Sidebar.svelte";
  import ChatView from "./components/ChatView.svelte";
  import AgentView from "./components/AgentView.svelte";
  import AgentPanel from "./components/AgentPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import Icon from "./components/Icon.svelte";

  let mode = $state<"chat" | "agent">("chat");

  let status = $state<ModelStatus>({
    loaded: false,
    model: null,
    model_name: null,
    port: 0,
  });
  let downloads = $state<DownloadState[]>([]);
  let loadProgress = $state<LoadProgress | null>(null);
  let info = $state<AppInfo | null>(null);
  let sessionId = $state(crypto.randomUUID());

  let pendingDownloads = $derived(
    downloads.filter((d) => d.status === "Downloading" || d.status === "Pending").length
  );

  onMount(async () => {
    try {
      status = await api.modelStatus();
      info = await api.getAppInfo();
    } catch (e) {
      console.error("init error", e);
    }

    onDownloadProgress((d) => {
      const idx = downloads.findIndex((x) => x.id === d.id);
      if (idx >= 0) {
        downloads[idx] = d;
      } else {
        downloads.push(d);
      }
      downloads = downloads;
    });

    onModelStatus((s) => {
      status = s;
      if (s.loaded) loadProgress = null;
    });

    onModelLoading((p) => {
      if (p.error) {
        // El fallo lo reporta quien inició la carga (alert); aquí sólo limpiamos.
        loadProgress = null;
        return;
      }
      loadProgress = p;
      if (p.percent >= 100) {
        const done = p;
        setTimeout(() => {
          if (loadProgress === done) loadProgress = null;
        }, 600);
      }
    });
  });

  async function refreshStatus() {
    try {
      status = await api.modelStatus();
    } catch (e) {
      console.error(e);
    }
  }

  function refreshDownloads() {
    // Las descargas llegan via evento, no hace nada extra; solo re-trigger
    downloads = downloads;
  }
</script>

<div class="app">
  <aside class="app__sidebar">
    <Sidebar
      {status}
      {downloads}
      {loadProgress}
      onDownloadsChange={refreshDownloads}
      onStatusChange={refreshStatus}
    />
  </aside>

  <main class="app__main">
    <div class="mode-switch">
      <div class="mode-tabs">
        <button class="mode-tab" class:active={mode === "chat"} onclick={() => (mode = "chat")}>
          <Icon name="chat" size="sm" /> Chat
        </button>
        <button class="mode-tab" class:active={mode === "agent"} onclick={() => (mode = "agent")}>
          <Icon name="agent" size="sm" /> Agente
        </button>
      </div>
    </div>
    {#if mode === "chat"}
      <ChatView {status} {sessionId} />
    {:else}
      <AgentView {status} sessionId={`agent-${sessionId}`} />
    {/if}
  </main>

  <aside class="app__panel">
    <AgentPanel {mode} />
  </aside>

  <footer class="app__statusbar">
    <StatusBar {status} {info} {pendingDownloads} {loadProgress} />
  </footer>
</div>

<style>
  .mode-switch {
    display: flex;
    justify-content: center;
    background: var(--bg-1);
    border-bottom: 1px solid var(--border);
    padding: 0 14px;
  }
  .mode-tabs {
    display: flex;
    gap: 4px;
  }
  .mode-tab {
    background: transparent;
    border: none;
    border-radius: 0;
    padding: 13px 22px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-2);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .mode-tab:hover {
    background: transparent;
    color: var(--text-0);
  }
  .mode-tab.active {
    color: var(--accent-2);
    border-bottom-color: var(--accent);
  }
  .mode-tab.active :global(svg.ico) {
    color: var(--accent);
  }
</style>
