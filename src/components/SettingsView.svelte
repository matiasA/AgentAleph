<script lang="ts">
  import { api } from "../lib/api";
  import type { Settings as S, GpuDevice } from "../lib/types";

  let {
    onSaved,
  }: { onSaved?: () => void } = $props();

  let s = $state<S | null>(null);
  let saving = $state(false);
  let savedAt = $state<number | null>(null);
  let gpus = $state<GpuDevice[]>([]);
  let loadingGpus = $state(true);

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
      <span class="small muted">Ajustes de inferencia</span>
      <div class="row" style="gap:6px">
        <button class="ghost small-btn" onclick={reset}>Restaurar</button>
        <button class="primary small-btn" onclick={save} disabled={saving}>
          {saving ? "Guardando..." : savedAt ? "Guardado ✓" : "Guardar"}
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
          <label>GPU layers <span class="dim">(0 = CPU, 99 = todo en GPU)</span></label>
          <input type="number" min="0" max="999" bind:value={s.n_gpu_layers} disabled={s.gpu_layers_auto} />
        </div>
      </div>

      <div class="field" style="margin-top:4px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.gpu_layers_auto} class="checkbox" />
          <span>GPU layers automático <span class="dim">(-fit: llama.cpp reparte capas según memoria libre; recomendado)</span></span>
        </label>
      </div>

      <div class="field" style="margin-top:4px">
        <label>Dispositivo GPU</label>
        {#if loadingGpus}
          <div class="dim small">Detectando GPUs...</div>
        {:else if gpus.length === 0}
          <select bind:value={s.device}>
            <option value="auto">auto (dejar que llama-server elija)</option>
            <option value="cpu">cpu (solo CPU)</option>
          </select>
        {:else}
          <select bind:value={s.device}>
            <option value="auto">auto (recomendado)</option>
            <option value="cpu">cpu (solo CPU)</option>
            {#each gpus as g (g.id)}
              <option value={g.id}>{g.id}: {g.name} ({g.free_mb} MB libres)</option>
            {/each}
          </select>
          <div class="dim small" style="margin-top:4px">
            {#each gpus as g (g.id)}
              <div>{g.name} — {g.total_mb} MB total, {g.free_mb} MB libres</div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="section-title">Optimización / memoria</div>
      <div class="grid">
        <div class="field">
          <label>Caché KV — clave <span class="dim">(menos RAM de contexto)</span></label>
          <select bind:value={s.cache_type_k}>
            <option value="f16">f16 (máx. calidad)</option>
            <option value="q8_0">q8_0 (~50% memoria)</option>
            <option value="q4_0">q4_0 (~75% memoria)</option>
          </select>
        </div>
        <div class="field">
          <label>Caché KV — valor</label>
          <select bind:value={s.cache_type_v}>
            <option value="f16">f16 (máx. calidad)</option>
            <option value="q8_0">q8_0 (~50% memoria)</option>
            <option value="q4_0">q4_0 (~75% memoria)</option>
          </select>
        </div>
        <div class="field">
          <label>Batch size <span class="dim">(prefill)</span></label>
          <input type="number" min="32" max="4096" step="32" bind:value={s.n_batch} />
        </div>
      </div>

      <div class="field" style="margin-top:4px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.use_mmap} class="checkbox" />
          <span>Usar mmap <span class="dim">(carga bajo demanda; desactívalo para forzar todo a RAM)</span></span>
        </label>
      </div>
      <div class="field">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.use_mlock} class="checkbox" />
          <span>mlock <span class="dim">(fija el modelo en RAM, evita swap; requiere RAM suficiente)</span></span>
        </label>
      </div>

      <div class="field" style="margin-top:8px">
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={s.enable_thinking} class="checkbox" />
          <span>Modo thinking <span class="dim">(razonamiento visible, modelos Qwen/Phi)</span></span>
        </label>
      </div>

      <div class="note small muted">
        GPU layers, context size, threads, caché KV, batch, mmap y mlock se aplican al
        <strong>recargar el modelo</strong>. Temperature, top_p y system prompt aplican al
        siguiente mensaje.
        <br /><br />
        Para <strong>modelos más grandes</strong> en la misma máquina: subí context size y poné
        la caché KV en <strong>q8_0</strong> (mitad de memoria de contexto, calidad casi intacta).
        Si el modelo no entra en VRAM, bajá GPU layers; si te sobra RAM y querés evitar swap,
        activá mlock.
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
