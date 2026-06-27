<script lang="ts">
  import { api, modelFit, detectHardware, type Hardware, type Fit } from "../lib/api";
  import type { CatalogModel, HfModel, HfFile, Topic } from "../lib/types";
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

  // Browse HF: all GGUF models, no query.
  let view = $state<"catalog" | "browse">("catalog");
  let sort = $state("downloads");
  let browseResults = $state<HfModel[]>([]);
  let browsing = $state(false);
  let browseLoaded = $state(false);
  let browseLimit = $state(40);

  // Topics / intent-based navigation.
  let topics = $state<Topic[]>([]);
  let activeTopic = $state<Topic | null>(null);
  let topicResults = $state<HfModel[]>([]);
  let topicLoading = $state(false);

  // Detected hardware for the fit badge.
  let hardware = $state<Hardware | null>(null);
  let hwLabel = $state("");
  let contextSize = $state(4096);

  const sortOptions = [
    { value: "downloads", label: "Most downloaded" },
    { value: "likes", label: "Most liked" },
    { value: "trendingScore", label: "Trending" },
    { value: "lastModified", label: "Recent" },
  ];

  $effect(() => {
    Promise.all([api.listCatalog(), api.listTopics(), detectHardware()]).then(
      ([c, tp, hw]) => {
        catalog = c;
        topics = tp;
        loading = false;
        hardware = hw.hardware;
        hwLabel = hw.label;
        contextSize = hw.contextSize;
      }
    );
  });

  function fitFor(m: CatalogModel): Fit {
    return modelFit(m.size_gb, contextSize, hardware);
  }

  let recommendedModels = $derived.by(() => {
    if (!activeTopic) return [];
    return activeTopic.recommended_model_ids
      .map((id) => catalog.find((m) => m.id === id))
      .filter((m): m is CatalogModel => !!m);
  });

  async function selectTopic(t: Topic) {
    activeTopic = t;
    query = "";
    searched = false;
    selectedHf = null;
    topicLoading = true;
    topicResults = [];
    try {
      const lists = await Promise.all(t.hf_queries.map((q) => api.searchHf(q)));
      const seen = new Set<string>();
      const merged: HfModel[] = [];
      for (const list of lists) {
        for (const m of list) {
          if (!seen.has(m.repo)) {
            seen.add(m.repo);
            merged.push(m);
          }
        }
      }
      merged.sort((a, b) => b.downloads - a.downloads);
      topicResults = merged.slice(0, 40);
    } catch (e) {
      topicResults = [];
    } finally {
      topicLoading = false;
    }
  }

  function showCatalog() {
    activeTopic = null;
    query = "";
    searched = false;
    view = "catalog";
  }

  function showBrowse() {
    activeTopic = null;
    query = "";
    searched = false;
    view = "browse";
    if (!browseLoaded) doBrowse();
  }

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
    activeTopic = null;
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
        filesError = "No .gguf files were found in this repo";
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
    const order = ["Tiny", "Light", "Medium", "Heavy", "MoE / Heavy+"];
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
      <button class="ghost" onclick={back}>← Back</button>
    </div>
    <div class="scroll" style="padding:8px 10px">
      {#if loadingFiles}
        <div class="muted small">Listing GGUF files...</div>
      {:else if filesError}
        <div class="small" style="color:var(--error)">{filesError}</div>
      {:else}
        <div class="small muted" style="margin-bottom:6px">
          {hfFiles.length} GGUF file(s). Choose a quantization:
        </div>
        {#each hfFiles as f (f.path)}
          <div class="file-row">
            <div class="col" style="flex:1;min-width:0">
              <div style="font-weight:500">{f.path.split("/").pop()}</div>
              <div class="dim small">{humanSize(f.size)}</div>
            </div>
            <button class="primary small-btn" onclick={() => downloadHfFile(f)}>
              ↓ Download
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
        placeholder="Search Hugging Face (for example: qwen 3.6, gemma 4)"
        bind:value={query}
        onkeydown={onKeydown}
      />
      <button onclick={doSearch} disabled={searching}>
        {searching ? "…" : "Search"}
      </button>
    </div>

    {#if hwLabel}
      <div class="hwbar" title="Estimates which models fit based on your free memory">
        <span>🖥️</span><span class="small">Your machine: {hwLabel}</span>
      </div>
    {/if}

    <div class="chips">
      <button
        class="chip"
        class:active={!activeTopic && !searched && view === "catalog"}
        onclick={showCatalog}
      >
        📦 Catalog
      </button>
      <button
        class="chip"
        class:active={!activeTopic && !searched && view === "browse"}
        onclick={showBrowse}
      >
        🧭 Browse HF
      </button>
      <span class="chip-sep"></span>
      {#each topics as t (t.id)}
        <button
          class="chip"
          class:active={activeTopic?.id === t.id}
          onclick={() => selectTopic(t)}
          title={t.blurb}
        >
          {t.icon} {t.label}
        </button>
      {/each}
    </div>

    {#if searched}
      <div class="scroll" style="padding:8px 10px">
        <div class="small muted" style="margin-bottom:6px">
          {searchResults.length} result(s) on HF Hub
        </div>
        {#each searchResults as m (m.repo)}
          {@render hfRow(m)}
        {/each}
      </div>
    {:else if activeTopic}
      <div class="scroll" style="padding:8px 10px">
        {#if activeTopic.note}
          <div class="topic-note">{activeTopic.note}</div>
        {/if}
        {#if topicLoading}
          <div class="muted small" style="padding:6px 0">
            Searching “{activeTopic.label}” on Hugging Face...
          </div>
        {:else}
          {#if recommendedModels.length}
            <div class="section-label">Recommended Generalists</div>
            {#each recommendedModels as m (m.id)}
              <ModelCard model={m} fit={fitFor(m)} onDownload={downloadFromCatalog} />
            {/each}
          {/if}
          <div class="section-label">
            {recommendedModels.length ? "Specialized Models (Hugging Face)" : "Results on Hugging Face"}
          </div>
          {#if topicResults.length === 0}
            <div class="muted small">No specialized results for this topic.</div>
          {:else}
            {#each topicResults as m (m.repo)}
              {@render hfRow(m)}
            {/each}
          {/if}
        {/if}
      </div>
    {:else}
      {#if view === "catalog"}
        {#if loading}
          <div class="muted small" style="padding:10px">Loading catalog...</div>
        {:else}
          <div class="scroll" style="padding:8px 10px">
            {#each groupedCatalog as group (group.category)}
              <div class="section-label">{group.category}</div>
              {#each group.models as m (m.id)}
                <ModelCard model={m} fit={fitFor(m)} onDownload={downloadFromCatalog} />
              {/each}
            {/each}
          </div>
        {/if}
      {:else}
        <div class="browse-bar">
          <span class="small muted">All GGUF models on the Hub</span>
          <div class="sort-wrap">
            <Select value={sort} options={sortOptions} onChange={changeSort} />
          </div>
        </div>
        <div class="scroll" style="padding:8px 10px">
          {#if browsing && browseResults.length === 0}
            <div class="muted small" style="padding:10px">Loading models...</div>
          {:else if browseResults.length === 0}
            <div class="muted small" style="padding:10px">No results.</div>
          {:else}
            {#each browseResults as m (m.repo)}
              {@render hfRow(m)}
            {/each}
            {#if browseResults.length >= browseLimit}
              <button class="ghost loadmore" onclick={loadMore} disabled={browsing}>
                {browsing ? "Loading..." : "Load more"}
              </button>
            {/if}
          {/if}
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .search-box {
    display: flex;
    gap: 6px;
    padding: 8px 10px;
  }
  .hwbar {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    margin: 2px 10px 8px;
    padding: 7px 10px;
    background: var(--bg-2);
    border: 1px solid var(--border-soft);
    border-radius: var(--radius-sm);
    color: var(--text-2);
    line-height: 1.45;
  }
  .hwbar > span:last-child {
    flex: 1;
    min-width: 0;
  }
  .chips {
    display: flex;
    align-items: center;
    gap: 7px;
    overflow-x: auto;
    padding: 0 10px 10px;
    scrollbar-width: thin;
  }
  .chip-sep {
    flex: none;
    width: 1px;
    align-self: stretch;
    margin: 2px 2px;
    background: var(--border);
  }
  .chips::-webkit-scrollbar {
    height: 5px;
  }
  .chip {
    flex: none;
    border: 1px solid var(--border);
    background: var(--bg-2);
    border-radius: 999px;
    padding: 5px 11px;
    font-size: 12px;
    color: var(--text-1);
    white-space: nowrap;
    cursor: pointer;
  }
  .chip:hover {
    background: var(--bg-hover);
    border-color: var(--text-3);
  }
  .chip.active {
    background: var(--accent);
    color: var(--accent-contrast);
    border-color: var(--accent);
    font-weight: 600;
  }
  .topic-note {
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-2);
    background: color-mix(in srgb, #d29922 12%, transparent);
    border: 1px solid color-mix(in srgb, #d29922 35%, transparent);
    border-radius: var(--radius-sm);
    padding: 6px 9px;
    margin-bottom: 8px;
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
