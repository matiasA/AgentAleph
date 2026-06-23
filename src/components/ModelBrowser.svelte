<script lang="ts">
  import { api } from "../lib/api";
  import type { CatalogModel, HfModel, HfFile } from "../lib/types";
  import ModelCard from "./ModelCard.svelte";

  let {
    onDownload,
  }: {
    onDownload: (repo: string, filename: string, displayName: string) => void;
  } = $props();

  let catalog = $state<CatalogModel[]>([]);
  let loading = $state(true);
  let query = $state("");
  let searchResults = $state<HfModel[]>([]);
  let searching = $state(false);
  let searched = $state(false);

  let selectedHf = $state<HfModel | null>(null);
  let hfFiles = $state<HfFile[]>([]);
  let loadingFiles = $state(false);
  let filesError = $state<string | null>(null);

  $effect(() => {
    api.listCatalog().then((c) => {
      catalog = c;
      loading = false;
    });
  });

  async function doSearch() {
    if (!query.trim()) {
      searchResults = [];
      searched = false;
      return;
    }
    searching = true;
    searched = true;
    try {
      searchResults = await api.searchHf(query.trim());
    } finally {
      searching = false;
    }
  }

  async function openHf(m: HfModel) {
    selectedHf = m;
    hfFiles = [];
    filesError = null;
    loadingFiles = true;
    try {
      hfFiles = await api.listModelFiles(m.repo);
      if (hfFiles.length === 0) {
        filesError = "No se encontraron archivos .gguf en este repo";
      }
    } catch (e: any) {
      filesError = String(e);
    } finally {
      loadingFiles = false;
    }
  }

  function humanSize(n: number): string {
    if (n >= 1e9) return (n / 1e9).toFixed(2) + " GB";
    if (n >= 1e6) return (n / 1e6).toFixed(1) + " MB";
    if (n >= 1e3) return (n / 1e3).toFixed(1) + " KB";
    return n + " B";
  }

  function downloadFromCatalog(m: CatalogModel, file: string) {
    onDownload(m.repo, file, m.name);
  }

  let groupedCatalog = $derived.by(() => {
    const order = ["Ultra-ligero", "Ligero", "Mediano", "Pesado", "MoE / Pesado+"];
    const groups: Record<string, CatalogModel[]> = {};
    for (const m of catalog) {
      if (!groups[m.category]) groups[m.category] = [];
      groups[m.category].push(m);
    }
    return order
      .filter((c) => groups[c])
      .map((c) => ({ category: c, models: groups[c] }));
  });

  function downloadHfFile(f: HfFile) {
    if (!selectedHf) return;
    onDownload(selectedHf.repo, f.path, selectedHf.name + " — " + f.path.split("/").pop());
  }

  function back() {
    selectedHf = null;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") doSearch();
  }
</script>

{#if selectedHf}
  <div class="col" style="flex:1;overflow:hidden">
    <div class="row between" style="padding:8px 10px;border-bottom:1px solid var(--border)">
      <div class="col" style="min-width:0">
        <div class="small muted">Repo HF</div>
        <div style="font-weight:600">{selectedHf.repo}</div>
      </div>
      <button class="ghost" onclick={back}>← Volver</button>
    </div>
    <div class="scroll" style="padding:8px 10px">
      {#if loadingFiles}
        <div class="muted small">Listando archivos GGUF...</div>
      {:else if filesError}
        <div class="small" style="color:var(--error)">{filesError}</div>
      {:else}
        <div class="small muted" style="margin-bottom:6px">
          {hfFiles.length} archivo(s) GGUF. Elige una cuantización:
        </div>
        {#each hfFiles as f (f.path)}
          <div class="file-row">
            <div class="col" style="flex:1;min-width:0">
              <div style="font-weight:500">{f.path.split("/").pop()}</div>
              <div class="dim small">{humanSize(f.size)}</div>
            </div>
            <button class="primary small-btn" onclick={() => downloadHfFile(f)}>
              ↓ Descargar
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
{:else}
  <div class="col" style="flex:1;overflow:hidden">
    <div class="search-box">
      <input
        type="text"
        placeholder="Buscar en HuggingFace (ej: qwen 3.6, gemma 4)"
        bind:value={query}
        onkeydown={onKeydown}
      />
      <button onclick={doSearch} disabled={searching}>
        {searching ? "..." : "Buscar"}
      </button>
    </div>

    {#if searched}
      <div class="scroll" style="padding:8px 10px">
        <div class="small muted" style="margin-bottom:6px">
          {searchResults.length} resultado(s) HF Hub
        </div>
        {#each searchResults as m (m.repo)}
          <div class="hf-row" onclick={() => openHf(m)}>
            <div class="col" style="flex:1;min-width:0">
              <div style="font-weight:500">{m.name}</div>
              <div class="dim small">{m.author}</div>
            </div>
            <div class="col" style="align-items:flex-end">
              <div class="dim small">{(m.downloads / 1000).toFixed(1)}k dl</div>
              <div class="dim small">{m.likes} ♥</div>
            </div>
          </div>
        {/each}
      </div>
    {:else if loading}
      <div class="muted small" style="padding:10px">Cargando catálogo...</div>
    {:else}
      <div class="scroll" style="padding:8px 10px">
        {#each groupedCatalog as group (group.category)}
          <div class="section-label">{group.category}</div>
          {#each group.models as m (m.id)}
            <ModelCard model={m} onDownload={downloadFromCatalog} />
          {/each}
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .search-box {
    display: flex;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .section-label {
    font-size: 10px;
    color: var(--text-3);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
    margin-top: 4px;
  }
  .hf-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 6px;
    cursor: pointer;
    background: var(--bg-2);
  }
  .hf-row:hover {
    background: var(--bg-hover);
    border-color: var(--text-3);
  }
  .file-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 6px;
    background: var(--bg-2);
  }
  .file-row:hover {
    border-color: var(--text-3);
  }
  .small-btn {
    padding: 3px 10px;
    font-size: 11px;
  }
</style>
