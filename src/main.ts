import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";
import { initAccent } from "./lib/theme";

// Solo en navegador (dev) sin runtime de Tauri: instala mocks de IPC para inspección visual.
// Dentro de la app Tauri real `__TAURI_INTERNALS__` existe y este bloque se omite. El import
// dinámico mantiene `devMocks` fuera del bundle de producción (DEV es false en `tauri build`).
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  const { installDevMocks } = await import("./lib/devMocks");
  installDevMocks();
}

initAccent();

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
