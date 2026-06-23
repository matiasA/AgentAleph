<script lang="ts">
  import { onMount } from "svelte";
  import { api, onDownloadProgress, onModelStatus, onModelLoading } from "./lib/api";
  import type { AppInfo, DownloadState, LoadProgress, ModelStatus } from "./lib/types";
  import Sidebar from "./components/Sidebar.svelte";
  import ChatView from "./components/ChatView.svelte";
  import AgentView from "./components/AgentView.svelte";
  import StatusBar from "./components/StatusBar.svelte";

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
      <button class:active={mode === "chat"} onclick={() => (mode = "chat")}>Chat</button>
      <button class:active={mode === "agent"} onclick={() => (mode = "agent")}>Agente</button>
    </div>
    {#if mode === "chat"}
      <ChatView {status} {sessionId} />
    {:else}
      <AgentView {status} sessionId={`agent-${sessionId}`} />
    {/if}
  </main>

  <footer class="app__statusbar">
    <StatusBar {status} {info} {pendingDownloads} {loadProgress} />
  </footer>
</div>

<style>
  .mode-switch {
    display: flex;
    gap: 1px;
    background: var(--border);
    border-bottom: 1px solid var(--border);
  }
  .mode-switch button {
    flex: 1;
    background: var(--bg-1);
    border: none;
    border-radius: 0;
    padding: 7px 6px;
    font-size: 11px;
    color: var(--text-2);
  }
  .mode-switch button:hover {
    background: var(--bg-2);
    color: var(--text-0);
  }
  .mode-switch button.active {
    background: var(--bg-2);
    color: var(--accent);
    border-bottom: 2px solid var(--accent);
  }
</style>
