# Agent Aleph

> **Un agente de codificación que corre 100% en tu máquina.** Descargás un modelo, lo
> cargás con un clic y le pedís que lea, escriba y ejecute en tu proyecto — sin nube, sin
> API keys, sin cuentas. Todo local, todo simple.

Agent Aleph junta dos mitades en una sola app de escritorio:

- **Gestor de modelos** estilo LM Studio: catálogo, descarga de GGUF desde Hugging Face,
  carga/descarga con un clic y ajustes de inferencia.
- **Agente de codificación** estilo Codex / Claude Code: un *harness* con bucle de
  herramientas, permisos y modos, capaz de operar sobre una carpeta de tu proyecto.

El motor es **llama.cpp** (`llama-server` precompilado y embebido), controlado por la app
vía su API OpenAI-compatible. No necesitás instalar nada del lado del modelo: viene en el
bundle.

---

## ¿Por qué Agent Aleph?

- 🔒 **100% local y privado.** Tus prompts, tu código y tus modelos no salen de tu equipo.
- 🧰 **Es un agente, no solo un chat.** Lee archivos, busca, edita, crea y ejecuta comandos
  bajo tu permiso.
- 🪶 **Simple por diseño.** Una ventana: descargás, cargás, trabajás. Sin configurar
  servidores ni endpoints.
- 🛡️ **Seguro por defecto.** Permisos *allow / ask / deny* por herramienta y un **modo plan**
  de solo lectura que no toca el disco.
- ⚡ **Pensado para modelos locales imperfectos.** Gramática GBNF para tool-calls siempre
  válidos, detección de bucles y gestión agresiva de contexto.

---

## Características

### Agente
- **Bucle de agente** con tope de pasos y detección de bucles (no se cuelga repitiendo).
- **7 herramientas**: `read_file`, `list`, `glob`, `grep`, `write_file`, `edit`, `bash`.
- **Permisos por herramienta** (`allow` / `ask` / `deny`) con confirmación en la UI antes de
  escribir o ejecutar.
- **Modos build / plan**: *build* opera con todo; *plan* es solo lectura (niega edits).
- **Working directory de sesión**: las herramientas de archivo se restringen a la carpeta
  que elegís.
- **Tool-calls garantizados** vía gramática **GBNF** (el modelo no puede emitir argumentos
  mal formados) + validación de schema.
- **Sesiones persistentes** con memoria entre turnos.

### Modelos
- **Catálogo curado** (Llama 3.2, Qwen, Phi 3.5, Mistral, Gemma…) + búsqueda libre en HF Hub.
- **Descarga con progreso** (velocidad, cancelable) de archivos GGUF.
- **Carga con progreso en %** y barra en vivo durante la inicialización del modelo.
- **Gestión local**: listar, cargar, expulsar, eliminar; carpetas de modelos externas.
- **Chat con streaming** token-a-token (modo "Ask" sin herramientas).

### Inferencia
- Ajustes: system prompt, temperature, top_p, max_tokens, context size, repeat penalty,
  threads, GPU layers.
- Palancas de runtime: cuantización de caché KV, `mmap`/`mlock`, batch size, auto-fit de
  capas a GPU.

---

## Instalación

> **Plataforma:** Linux x64 (Ubuntu/Debian con `webkit2gtk-4.1`, `gtk-3`).
> Requiere **Rust 1.77+** y **Node 18+**.

```bash
git clone https://github.com/matiasA/AgentAleph.git
cd AgentAleph
npm install
./scripts/setup-llama.sh   # descarga el binario de llama.cpp (no va en git por peso)
npm run tauri dev          # desarrollo (hot reload)
# o
npm run tauri build        # build de producción (.deb / .AppImage)
```

El binario de `llama-server` **no se versiona** (pesa ~130 MB); `setup-llama.sh` lo
descarga del release oficial de llama.cpp y lo deja en
`src-tauri/binaries/llama-linux-x64/`. Variante por defecto: Vulkan x64. Para CPU puro:
`LLAMA_FLAVOR=x64 ./scripts/setup-llama.sh`.

### Generar instalables (CI)

Los `.deb` / `.AppImage` se construyen en GitHub Actions **cuando quieras**:

- **Manual:** pestaña *Actions* → *Build installers* → *Run workflow*. Quedan como
  artefactos descargables.
- **Por versión:** empujá un tag `vX.Y.Z` y además se crea un *Release* con los instalables
  adjuntos.

```bash
git tag v0.1.0 && git push origin v0.1.0   # dispara build + Release
```

La app crea estos directorios:
- `~/.local/share/agent-aleph/models/` — modelos GGUF descargados
- `~/.config/agent-aleph/settings.json` — ajustes persistentes

---

## Uso en 4 pasos

1. **Descargar** un modelo: pestaña *Catálogo* → elegir → *Descargar* (Q4_K_M por defecto).
2. **Cargar**: pestaña *Modelos* → *Cargar* (verás el progreso en %).
3. **Elegir modo**: *Chat* para conversar, *Agente* para que opere sobre tu proyecto.
4. **Trabajar**: en modo Agente, elegí la carpeta del proyecto y pedile la tarea. Aprobá o
   rechazá cada acción de escritura/ejecución cuando te lo pida.

---

## Habilitar GPU (NVIDIA, opcional)

El bundle usa el build **CPU** de llama.cpp. Para offload a GPU:

1. Descargá un build con GPU de `llama-server` desde
   [llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
   (ej. el paquete CUDA o Vulkan para Linux x64).
2. Reemplazá el contenido de `src-tauri/binaries/llama-linux-x64/` con esos binarios + `.so`.
3. En *Ajustes* → **GPU layers** subí el valor (o dejá el auto-fit).

Los binarios traen sus propias librerías runtime, así que funcionan sin instalar el toolkit
en el sistema.

---

## Modelos recomendados según hardware

| Hardware | Modelo (Q4_K_M) | Tamaño |
|----------|-----------------|--------|
| 8 GB RAM, CPU | Llama 3.2 1B / Qwen 0.5–1B | ~0.7–0.8 GB |
| 16 GB RAM, CPU | Qwen 2.5 3B / Phi 3.5 mini | ~2 GB |
| 16 GB RAM + GPU 8 GB | Qwen 2.5 7B / Llama 3.1 8B | ~4.5–5 GB |
| 32 GB+ RAM | Coder 7B+ con contexto largo | ~5 GB+ |

Para el **modo agente** conviene un modelo *coder* de 7B o más; los muy chicos sirven para
probar el flujo pero fallan más en tareas reales.

---

## Stack

- **Tauri 2** (backend Rust + webview nativo)
- **Svelte 5** (runes) + TypeScript + Vite
- **llama.cpp** (binario precompilado, API OpenAI-compatible)
- **Hugging Face Hub API** para catálogo y descargas

---

## Estado y hoja de ruta

El núcleo del agente (bucle + herramientas + permisos + modos + GBNF + contexto +
persistencia) está implementado. El backlog priorizado vive en
[`PLAN-AGENTE.md`](./PLAN-AGENTE.md): schema/GBNF por-herramienta, compactación con resumen,
tool-calling nativo (`--jinja`), subagentes y métricas.

### Limitaciones actuales
- Una sesión de chat a la vez; un modelo cargado a la vez.
- Sin API OpenAI-compatible expuesta hacia afuera (llama-server es solo interno).
- Sin vision/multimodal.
- Bundle solo Linux x64 (Windows/Mac requieren su propio binario de `llama-server`).

---

## Licencia

MIT — ver [`LICENSE`](./LICENSE). El binario `llama-server` incluido mantiene su propia
licencia (MIT de llama.cpp, ver `src-tauri/binaries/llama-linux-x64/LICENSE`).
