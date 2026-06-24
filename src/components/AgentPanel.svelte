<script lang="ts">
  import Icon from "./Icon.svelte";

  // Panel derecho — maqueta visual. Se cableará al backend en una iteración
  // posterior; por ahora presenta las herramientas y la zona de contexto.
  let { mode = "chat" }: { mode?: "chat" | "agent" } = $props();

  let toolsOpen = $state(true);
  let dragOver = $state(false);

  const tools = [
    { icon: "system", title: "Sistema", desc: "Información del sistema y entorno" },
    { icon: "files", title: "Archivos", desc: "Sube y consulta archivos locales" },
    { icon: "search", title: "Búsqueda", desc: "Buscar en la web" },
    { icon: "terminal", title: "Ejecutar", desc: "Ejecuta comandos locales" },
  ];
</script>

<div class="panel">
  <div class="panel-section">
    <button class="section-head" onclick={() => (toolsOpen = !toolsOpen)}>
      <span class="section-label">Herramientas del Agente</span>
      <span class="chev" class:open={toolsOpen}><Icon name="chevron-up" size="sm" /></span>
    </button>

    {#if toolsOpen}
      <div class="tools">
        {#each tools as t (t.title)}
          <button class="tool-card">
            <span class="tool-ico"><Icon name={t.icon} /></span>
            <span class="tool-text">
              <span class="tool-title">{t.title}</span>
              <span class="tool-desc">{t.desc}</span>
            </span>
            <span class="tool-chev"><Icon name="chevron-right" size="sm" /></span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <div class="panel-section grow">
    <div class="section-head static">
      <span class="section-label">Contexto</span>
      <button class="add-btn"><Icon name="plus" size="sm" /> Agregar</button>
    </div>

    <div
      class="dropzone"
      class:over={dragOver}
      role="button"
      tabindex="0"
      ondragover={(e) => {
        e.preventDefault();
        dragOver = true;
      }}
      ondragleave={() => (dragOver = false)}
      ondrop={(e) => {
        e.preventDefault();
        dragOver = false;
      }}>
      <span class="dz-ico"><Icon name="file-text" size="lg" /></span>
      <div class="dz-title">Añade archivos o texto al contexto</div>
      <div class="dz-sub">Arrastra y suelta aquí o haz clic para seleccionar</div>
    </div>

    <div class="empty-context">Sin contexto agregado</div>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px 14px;
    gap: 18px;
    overflow-y: auto;
  }
  .panel-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .panel-section.grow {
    flex: 1;
    min-height: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: transparent;
    border: none;
    border-radius: 0;
    padding: 2px 2px;
    color: var(--text-2);
  }
  .section-head:hover:not(.static) {
    background: transparent;
    color: var(--text-1);
  }
  .section-head.static {
    cursor: default;
  }
  .chev {
    display: inline-flex;
    color: var(--text-2);
    transition: transform var(--t);
  }
  .chev:not(.open) {
    transform: rotate(180deg);
  }
  .add-btn {
    border: none;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--text-1);
    border-radius: var(--radius-sm);
  }
  .add-btn:hover {
    background: var(--bg-hover);
    color: var(--text-0);
  }

  .tools {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .tool-card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    text-align: left;
    background: var(--bg-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px;
    transition: background var(--t-fast), border-color var(--t-fast),
      transform var(--t-fast);
  }
  .tool-card:hover {
    background: var(--bg-3);
    border-color: var(--border-strong);
  }
  .tool-ico {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-sm);
    background: var(--bg-0);
    border: 1px solid var(--border);
    color: var(--accent-2);
    flex: none;
  }
  .tool-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .tool-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-0);
  }
  .tool-desc {
    font-size: 11px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tool-chev {
    color: var(--text-3);
    display: inline-flex;
  }
  .tool-card:hover .tool-chev {
    color: var(--text-1);
  }

  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 6px;
    padding: 26px 16px;
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--text-2);
    cursor: pointer;
    transition: border-color var(--t-fast), background var(--t-fast);
  }
  .dropzone:hover,
  .dropzone.over {
    border-color: var(--accent-border);
    background: var(--accent-bg);
  }
  .dz-ico {
    color: var(--text-3);
    margin-bottom: 2px;
  }
  .dropzone:hover .dz-ico,
  .dropzone.over .dz-ico {
    color: var(--accent-2);
  }
  .dz-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-1);
  }
  .dz-sub {
    font-size: 11px;
    color: var(--text-3);
    max-width: 220px;
  }
  .empty-context {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-soft);
    border-radius: var(--radius);
    color: var(--text-3);
    font-size: 12px;
    min-height: 80px;
  }
</style>
