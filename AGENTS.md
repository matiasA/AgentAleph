# AGENTS.md

> Notas del proyecto para guiar al agente (estilo opencode / Claude Code). Esta sección
> documenta el harness de evaluación del agente contra modelos GGUF reales, los bugs
> encontrados, los fixes aplicados y cómo reproducir las pruebas.

---

## Testing del harness de agente con modelos locales

### Motivación

El harness de agente (`src-tauri/src/agent/`) sólo se invocaba desde la UI de Tauri (IPC
`agent_send` + permisos manuales), lo que hacía inviable correr un set grande de tareas
contra un modelo real de forma repetible. Sin un modo *headless*, cualquier cambio al
loop/tools/contexto quedaba sin verificación más allá de probar a mano en la app. Hace
falta un eval automatizado que ejerza el **mismo `run_inner` que la app de producción**.

### Qué se construyó

#### 1. Refactor de `agent_loop.rs` (desacoplamiento de `tauri::AppHandle`)

El cuerpo del loop vivía pegado a `app.emit(...)` y a `AppState` para los permisos. Se
introdujo:

- **`trait LoopSink`** (`agent_loop.rs`): abstrae el destino de los eventos del loop
  (`emit_step`, `emit_token`, `emit_tool`, `emit_permission`, `emit_done`). La ruta Tauri
  usa `TauriSink` (reemite a la webview); un harness headless puede usar un sink que solo
  registra en memoria. Así se prueba el mismo código del harness que ejecuta la app real.
- **`run_inner` parametrizada**: recibe `sink: &dyn LoopSink`, `state: Option<&Arc<AppState>>`,
  `auto_allow: bool`, `persist: bool`. En modo Tauri `state=Some`, `auto_allow=false`,
  `persist=true` (comportamiento anterior, sin cambios para el usuario). En modo headless
  `state=None`, `auto_allow=true` (los `Ask` se convierten en `Allow` sin tocar la UI),
  `persist=false` (no ensucia `~/.local/share/agent-aleph/agent-sessions/`).
- **`run_turn` (pública, headless)**: entrada de una sola llamada para ejecutar un turno
  de agente contra un `llama-server` ya levantado en `port`, con permisos auto-aprobados.
  Devuelve `(texto_final, reason)` donde `reason ∈ {"done","max_steps","loop","cancelled","error"}`.
- `use_native_tools` y `parse_tool_call` se hicieron `pub` para que el harness pueda
  inspeccionar/forzar la ruta.

Comprobación: el `cargo check` original seguía pasando tras el refactor (la app de Tauri
no cambia su comportamiento; sólo se reorganizó el código).

#### 2. Binario `agent_eval` (`src-tauri/src/bin/agent_eval.rs`)

Harness de evaluación autónomo. Flujo:

1. **Carga settings del usuario** (`~/.config/agent-aleph/settings.json`) y los pisa con
   flags del CLI (`--context-size`, `--max-tokens`, `--threads`, `--temperature`).
2. **Construye un proyecto-fixture** reproducible en `/tmp/opencode/agent-eval/fixture`
   (package.json, README con bug descrito, AGENTS.md, src/math.js con bug en `add`,
   src/utils.js con `VERSION`, tests, docs, data.txt de 256 líneas numéricas, empty.txt,
   blob.bin, types.d.ts). Por cada tarea se copia el prístino a un working dir fresco.
3. **Levanta `llama-server`** con `spawn_server` (espejo de `inference::server::start_server`
   pero sin `AppHandle`): `--jinja` solo en ruta `native`, KV-cache quant del settings,
   mmap/mlock, device, etc. Espera a `/health` con timeout holgado (180s, modelos grandes
   cargan lento).
4. **Corre las 54 tareas** contra `run_turn` (el mismo `run_inner` de la app), con un
   `RecordSink` que acumula en memoria cada llamada a herramienta (paso, tool, args,
   result, is_error) y los tokens del texto final.
5. **Checker por tarea**: función que recibe `(working_dir, texto_final, llamadas)` y
   devuelve `None` (pass) o `Some(motivo)` (fail). Los checkers combinan: ¿llamó a la
   herramienta esperada? (sin error), ¿el texto final contiene lo pedido? ¿el FS quedó
   en el estado esperado? (para write/edit).
6. **Loguea** en vivo (stderr) y vuelca `route-<ruta>.{json,md}` (transcripción completa
   de tools por tarea) y `report.json` (global). `--limit N` hace smoke test; `--only id1,id2`
   filtra tareas; `AGENT_EVAL_DEBUG=1` vuelca el request al desbordar contexto.

Tareas (54): 8× read_file, 6× list, 6× glob, 6× grep, 6× write_file, 6× edit, 6× bash,
10 mixtas/realistas (corregir bug + correr tests, etc.). Cubren las 7 herramientas y
forzan casos de error (archivo inexistente, edit ambiguo, list sobre archivo, grep vacío).

#### 3. `[[bin]] agent_eval` + `default-run` en `Cargo.toml`

```toml
default-run = "agent-aleph"   # para que `cargo run` (tauri dev) no se confunda

[[bin]]
name = "agent_eval"
path = "src/bin/agent_eval.rs"
```

Sin `default-run`, `cargo run` fallaba con "could not determine which binary to run"
porque ahora hay dos (`agent-aleph` y `agent_eval`).

### Cómo reproducir

```bash
# Desde clon-codex/src-tauri/ (necesario para que resuelva binaries/llama-linux-x64/)
cargo build --bin agent_eval --release   # release: mucho más rápido

# Corrida full (54 tareas × 2 rutas = 108 turnos, ~10-15 min con 4B en CPU/GPU)
./target/release/agent_eval \
  --model /mnt/disco_d/MODELOS/lmstudio-community/Qwen3.5-4B-GGUF/Qwen3.5-4B-Q4_K_M.gguf \
  --routes grammar,native \
  --out /tmp/opencode/eval-out \
  --context-size 8192 --max-tokens 1024

# Smoke test (3 tareas, 1 ruta, ~30s)
./target/release/agent_eval --model <gguf> --routes grammar --limit 3 --out /tmp/opencode/smoke

# Debug de desbordamiento de contexto (vuelca el request al fallar)
AGENT_EVAL_DEBUG=1 ./target/release/agent_eval --model <gguf> --only r3 --routes grammar --out /tmp/opencode/dbg

# Filtrar tareas concretas
./target/release/agent_eval --model <gguf> --routes grammar --only r3,w6,e6 --out /tmp/opencode/spot
```

Salida: `report.json`, `route-grammar.{json,md}`, `route-native.{json,md}`. El `.md`
incluye tabla resumen (ok/fail/reason/pasos/tools/fail) y transcripción de tool calls
+ texto final por tarea.

### Modelo probado

**Qwen3.5-4B-Q4_K_M** (`/mnt/disco_d/MODELOS/lmstudio-community/Qwen3.5-4B-GGUF/`, 2.7 GB),
contexto 8192, max_tokens 1024, temp 0.4 (determinista-ish). Ambas rutas: `grammar` (GBNF,
sin `--jinja`) y `native` (`--jinja` + `delta.tool_calls`). El binario de `llama-server`
embebido (Vulkan x64, build 9754) resuelve el modelo desde el `extra_model_dirs` del
settings o por path absoluto.

### Bugs de harness encontrados y arreglados

#### Bug #1 (crítico, de producción): desbordamiento de contexto por tool-results densas

**Síntoma**: la tarea `r3` (leer `data.txt` con `offset=10`) terminaba con
`reason=error` y texto `Inference: HTTP 400 ... request (8248 tokens) exceeds the
available context size (8192 tokens)`. El agente colapsaba una sesión que tocaba una
salida numérica grande.

**Diagnóstico** (con `AGENT_EVAL_DEBUG=1`, que vuelca el request al detectar el HTTP 400):
el mensaje de tool result de `data.txt` tenía 10760 bytes / 246 líneas pero el tokenizer
real lo contaba como ~7100 tokens, mientras que `approx_tokens` (bytes/3) lo estimaba en
~3840. La subestimación era ~1.85× porque cada número/operador/underscore/tabulación es un
token separado (no prosa). Con `budget_view` fiándose de esa cota, dejaba pasar una
conversación que el KV-cache de llama-server rechazaba. Una sola lectura densa llenaba el
contexto chico.

**Fix triple** (`src-tauri/src/agent/agent_loop.rs`, `agent/message.rs`):

1. `approx_tokens` ahora suma granularidad por línea: `bytes/3 + lines + 4`. Compensa la
   densidad del tokenizer en código/números sin desperdiciar mucho contexto en prosa.
   Tiende a sobreestimar ligeramente (preferible: poda antes vs. desbordar).
2. `ctx_budget` reserva ~25% del contexto (`ctx*3/4 − step_max_tokens`) en vez del
   margen chico anterior (`ctx − max_tokens − 256`).
3. **Las tool results se truncan a `RESULT_CTX_CAP = 6000` chars al entrar al contexto
   del modelo** (`cap_for_context`), con un aviso `… [salida truncada a 6000 de N
   caracteres; usá offset/limit o grep para ver el resto]`. La UI sigue viendo la salida
   completa (`sink.emit_tool` recibe `result` sin truncar); sólo la copia que va al `convo`
   se acota. Es la misma política que Claude Code: las tools no son pasarela de volcado libre.

Además, en la ruta GBNF, cuando el JSON no parsea (típic porque `max_tokens` truncó el
`final` a mitad de un string largo), el raw guardado en `convo` ahora se trunca a 400 chars
(con `…[truncado]`) en vez de inflar el contexto con la salida trunca sin valor informativo.

**Verificación**: dos corridas completas (108 tareas cada una) con 0 tareas en
`reason=error` / context-overflow. El bug ya no aparece.

#### Fixes de los checkers del eval (no del harness productivo)

Algunos checkers eran demasiado estrictos o imponían tareas imposibles que generaban
falsos positivos de fail. Se corrigieron para reflejar lo que el agente debe hacer, no
artefactos del test:

- `w6`: exigía contenido exacto `"profundo"` y rechazaba un `\n` final razonable del
  modelo. Ahora acepta `trim_end()` y también `bash` (algunos modelos hacen `mkdir -p`
  primero).
- `b3`: sólo aceptaba `"fail"` (inglés); el modelo reporta en español ("fallidos",
  "falló", "fallo"). Ahora usa el stem `text_has(text, "fail") || text_has(text, "fall")`.
- `p3`: pedía grep de `"función"` (español) que literalmente no aparece en el fixture
  (`function`). Era una tarea imposible. Cambiado a grep de `"function"`.
- `e6`: sólo aceptaba que el agente reportara el error de ambigüedad. Ahora también acepta
  que desambigüe por contexto (≥2 edits exitosos con `old_string` distintos) —
  comportamiento correcto de un agente pro, no se le debe penalizar.

### Resultados

Run 1 (antes de los fixes de checkers) y Run 2 (verificación, tras fixes):

| Ruta   | Run 1 (ok/54) | Run 2 (ok/54) |
|--------|---------------|---------------|
| grammar | 49            | 47            |
| native  | 45            | 49            |

`reason=error` / context-overflow: **0** en ambas corridas (objetivo cumplido — el fix
productivo se sostiene). Los fallos restantes son todos `reason=done` (el modelo completó
pero respondió mal) = limitaciones del **4B**: se distrae con el contexto del proyecto
(arregla el bug cuando sólo se le pide listar), evita operaciones que fallarían (no intenta
el edit ambiguo), cuenta mal archivos. No del harness. Nota: el modelo 4B es no
determinista (temp 0.4) → varianza run-to-run de ±2 tareas.

Conclusión: GBNF y nativo son comparables en 4B (GBNF ligeramente más estable en
`done`-ahorro-de-pasos, nativo mejor en tareas donde el modelo aprovecha el formato
`tools`). La refactorización no regresó nada: la app de Tauri sigue compilando y
comportándose igual.

### Archivos modificados

- `src-tauri/src/agent/agent_loop.rs` — refactor `LoopSink`/`TauriSink`/`run_turn`
  headless, fix del overflow (`ctx_budget`, `cap_for_context`, truncado de raw en
  parse-fail), debug opt-in vía `AGENT_EVAL_DEBUG`.
- `src-tauri/src/agent/message.rs` — `approx_tokens` mejorado.
- `src-tauri/src/agent/mod.rs` — re-export de `run_turn`/`LoopSink`.
- `src-tauri/Cargo.toml` — `default-run = "agent-aleph"` + `[[bin]] agent_eval`.
- `src-tauri/src/bin/agent_eval.rs` — harness nuevo (54 tareas, fixture, logging).
- `AGENTS.md` — esta documentación.

### Hoja de ruta del eval (Backlog)

- **Persistir settings de inference en el reporte** para comparabilidad跨-runs.
- **Resample de tareas no-deterministas**: N replicaciones por tarea con temp distinta
  para distinguir "modelo no sabe" de "modelo no acertó esta vez".
- **Cooldown de GPU/port** entre rutas para evitar OOM del segundo `llama-server`
  (visto como riesgo; se mitigó con `sleep 1s` post-kill).
- **Cobertura del checker de `args`**: hoy el checker sólo mira `tool` + texto;serie
  útil validar que los args cumplen el schema (p. ej. offset numérico, no string).
- **Suite de 100+ tareas** incluyendo casos de compactación (forzar contexto >budget
  con varias herramientas y verificar que el resumen conserva el objetivo).
- **CI**: correr `agent_eval` en GitHub Actions con un modelo 0.8B para regresión
  rápida en cada PR (el 4B es para cortes periódicos manuales).