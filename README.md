<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-light.png">
    <img src="docs/logo-dark.png" alt="Agent Aleph" width="96" />
  </picture>
</p>

# Agent Aleph

> **Un agente de codificación que corre 100% en tu máquina.** Descargás un modelo, lo
> cargás con un clic y le pedís que lea, escriba y ejecute en tu proyecto — sin nube, sin
> API keys, sin cuentas. Todo local, todo simple.

<p align="center">
  <img src="docs/screenshot.png" alt="Agent Aleph — modo Agente con el panel de Skills, Conexiones y Contexto" width="900" />
</p>

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
- ⚡ **Pensado para modelos locales imperfectos.** Gramática GBNF (o tool-calling nativo si
  el modelo lo soporta bien) para tool-calls siempre válidos, detección de bucles y
  compactación de contexto con resumen.
- 🧩 **Extensible con skills.** Sumale conocimiento especializado al agente con paquetes de
  instrucciones locales, sin tocar código.

---

## Características

### Agente
- **Bucle de agente** con tope de pasos y detección de bucles consciente de estado (una
  escritura exitosa resetea el contador: releer un archivo que el propio agente acaba de
  modificar no se confunde con repetirse).
- **7 herramientas**: `read_file`, `list`, `glob`, `grep`, `write_file`, `edit`, `bash`.
- **Tool-calling con selección automática por modelo**: nativo (`tools` + `--jinja` +
  `delta.tool_calls`, varias herramientas por paso) para modelos capaces, o **gramática
  GBNF** por-herramienta (el modelo no puede emitir argumentos mal formados) como ruta
  universal para modelos chicos. Configurable en *Ajustes* (`auto` / `nativo` / `GBNF`).
- **Permisos por herramienta** (`allow` / `ask` / `deny`) con confirmación en la UI antes de
  escribir o ejecutar.
- **Modos build / plan**: *build* opera con todo; *plan* es solo lectura (niega edits).
- **Working directory de sesión**: las herramientas de archivo se restringen a la carpeta
  que elegís.
- **Contexto de proyecto automático**: si el repo tiene `AGENTS.md` o `CLAUDE.md`, se inyectan
  en el system prompt con prioridad, igual que Claude Code / opencode.
- **Compactación con resumen**: al acercarse al límite de contexto, el harness resume los
  turnos antiguos (conservando objetivos y decisiones) en vez de simplemente borrarlos.
- **Skills**: paquetes locales de instrucciones + recursos (`SKILL.md`) que activás/desactivás
  desde el panel del agente para darle conocimiento especializado a una tarea.
- **Contexto adjunto**: arrastrá archivos o pegá texto en el panel del agente para sumarlos
  al turno sin tener que pedirle al modelo que los lea.
- **Sesiones persistentes** con memoria entre turnos.

### Modelos
- **Catálogo curado** (Qwen 3.5 / 3.6, Phi-4, Gemma 4, incluidas variantes MoE y QAT) +
  búsqueda libre y exploración del HF Hub (por descargas, likes, tendencia o recientes).
- **Navegación por temática**: chips de uso (Código, Razonamiento, Sin censura, Agente,
  Roleplay, Legal, Médico, Finanzas) que lanzan búsquedas curadas en HF. Para áreas con
  poco material (legal/médico/finanzas) se sugieren primero **generalistas fuertes** del
  catálogo y luego los especializados, con su aviso.
- **Badge "¿te entra?"**: cada modelo muestra 🟢 / 🟡 / 🔴 según tu hardware detectado —
  suma la **VRAM libre de tus GPUs + la RAM** (con más peso en VRAM) y te dice si correrá
  fluido, lento (offload a CPU) o si no entra. Sin tener que descifrar tamaños y cuantizaciones.
- **Catálogo y temas editables sin recompilar**: se cargan de un `catalog.json` local
  (`~/.config/agent-aleph/catalog.json`) con fallback embebido; personalizá modelos o
  agregá temas a mano.
- **Descarga con progreso** (velocidad, cancelable) de archivos GGUF.
- **Carga con progreso en %** y barra en vivo durante la inicialización del modelo.
- **Gestión local**: listar, cargar, expulsar, eliminar; carpetas de modelos externas.
- **Chat con streaming** token-a-token (modo "Ask" sin herramientas).
- **Paneles laterales redimensionables**: arrastrá los bordes de la barra lateral y del panel
  derecho; los anchos se recuerdan entre sesiones.

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

## GPU

`setup-llama.sh` baja por defecto el build **Vulkan x64**, así que el offload a GPU
funciona out-of-the-box con NVIDIA, AMD e Intel (vía Vulkan) — la app detecta tus GPUs y su
VRAM libre, lo refleja en el badge "¿te entra?" y reparte capas automáticamente. En
*Ajustes* podés dejar el **auto-fit** o fijar las **GPU layers** a mano.

¿Preferís otro backend? Pasá `LLAMA_FLAVOR` al script:

```bash
LLAMA_FLAVOR=x64          ./scripts/setup-llama.sh   # CPU puro
LLAMA_FLAVOR=rocm-7.2-x64 ./scripts/setup-llama.sh   # AMD ROCm
LLAMA_FLAVOR=sycl-fp16-x64 ./scripts/setup-llama.sh  # Intel oneAPI
```

Los binarios traen sus propias librerías runtime, así que funcionan sin instalar el toolkit
en el sistema. Para un build CUDA específico, reemplazá el contenido de
`src-tauri/binaries/llama-linux-x64/` con los binarios + `.so` del release de
[llama.cpp](https://github.com/ggml-org/llama.cpp/releases).

---

## Modelos recomendados según hardware

> Actualizado a **junio de 2026**. Generaciones vigentes en el catálogo: **Qwen 3.5 / 3.6**,
> **Gemma 4**, **Phi-4**. Tamaños aproximados para cuantización **Q4_K_M**.
>
> 💡 No hace falta que memorices esta tabla: la app detecta tu GPU + RAM y marca cada modelo
> con 🟢 / 🟡 / 🔴 ("¿te entra?"). Esto es solo una referencia rápida.

| Hardware | Modelo recomendado (Q4_K_M) | Tamaño aprox. |
|----------|-----------------------------|---------------|
| 8 GB RAM, CPU | Qwen 3.5 0.8B / 2B | ~0.5–1.3 GB |
| 16 GB RAM, CPU | Qwen 3.5 4B / Phi-4 mini | ~2.5 GB |
| 16 GB RAM + GPU 6–8 GB | Qwen 3.5 9B / Gemma 4 E4B | ~5 GB |
| 24–32 GB RAM/VRAM | Phi-4 14B / Gemma 4 12B | ~7–9 GB |
| 32 GB+ RAM o VRAM (MoE) | Qwen 3.6 35B-A3B *(MoE, ~3B activos → rápido)* | ~18–22 GB |

Para el **modo agente** conviene un modelo fuerte en código/tools (usá el chip **Código** o
**Agente** para buscarlos) o un **MoE** como **Qwen 3.6 35B-A3B**, que rinde como un modelo
grande pero corre rápido porque solo activa ~3B de parámetros por token. Los modelos muy
chicos (≤1B) sirven para probar el flujo, pero fallan más en tareas reales.

---

## Stack

- **Tauri 2** (backend Rust + webview nativo)
- **Svelte 5** (runes) + TypeScript + Vite
- **llama.cpp** (binario precompilado, API OpenAI-compatible)
- **Hugging Face Hub API** para catálogo y descargas

---

## Estado y hoja de ruta

El núcleo del agente está implementado: bucle + 7 herramientas + permisos + modos + mensaje
rico + GBNF/tool-calling nativo con selección por modelo + compactación con resumen +
contexto de proyecto + skills + contexto adjunto + persistencia de sesiones. El backlog
priorizado vive en [`PLAN-AGENTE.md`](./PLAN-AGENTE.md): conexiones reales (GitHub/Google),
subagentes, MCP, slash commands, diff en la UI de permisos y métricas (tok/s, contexto usado).

### Limitaciones actuales
- Una sesión de chat a la vez; un modelo cargado a la vez.
- Sin API OpenAI-compatible expuesta hacia afuera (llama-server es solo interno).
- Sin vision/multimodal.
- Bundle solo Linux x64 (Windows/Mac requieren su propio binario de `llama-server`).
- **Conexiones** (GitHub, Google) se ven en el panel del agente pero son un placeholder:
  integrarlas requiere red + OAuth, fuera del alcance 100% local actual.

---

## Licencia

MIT — ver [`LICENSE`](./LICENSE). El binario `llama-server` incluido mantiene su propia
licencia (MIT de llama.cpp, ver `src-tauri/binaries/llama-linux-x64/LICENSE`).
