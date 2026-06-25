<script lang="ts">
  // Iconos line-style (24x24, stroke). El color/grosor vienen de `svg.ico` en app.css.
  let {
    name,
    size = "",
  }: {
    name: string;
    size?: "" | "sm" | "lg";
  } = $props();

  // Cada entrada es el contenido interno del <svg viewBox="0 0 24 24">.
  const paths: Record<string, string> = {
    box: `<path d="M21 8 12 3 3 8v8l9 5 9-5z"/><path d="m3 8 9 5 9-5"/><path d="M12 13v8"/>`,
    catalog: `<rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/>`,
    download: `<path d="M12 3v12"/><path d="m7 11 5 5 5-5"/><path d="M5 21h14"/>`,
    settings: `<path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"/><path d="M19.4 13.5a1.7 1.7 0 0 0 .3 1.9l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.9 1.2v.1a2 2 0 1 1-4 0v-.2a1.7 1.7 0 0 0-2.9-1.1l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0-1.2-2.9H3a2 2 0 1 1 0-4h.2a1.7 1.7 0 0 0 1.1-2.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.2a1.7 1.7 0 0 0 2.9 1.1l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0 1.2 2.9H21a2 2 0 1 1 0 4h-.2a1.7 1.7 0 0 0-1.5 1Z"/>`,
    folder: `<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>`,
    "folder-plus": `<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M12 11v5M9.5 13.5h5"/>`,
    refresh: `<path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/><path d="M3 21v-5h5"/>`,
    grid: `<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>`,
    chat: `<path d="M21 11.5a8.5 8.5 0 0 1-12.3 7.6L3 21l1.9-5.7A8.5 8.5 0 1 1 21 11.5Z"/>`,
    agent: `<rect x="4" y="7" width="16" height="12" rx="3"/><path d="M9 2v3M15 2v3M2 12h2M20 12h2"/><circle cx="9" cy="13" r="1.2"/><circle cx="15" cy="13" r="1.2"/>`,
    send: `<path d="M22 2 11 13"/><path d="M22 2 15 22l-4-9-9-4z"/>`,
    plus: `<path d="M12 5v14M5 12h14"/>`,
    menu: `<path d="M3 6h18M3 12h18M3 18h18"/>`,
    sliders: `<path d="M4 6h10M18 6h2M4 12h2M10 12h10M4 18h12M20 18h0"/><circle cx="16" cy="6" r="2"/><circle cx="8" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>`,
    paperclip: `<path d="M21 11.5 12.5 20a5 5 0 0 1-7-7l8.5-8.5a3.3 3.3 0 0 1 4.7 4.7l-8.5 8.5a1.6 1.6 0 0 1-2.3-2.3l7.8-7.8"/>`,
    "chevron-right": `<path d="m9 6 6 6-6 6"/>`,
    "chevron-up": `<path d="m6 15 6-6 6 6"/>`,
    "chevron-down": `<path d="m6 9 6 6 6-6"/>`,
    system: `<rect x="3" y="4" width="18" height="13" rx="2"/><path d="M8 21h8M12 17v4"/>`,
    files: `<path d="M14 3v5h5"/><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/>`,
    search: `<circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/>`,
    terminal: `<path d="m7 9 3 3-3 3M13 15h4"/><rect x="3" y="4" width="18" height="16" rx="2"/>`,
    "file-text": `<path d="M14 3v5h5"/><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M9 13h6M9 17h4"/>`,
    palette: `<path d="M12 22a10 10 0 1 1 10-10c0 2.5-2 3.5-4 3.5h-1.5a2 2 0 0 0-1.5 3.3 1.8 1.8 0 0 1-1.5 3.2Z"/><circle cx="7.5" cy="11" r="1"/><circle cx="12" cy="7.5" r="1"/><circle cx="16.5" cy="11" r="1"/>`,
    x: `<path d="M18 6 6 18M6 6l12 12"/>`,
    dots: `<circle cx="12" cy="5" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="12" cy="19" r="1.4"/>`,
    eject: `<path d="m12 4 7 9H5z"/><path d="M5 19h14"/>`,
    check: `<path d="m20 6-11 11-5-5"/>`,
    stop: `<rect x="6" y="6" width="12" height="12" rx="2"/>`,
    spark: `<path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2.5 2.5M15.5 15.5 18 18M18 6l-2.5 2.5M8.5 15.5 6 18"/>`,
    alert: `<path d="M12 3 2 20h20z"/><path d="M12 10v4"/><path d="M12 17h0"/>`,
  };

  const inner = $derived(paths[name] ?? "");
</script>

<svg
  class="ico {size}"
  viewBox="0 0 24 24"
  aria-hidden="true"
  xmlns="http://www.w3.org/2000/svg">
  {@html inner}
</svg>
