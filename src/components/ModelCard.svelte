<script lang="ts">
  import type { CatalogModel } from "../lib/types";
  import type { Fit } from "../lib/api";
  import ModelFamilyBadge from "./ModelFamilyBadge.svelte";

  let {
    model,
    onDownload,
    fit = null,
  }: {
    model: CatalogModel;
    onDownload: (m: CatalogModel, file: string) => void;
    fit?: Fit | null;
  } = $props();

  let downloading = $state(false);

  // Marca de "español nativo" derivada de los tags (sin campo extra en el modelo).
  const isSpanish = $derived(
    model.tags.some((t) => ["es", "espanol", "español", "multilingue", "multilingüe"].includes(t.toLowerCase()))
  );

  async function handleDownload() {
    downloading = true;
    try {
      await onDownload(model, model.default_file);
    } finally {
      downloading = false;
    }
  }
</script>

<div class="model-card">
  <div class="row between" style="margin-bottom:6px">
    <ModelFamilyBadge name={`${model.name} ${model.tags.join(" ")}`} />
    <div class="col" style="flex:1;min-width:0">
      <div class="row" style="gap:6px;flex-wrap:wrap">
        <span class="name">{model.name}</span>
        <span class="tag">{model.params}</span>
        <span class="tag dim-tag">{model.size_gb} GB</span>
        {#if isSpanish}<span class="tag" title="Buen soporte de español">🇪🇸 Español</span>{/if}
      </div>
      <div class="dim small">{model.author}/{model.repo.split("/")[1] ?? model.repo}</div>
    </div>
    {#if fit && fit.level !== "unknown"}
      <span class="fit fit-{fit.level}" title={fit.detail}>
        <span class="dot"></span>{fit.label}
      </span>
    {/if}
  </div>
  <div class="desc muted small">{model.description}</div>
  {#if model.tags.length}
    <div class="row" style="flex-wrap:wrap;gap:4px;margin-top:6px">
      {#each model.tags as t}
        <span class="tag">{t}</span>
      {/each}
    </div>
  {/if}
  <div class="row between" style="margin-top:8px">
    <span class="dim small">Q4_K_M · {model.size_gb} GB</span>
    <button class="primary small-btn" onclick={handleDownload} disabled={downloading}>
      {downloading ? "Iniciando..." : "Descargar"}
    </button>
  </div>
</div>

<style>
  .model-card {
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px;
    margin-bottom: 8px;
  }
  .model-card:hover {
    border-color: var(--text-3);
  }
  .name {
    font-weight: 600;
    color: var(--text-0);
  }
  .desc {
    margin-top: 4px;
    line-height: 1.45;
  }
  .small-btn {
    padding: 3px 10px;
    font-size: 11px;
  }
  .dim-tag {
    color: var(--text-2);
  }
  .fit {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 999px;
    white-space: nowrap;
    align-self: flex-start;
  }
  .fit .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: currentColor;
  }
  .fit-green {
    color: #2ea043;
    background: color-mix(in srgb, #2ea043 16%, transparent);
  }
  .fit-amber {
    color: #d29922;
    background: color-mix(in srgb, #d29922 16%, transparent);
  }
  .fit-red {
    color: #f85149;
    background: color-mix(in srgb, #f85149 16%, transparent);
  }
</style>
