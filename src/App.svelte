<script lang="ts">
  import { onMount } from "svelte";
  import { api, onDownloadProgress, onModelStatus, onModelLoading } from "./lib/api";
  import type { AppInfo, DownloadState, LoadProgress, ModelStatus, UpdateStatus } from "./lib/types";
  import { startUpdateChecker } from "./lib/updater";
  import Sidebar from "./components/Sidebar.svelte";
  import ChatView from "./components/ChatView.svelte";
  import AgentView from "./components/AgentView.svelte";
  import AgentPanel from "./components/AgentPanel.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import Icon from "./components/Icon.svelte";

  let mode = $state<"chat" | "agent">("chat");
  let sidebarOpen = $state(true);

  // Anchos redimensionables de los paneles laterales (px), persistidos.
  let sidebarW = $state(280);
  let panelW = $state(320);
  let dragging = $state<null | "sidebar" | "panel">(null);
  let dragStartX = 0;
  let dragStartW = 0;

  const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

  try {
    const saved = JSON.parse(localStorage.getItem("layout") ?? "{}");
    if (typeof saved.sidebarW === "number") sidebarW = clamp(saved.sidebarW, 200, 480);
    if (typeof saved.panelW === "number") panelW = clamp(saved.panelW, 220, 560);
  } catch {}

  function startDrag(which: "sidebar" | "panel", e: PointerEvent) {
    dragging = which;
    dragStartX = e.clientX;
    dragStartW = which === "sidebar" ? sidebarW : panelW;
    window.addEventListener("pointermove", onDrag);
    window.addEventListener("pointerup", endDrag);
    e.preventDefault();
  }

  function onDrag(e: PointerEvent) {
    const dx = e.clientX - dragStartX;
    if (dragging === "sidebar") {
      sidebarW = clamp(dragStartW + dx, 200, 480);
    } else if (dragging === "panel") {
      // El panel está a la derecha: arrastrar hacia la izquierda lo agranda.
      panelW = clamp(dragStartW - dx, 220, 560);
    }
  }

  function endDrag() {
    dragging = null;
    window.removeEventListener("pointermove", onDrag);
    window.removeEventListener("pointerup", endDrag);
    localStorage.setItem("layout", JSON.stringify({ sidebarW, panelW }));
  }

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
  let updateStatus = $state<UpdateStatus | null>(null);

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

    startUpdateChecker((s) => {
      updateStatus = s;
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

<div class="shell">
  <TitleBar
    {status}
    {loadProgress}
    {pendingDownloads}
    {downloads}
    onToggleSidebar={() => (sidebarOpen = !sidebarOpen)}
  />

  <div
    class="app"
    class:sidebar-collapsed={!sidebarOpen}
    class:dragging={dragging !== null}
    style="grid-template-columns: {sidebarOpen ? sidebarW : 0}px minmax(0, 1fr) {panelW}px"
  >
  <aside class="app__sidebar">
    <Sidebar
      {status}
      {downloads}
      {loadProgress}
      {updateStatus}
      onDownloadsChange={refreshDownloads}
      onStatusChange={refreshStatus}
    />
  </aside>

  {#if sidebarOpen}
    <div
      class="resizer"
      class:active={dragging === "sidebar"}
      style="left: {sidebarW}px"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize sidebar"
      onpointerdown={(e) => startDrag("sidebar", e)}
    ></div>
  {/if}
  <div
    class="resizer"
    class:active={dragging === "panel"}
    style="right: {panelW}px"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize panel"
    onpointerdown={(e) => startDrag("panel", e)}
  ></div>

  <main class="app__main">
    <div class="mode-switch">
      <div class="mode-tabs">
        <button class="mode-tab" class:active={mode === "chat"} onclick={() => (mode = "chat")}>
          <Icon name="chat" size="sm" /> Chat
        </button>
        <button class="mode-tab" class:active={mode === "agent"} onclick={() => (mode = "agent")}>
          <Icon name="agent" size="sm" /> Agent
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
</div>

<style>
  /* El grid base vive en app.css; aquí lo hacemos contenedor de las manijas. */
  .app {
    position: relative;
  }
  /* Sin animación de columnas mientras se arrastra (evita lag) y sin selección. */
  .app.dragging {
    transition: none;
    user-select: none;
    cursor: col-resize;
  }
  .resizer {
    position: absolute;
    top: 0;
    bottom: 26px; /* deja libre la barra de estado */
    width: 9px;
    transform: translateX(-50%);
    cursor: col-resize;
    z-index: 6;
    touch-action: none;
  }
  /* Línea visible solo al pasar/arrastrar, centrada en la manija. */
  .resizer::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 2px;
    transform: translateX(-50%);
    background: transparent;
    transition: background var(--t);
  }
  .resizer:hover::after,
  .resizer.active::after {
    background: var(--accent);
  }

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
