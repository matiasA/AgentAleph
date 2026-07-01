<script lang="ts">
  import { api } from "../lib/api";
  import type { Settings as S, GpuDevice } from "../lib/types";
  import Select from "./Select.svelte";

  const cacheOptions = [
    { value: "f16", label: "f16 (max quality)" },
    { value: "q8_0", label: "q8_0 (~50% memory)" },
    { value: "q4_0", label: "q4_0 (~75% memory)" },
  ];

  const toolCallingOptions = [
    { value: "auto", label: "auto (based on model)" },
    { value: "native", label: "native (tools + jinja)" },
    { value: "grammar", label: "GBNF (grammar, universal)" },
  ];

  let {
    onSaved,
  }: { onSaved?: () => void } = $props();

  let s = $state<S | null>(null);
  let saving = $state(false);
  let savedAt = $state<number | null>(null);
  let gpus = $state<GpuDevice[]>([]);
  let loadingGpus = $state(true);

  let deviceOptions = $derived([
    { value: "auto", label: gpus.length ? "auto (recommended)" : "auto (let llama-server choose)" },
    { value: "cpu", label: "cpu (CPU only)" },
    ...gpus.map((g) => ({ value: g.id, label: `${g.id}: ${g.name} (${g.free_mb} MB free)` })),
  ]);

  $effect(() => {
    api.getSettings().then((v) => (s = v));
    api.listGpus()
      .then((d) => (gpus = d))
      .catch(() => (gpus = []))
      .finally(() => (loadingGpus = false));
  });

  async function save() {
    if (!s) return;
    saving = true;
    try {
      await api.saveSettings(s);
      savedAt = Date.now();
      onSaved?.();
      setTimeout(() => (savedAt = null), 1500);
    } finally {
      saving = false;
    }
  }

  function reset() {
    api.getSettings().then((v) => (s = v));
  }
</script>

{#if s}
  <div class="col" style="flex:1;overflow:hidden">
    <div class="row between" style="padding:8px 10px;border-bottom:1px solid var(--border)">
      <span class="small muted">Inference Settings</span>
      <div class="row" style="gap:6px">
        <button class="ghost small-btn" onclick={reset}>Restore</button>
        <button class="primary small-btn" onclick={save} disabled={saving}>
          {saving ? "Saving..." : savedAt ? "Saved ✓" : "Save"}
        </button>
      </div>
    </div>

    <div class="scroll" style="padding:10px">
      <div class="field">
        <label>System Prompt</label>
        <textarea bind:value={s.system_prompt} rows="4"></textarea>
      </div>

      <div class="grid">
        <div class="field">
          <label>Temperature <span class="dim">{s.temperature.toFixed(2)}</span></label>
          <input type="range" min="0" max="2" step="0.05" bind:value={s.temperature} />
        </div>
        <div class="field">
          <label>Top P <span class="dim">{s.top_p.toFixed(2)}</span></label>
          <input type="range" min="0" max="1" step="0.05" bind:value={s.top_p} />
        </div>
        <div class="field">
          <label>Max tokens</label>
          <input type="number" min="16" max="32768" step="16" bind:value={s.max_tokens} />
        </div>
        <div class="field">
          <label>Context size</label>
          <input type="number" min="512" max="131072" step="512" bind:value={s.context_size} />
        </div>
        <div class="field">
          <label>Repeat penalty <span class="dim">{s.repeat_penalty.toFixed(2)}</span></label>
          <input type="range" min="1" max="2" step="0.05" bind:value={s.repeat_penalty} />
        </div>
        <div class="field">
          <label>Threads</label>
          <input type="number" min="1" max="64" bind:value={s.threads} />
        </div>
        <div class="field">
          <label>GPU layers <span class="dim">(0 = CPU, 99 = all on GPU)</span></label>
          <input type="number" min="0" max="999" bind:value={s.n_gpu_layers} disabled={s.gpu_layers_auto} />
        </div>
      </div>

      <div class="field" style="margin-top:4px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.gpu_layers_auto} class="checkbox" />
          <span>Automatic GPU layers <span class="dim">(-fit: llama.cpp distributes layers based on free memory; recommended)</span></span>
        </label>
      </div>

      <div class="field" style="margin-top:4px">
        <label>GPU device</label>
        {#if loadingGpus}
          <div class="dim small">Detecting GPUs...</div>
        {:else if gpus.length === 0}
          <Select bind:value={s.device} options={deviceOptions} />
        {:else}
          <Select bind:value={s.device} options={deviceOptions} />
          <div class="dim small" style="margin-top:4px">
            {#each gpus as g (g.id)}
              <div>{g.name} — {g.total_mb} MB total, {g.free_mb} MB free</div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="section-title">Optimization / Memory</div>
      <div class="grid">
        <div class="field">
          <label>KV cache — key <span class="dim">(less context RAM)</span></label>
          <Select bind:value={s.cache_type_k} options={cacheOptions} />
        </div>
        <div class="field">
          <label>KV cache — value</label>
          <Select bind:value={s.cache_type_v} options={cacheOptions} />
        </div>
        <div class="field">
          <label>Batch size <span class="dim">(prefill)</span></label>
          <input type="number" min="32" max="4096" step="32" bind:value={s.n_batch} />
        </div>
      </div>

      <div class="field" style="margin-top:4px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.use_mmap} class="checkbox" />
          <span>Use mmap <span class="dim">(load on demand; disable to force everything into RAM)</span></span>
        </label>
      </div>
      <div class="field">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.use_mlock} class="checkbox" />
          <span>mlock <span class="dim">(locks the model in RAM, avoids swap; requires enough RAM)</span></span>
        </label>
      </div>

      <div class="field" style="margin-top:8px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.enable_thinking} class="checkbox" />
          <span>Thinking mode <span class="dim">(visible reasoning, Qwen/Phi models)</span></span>
        </label>
      </div>

      <div class="section-title">Agent</div>
      <div class="field">
        <label>Brave Search API key <span class="dim">(optional — for reliable web_search without rate limits)</span></label>
        <input
          type="password"
          class="input"
          placeholder="Leave empty to use DuckDuckGo (may get rate-limited)"
          bind:value={s.brave_api_key}
        />
        <div class="dim small" style="margin-top:4px">
          Free at <strong>brave.com/search/api</strong> — 2000 searches/month. If empty, falls back to DuckDuckGo.
        </div>
      </div>
      <div class="field">
        <label>Tool calling <span class="dim">(how the agent invokes tools)</span></label>
        <Select bind:value={s.tool_calling} options={toolCallingOptions} />
        <div class="dim small" style="margin-top:4px">
          <strong>auto</strong>: native for capable models (Coder/Qwen/Llama 3.1+...), GBNF for
          smaller ones. <strong>GBNF</strong> forces grammar mode, which is more robust on weaker
          models. Applies after <strong>reloading the model</strong>.
        </div>
      </div>

      <div class="note small muted">
        GPU layers, context size, threads, KV cache, batch, mmap, and mlock apply after
        <strong>reloading the model</strong>. Temperature, top_p, and system prompt apply to the
        next message.
        <br /><br />
        For <strong>larger models</strong> on the same machine: raise context size and set the
        KV cache to <strong>q8_0</strong> (about half the context memory, quality almost intact).
        If the model does not fit in VRAM, lower GPU layers; if you have spare RAM and want to
        avoid swap, enable mlock.
      </div>
    </div>
  </div>
{/if}

<style>
  .field {
    margin-bottom: 12px;
  }
  .field label {
    display: block;
    font-size: 11px;
    color: var(--text-1);
    margin-bottom: 4px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .section-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 1px;
    color: var(--text-2);
    margin: 14px 0 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .note {
    margin-top: 10px;
    padding: 8px;
    border-left: 2px solid var(--border);
    line-height: 1.5;
  }
  .small-btn {
    padding: 3px 10px;
    font-size: 11px;
  }
  input[type="range"] {
    width: 100%;
    padding: 0;
    background: transparent;
    border: none;
  }
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .checkbox {
    width: auto;
    margin: 0;
    cursor: pointer;
  }
</style>
