<script lang="ts">
  import { api } from "../lib/api";
  import type { CatalogModel, HfModel, HfFile } from "../lib/types";
  import ModelCard from "./ModelCard.svelte";
  import Select from "./Select.svelte";

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

  // Explorar HF (todos los modelos GGUF, sin query)
  let view = $state<"catalog" | "browse">("catalog");
  let sort = $state("downloads");
  let browseResults = $state<HfModel[]>([]);
  let browsing = $state(false);
  let browseLoaded = $state(false);
  let browseLimit = $state(40);

  const sortOptions = [
    { value: "downloads", label: "Más descargados" },
    { value: "likes", label: "Más gustados" },
    { value: "trendingScore", label: "Tendencia" },
    { value: "lastModified", label: "Recientes" },
  ];

  $effect(() => {
    api.listCatalog().then((c) => {
      catalog = c;
      loading = false;
    });
  });

  async function doBrowse(reset = true) {
    if (reset) browseLimit = 40;
    browsing = true;
    browseLoaded = true;
    try {
      browseResults = await api.browseHf(sort, browseLimit);
    } catch (e) {
      browseResults = [];
    } finally {
      browsing = false;
    }
  }

  async function loadMore() {
    browseLimit += 40;
    await doBrowse(false);
  }

  function setView(v: "catalog" | "browse") {
    view = v;
    if (v === "browse" && !browseLoaded) doBrowse();
  }

  function changeSort(v: string) {
    sort = v;
    doBrowse();
  }

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
  {#snippet hfRow(m: HfModel)}
    <div class="hf-row" role="button" tabindex="0" onclick={() => openHf(m)}>
      <div class="col" style="flex:1;min-width:0">
        <div class="hf-name">{m.name}</div>
        <div class="dim small">{m.author}</div>
      </div>
      <div class="col" style="align-items:flex-end;flex:none">
        <div class="dim small">{(m.downloads / 1000).toFixed(1)}k ↓</div>
        <div class="dim small">{m.likes} ♥</div>
      </div>
    </div>
  {/snippet}

  <div class="col" style="flex:1;overflow:hidden">
    <div class="search-box">
      <input
        type="text"
        placeholder="Buscar en HuggingFace (ej: qwen 3.6, gemma 4)"
        bind:value={query}
        onkeydown={onKeydown}
      />
      <button onclick={doSearch} disabled={searching}>
        {searching ? "…" : "Buscar"}
      </button>
    </div>

    <div class="segmented">
      <button class:active={view === "catalog"} onclick={() => setView("catalog")}>
        Catálogo
      </button>
      <button class:active={view === "browse"} onclick={() => setView("browse")}>
        Explorar HF
      </button>
    </div>

    {#if searched}
      <div class="scroll" style="padding:8px 10px">
        <div class="small muted" style="margin-bottom:6px">
          {searchResults.length} resultado(s) en HF Hub
        </div>
        {#each searchResults as m (m.repo)}
          {@render hfRow(m)}
        {/each}
      </div>
    {:else if view === "catalog"}
      {#if loading}
        <div class="muted small" style="padding:10px">Cargando catálogo…</div>
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
    {:else}
      <div class="browse-bar">
        <span class="small muted">Todos los GGUF del Hub</span>
        <div class="sort-wrap">
          <Select value={sort} options={sortOptions} onChange={changeSort} />
        </div>
      </div>
      <div class="scroll" style="padding:8px 10px">
        {#if browsing && browseResults.length === 0}
          <div class="muted small" style="padding:10px">Cargando modelos…</div>
        {:else if browseResults.length === 0}
          <div class="muted small" style="padding:10px">Sin resultados.</div>
        {:else}
          {#each browseResults as m (m.repo)}
            {@render hfRow(m)}
          {/each}
          {#if browseResults.length >= browseLimit}
            <button class="ghost loadmore" onclick={loadMore} disabled={browsing}>
              {browsing ? "Cargando…" : "Cargar más"}
            </button>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .search-box {
    display: flex;
    gap: 6px;
    padding: 8px 10px;
  }
  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
    margin: 0 10px 6px;
    padding: 3px;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .segmented button {
    border: none;
    background: transparent;
    border-radius: 6px;
    padding: 6px 8px;
    font-size: 12px;
    color: var(--text-1);
  }
  .segmented button:hover {
    background: var(--bg-hover);
  }
  .segmented button.active {
    background: var(--accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }
  .browse-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 10px 8px;
    border-bottom: 1px solid var(--border-soft);
  }
  .sort-wrap {
    width: 170px;
    flex: none;
  }
  .hf-name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .loadmore {
    width: 100%;
    margin-top: 4px;
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
