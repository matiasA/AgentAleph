# Plan: Núcleo de Agente Profesional sobre Clon Codex

> Objetivo: convertir Clon Codex (gestor de modelos locales tipo LM Studio) en un
> **agente de codificación de nivel profesional** comparable a opencode / Claude Code,
> ejecutándose **100% local** sobre llama.cpp.

## Estado de avance

- ✅ **Fase 1** — Loop de agente + gramática GBNF + `read_file` + UI de pasos. Validado con 0.8B.
- ✅ **Fase 2** — Refactor a `trait Tool` + `Registry`; herramientas `list`/`glob`/`grep`/`write_file`/`edit`/`bash`; permisos allow/ask/deny con round-trip a la UI; modos build/plan.
- ✅ **Fase 3** — Detección de bucles (nudge + abort), gestión/recorte de contexto, cap de tokens por paso, prompt reforzado. Validado: corta bucles y no desborda contexto.
- 🚧 **Fase 4** — Hecho: optimización de runtime (caché KV cuantizada, batch, mmap/mlock); carpetas de modelos externas; **persistencia de sesiones con memoria entre turnos**. Pendiente: subagentes, métricas, speculative decoding.
- 🚧 **Fase 5 — Profesionalización del harness (post-review)** — ver §14.

## 14. Fase 5 — Profesionalización del harness (backlog priorizado)

Brechas detectadas en revisión externa y **verificadas contra el código** (2026-06-24).
Reordenadas por dependencia estructural, no solo por impacto: el modelo de mensaje rico es
el habilitador que destraba casi todo lo demás, así que va primero.

### Hecho

- ✅ **Schema por herramienta + GBNF por-tool + validación de args**. `params()` en el
  `trait Tool` (`tools/mod.rs:70`), gramática ligada al schema exacto (`grammar.rs:64`) y
  validación previa como red (`tools/mod.rs:112`). Es el diferenciador local-first y ya está.
- ✅ **Prioridad 1 — Mensaje rico (`agent/message.rs`).** `enum Role{System,User,Assistant,
  Tool}` + `AgentMsg{tool_name, tool_call_id, tool_calls, is_error, harness}` y constructores
  semánticos. Los nudges, errores de parseo y el marcador de compactación pasan a rol `system`
  (antes `user`); los resultados son mensajes `Tool` de primera clase. Capa `to_wire()` que
  aísla el formato de cable: en la ruta sin `--jinja` (plantilla legacy) los `Tool` se
  renderizan como `user` con el encuadre probado, porque el rol `tool` no es seguro en
  Mistral/Gemma del catálogo — el cambio a rol `tool` real queda en un único punto para la
  prioridad 3 (que requiere `--jinja`). Frontend (`types.ts`, `AgentView.reconstruct`)
  sincronizado y compatible con sesiones antiguas.
- ✅ **Prioridad 2 — Compactación con resumen (`agent/context.rs`).** `maybe_compact` dispara
  al cruzar el presupuesto: conserva cabecera (system prompt + tarea) y la cola reciente,
  resume el medio con una llamada dedicada al modelo (no-stream, sin gramática, temp baja,
  thinking off) y lo sustituye por un único resumen. `budget_view` queda como red de
  seguridad. Cancelable.

- ✅ **Prioridad 3 — Tool-calling nativo.** `--jinja` en `inference/server.rs` (salvo modo
  `grammar`); `Registry::openai_tools()` emite el esquema `tools`; `infer_step` bifurca por
  ruta y acumula `delta.tool_calls` del stream (soporta N llamadas/paso); `to_wire_native`
  usa rol `tool` + `tool_call_id` y asistente con `tool_calls`. **Selección por capacidad**:
  `Settings.tool_calling` = `auto`|`native`|`grammar`; en `auto`, heurística por familia/tamaño
  (`use_native_tools`/`auto_native`) — familias fiables → nativo, modelos chicos → GBNF. System
  prompt mode-aware (`prompt.rs`). Lógica por-llamada unificada en `process_call` (reduce
  `run_inner`). UI: selector en Ajustes; `AgentView.reconstruct`/done deduplican el texto final.

### Prioridad (orden de ejecución)

1. ✅ Mensaje rico — hecho (ver arriba).
2. ✅ Compactación con resumen — hecho (ver arriba).
3. ✅ Tool-calling nativo + selección por capacidad — hecho (ver arriba).

### Baratos / seguridad (en paralelo)

- ✅ **`bash`: blacklist de comandos destructivos** + timeout 60s + truncado (`bash.rs:11`).
  Pendiente: aislamiento de red por defecto.
- ✅ **Inyección de contexto de proyecto**: `prompt.rs` lee AGENTS.md/CLAUDE.md del working
  dir (cap 8000 chars c/u) y los inyecta como «Instrucciones del proyecto» con prioridad.
- ✅ **Detección de bucles consciente de estado**: tras una ejecución exitosa Write/Exec se
  resetea el contador de firmas (`agent_loop.rs`), porque una mutación del mundo invalida la
  evidencia previa de bucle (releer un archivo tras modificarlo deja de marcarse).
- **grep con regex / ripgrep**: hoy es `walk_files` + `contains` literal recorriendo todo el
  árbol sin `.gitignore` (`grep.rs:40`).

### Panel del agente (Skills / Conexiones / Contexto)

- ✅ **Skills** — paquetes de instrucciones+recursos (`agent/skills.rs`): carpetas
  `data_dir/agent-aleph/skills/<slug>/SKILL.md` (frontmatter + cuerpo), estado activo en
  `enabled.json`. Comandos list/create/import/delete/toggle/read. Las skills *activas* se
  inyectan en el system prompt bajo «Conocimiento especializado» (cap 6000 chars). UI funcional
  en `AgentPanel` (lista, toggle, crear/importar/borrar).
- ✅ **Contexto adjunto** — `read_context_file` + store compartido `agentContext.svelte.ts`;
  `AgentPanel` adjunta archivos/texto, `AgentView.send` antepone el bloque al input del turno.
- ⏳ **Conexiones** (GitHub/Google) — maqueta «próximamente» en el panel; la integración real
  (red + OAuth/token, fuera del local-first actual) queda pendiente.

### Mayores (acercan a opencode-local)

- Subagentes (Task/`@general`), MCP, slash commands (`/commit`, `/review`, `/compact`),
  diff real en la UI de permisos, métricas (tok/s, pasos, contexto usado).

### Notas de calidad de código (refactor oportunista)

- ✅ `run_inner` aligerado: la lógica por-llamada (validar/bucle/permiso/ejecutar) vive ahora
  en `process_call`, compartida por ambas rutas. Queda margen para extraer el armado de la
  petición y el manejo del stream si crece más.
- `budget_view` usa `view.remove(2)` en bucle (O(n²)); irrelevante en sesiones chicas pero se
  reemplaza al implementar la compactación (prioridad 2).
- `walk_files` relee el árbol entero (hasta 5000 archivos, depth 12) en cada grep/glob sin
  caché ni `.gitignore`.

---

## 1. Visión

Unir las dos mitades:

- **Mitad "LM Studio"** (ya existe): catálogo, descargas HF, llama-server, offload GPU,
  ajustes de inferencia. → `models/`, `inference/`, `settings.rs`.
- **Mitad "Codex / opencode"** (a construir): un **harness agéntico** con loop de
  herramientas, permisos, sesiones, gestión de contexto y una UI de pasos.

El producto es el **harness**. El runtime de modelos es el habilitador. La calidad final
depende de dos ejes que avanzan en paralelo:

1. **Capacidad del modelo local** (que entre uno suficientemente bueno y corra rápido).
2. **Robustez del harness** (que exprima al máximo a un modelo imperfecto).

### Principio rector frente a modelos locales débiles

A diferencia de Claude Code (modelo frontera detrás), aquí el modelo **falla más**: se
pierde, repite herramientas, alucina formato. El harness debe **compensar por diseño**:

- **Salida restringida por gramática (GBNF)** → tool-calls siempre sintácticamente válidos.
- **Loop defensivo** → topes de turnos, detección de bucles, validación de argumentos.
- **Contexto agresivamente gestionado** → el contexto local es pequeño y caro.
- **Prompts y herramientas de alta señal** → menos ambigüedad = menos errores del modelo.

---

## 2. Estado actual (línea base)

| Capa | Archivo | Estado |
|------|---------|--------|
| Runtime modelos | `inference/server.rs` | ✅ Sólido. Faltan palancas (KV-cache quant, mmap/mlock). |
| Chat streaming | `chat/session.rs` | ⚠️ One-shot. Solo acumula `content`/`reasoning_content` como texto. **Sin tool-calls, sin loop.** |
| Estado | `state.rs` | ⚠️ `ChatSession` solo guarda `Vec<ChatMsg>{role,content}`. Sin tool-calls, sin tipos de mensaje ricos. |
| IPC | `commands.rs` / `api.ts` | ✅ Patrón limpio. Hay que añadir comandos de agente. |
| Frontend | `ChatView.svelte` | ⚠️ Chat plano. Necesita vista de pasos de agente. |

**Brecha central:** existe el runtime, falta TODO el harness agéntico.

---

## 3. Arquitectura objetivo

### 3.1 Diagrama de módulos (backend Rust)

```
src-tauri/src/
├── agent/                  ← NUEVO: el harness
│   ├── mod.rs
│   ├── loop.rs             # bucle de agente (orquestador)
│   ├── message.rs          # tipos de mensaje ricos (tool_call, tool_result, …)
│   ├── tools/
│   │   ├── mod.rs          # trait Tool + registro
│   │   ├── read_file.rs
│   │   ├── write_file.rs
│   │   ├── edit.rs
│   │   ├── bash.rs
│   │   ├── grep.rs
│   │   ├── glob.rs
│   │   └── list.rs
│   ├── grammar.rs          # genera GBNF desde los JSON schemas de las tools
│   ├── permissions.rs      # política allow/ask/deny + eventos de confirmación
│   ├── context.rs          # tokenización aprox., truncado, compactación
│   ├── prompt.rs           # system prompt del agente + plantillas de tool
│   └── parser.rs           # parseo de tool_calls del stream (nativo o por gramática)
├── inference/              # (existente) + nuevas palancas de optimización
├── chat/                   # (existente) chat simple — se mantiene como modo "ask"
├── models/                 # (existente)
└── …
```

### 3.2 Capas conceptuales (alineado con opencode / Claude Code)

1. **Provider** — abstracción sobre el backend de inferencia. Hoy: llama-server local.
   Diseñar el trait para poder añadir otros (OpenAI-compat remoto) sin tocar el loop.
2. **Session** — conversación con estado: historial de mensajes ricos, modo (build/plan),
   modelo activo, working directory, presupuesto de contexto.
3. **Agent / Mode** — política de comportamiento. Mínimo dos como opencode:
   - **build**: acceso completo (lee, escribe, ejecuta).
   - **plan**: solo lectura, niega edits, pide permiso para bash.
4. **Tool** — capacidad ejecutable con JSON schema, validación y ejecutor.
5. **Permission** — capa transversal allow/ask/deny por herramienta y por patrón.
6. **Loop** — el motor que une todo (ver §4).

---

## 4. El loop de agente (corazón del sistema)

```
fn run_agent_turn(session, user_input):
    session.push(user_message(user_input))
    for step in 0..MAX_STEPS:                 # tope duro (p.ej. 25)
        request  = build_request(session)     # mensajes + tools + grammar
        response = stream_inference(request)   # SSE → emite tokens a la UI
        tool_calls = parse_tool_calls(response)

        if tool_calls.is_empty():              # el modelo respondió texto final
            session.push(assistant_message(response.text))
            return Done

        session.push(assistant_message_with_tool_calls(response))
        for call in tool_calls:
            if not validate(call):             # args inválidos → feedback al modelo
                session.push(tool_error(call, "argumentos inválidos: …"))
                continue
            decision = permissions.check(call) # allow / ask(UI) / deny
            if decision == Deny:
                session.push(tool_error(call, "denegado por política"))
                continue
            if decision == Ask:
                decision = await_user_confirmation(call)   # evento → UI → respuesta
            result = execute(call)             # ejecutor de la tool
            session.push(tool_result(call, result))

        if detect_loop(session):               # misma tool+args repetida N veces
            session.push(system_note("posible bucle; replantea el enfoque"))

        context.compact_if_needed(session)     # gestión de presupuesto
    return MaxStepsReached
```

**Decisiones clave:**

- **Streaming + loop conviven**: cada paso transmite tokens a la UI (`agent://token`),
  pero el backend retiene la respuesta para parsear tool-calls al cerrar el paso.
- **Tool-calling: dos rutas.**
  - **Ruta A (preferida si el modelo la soporta):** `tools` estilo OpenAI + `--jinja` en
    llama-server, parseando `delta.tool_calls`.
  - **Ruta B (universal, para modelos débiles):** **gramática GBNF** que obliga al modelo
    a emitir un JSON de tool-call válido. Es la red de seguridad que hace usable un 14B.
  - El harness elige ruta por capacidad detectada del modelo (config en el catálogo).
- **Cancelación**: reutilizar `CancellationToken` existente (`state.cancel_tokens`),
  extendido para abortar a mitad de un paso o de la ejecución de una tool.

---

## 5. Herramientas (set inicial estilo Claude Code / opencode)

| Tool | Descripción | Permiso por defecto | Modo plan |
|------|-------------|---------------------|-----------|
| `read_file` | Lee archivo (con offset/limit, paginado) | allow | allow |
| `list` / `glob` | Lista directorio / busca por patrón | allow | allow |
| `grep` | Búsqueda de contenido (ripgrep si está) | allow | allow |
| `write_file` | Crea/sobrescribe archivo | ask | **deny** |
| `edit` | Reemplazo exacto de string en archivo | ask | **deny** |
| `bash` | Ejecuta comando shell | ask | **ask** |

**Diseño de cada tool:**

- Trait común:
  ```rust
  #[async_trait]
  trait Tool {
      fn name(&self) -> &str;
      fn description(&self) -> &str;
      fn schema(&self) -> serde_json::Value;   // JSON Schema de args
      async fn execute(&self, args: Value, ctx: &ToolCtx) -> AppResult<ToolOutput>;
  }
  ```
- `edit` con la semántica de Claude Code: `old_string` único, falla si no coincide o es
  ambiguo. Más fiable que diffs para modelos locales.
- `read_file` con límite de líneas/bytes y numeración → controla gasto de contexto.
- `bash`: timeout, captura stdout/stderr, working dir de la sesión, truncado de salida.
- Salidas siempre **truncadas y resumidas** antes de volver al contexto.

---

## 6. Seguridad y permisos

Esto ejecuta shell y escribe en disco → es la parte peligrosa.

- **Working directory de sesión**: el usuario elige una carpeta-proyecto; las tools de
  archivo se restringen a ella (rechazo de paths fuera del root salvo allow explícito).
- **Política de tres niveles** por tool y por patrón (`allow` / `ask` / `deny`),
  configurable en Ajustes y override por modo (build/plan).
- **Confirmación en UI**: cuando la decisión es `ask`, el backend emite
  `agent://permission-request` con la acción concreta; la UI muestra diff/comando y el
  usuario aprueba/rechaza → `respond_permission`.
- **Sin red por defecto** para `bash` (documentar; el sandbox real de red es fase posterior).
- **Lista negra de comandos** destructivos con confirmación reforzada.

---

## 7. Gestión de contexto (crítico en local)

Los modelos locales tienen contexto pequeño (4k–32k) y cada token cuesta velocidad.

- **Contador de tokens aproximado** por mensaje (heurística por caracteres o tokenizer
  ligero) para conocer el presupuesto.
- **Truncado de salidas de tools** (p.ej. máx N líneas, "… [recortado]").
- **Compactación**: al acercarse al límite de `context_size`, resumir los turnos antiguos
  en un mensaje de sistema, conservando objetivos y decisiones.
- **Lectura paginada de archivos** en vez de volcar archivos enteros.
- Mostrar en la UI el uso de contexto (barra), como hace Claude Code.

---

## 8. Optimización del runtime (habilitador — corre en paralelo)

Para que entre un modelo lo bastante capaz y rápido (ver conversación previa):

- **Exponer en `Settings` + UI**: `cache_type_k`, `cache_type_v` (q8_0/q4_0),
  `no_mmap`, `mlock`, `n_batch`, `tensor_split`, `model_draft` (speculative decoding).
- **Auto-cálculo de `n_gpu_layers`** desde `list_gpu_devices()` (VRAM libre) en vez de 99 fijo.
- **Sugerencia de cuantización** en el catálogo según RAM/VRAM detectada (priorizar IQ3/IQ4
  y modelos **MoE** —Qwen3-MoE, etc.— que activan pocos parámetros por token).
- Estos cambios son aditivos sobre `inference/server.rs:54` (`start_server`).

---

## 9. Cambios de IPC y tipos

**Nuevos comandos** (`commands.rs` + registro en `lib.rs` + wrappers en `api.ts`):

- `agent_send(session_id, working_dir, mode, input)` — inicia/continúa un turno de agente.
- `agent_stop(session_id)` — cancela (sobre `cancel_tokens`).
- `respond_permission(request_id, decision)` — respuesta a `ask`.
- `set_agent_mode(session_id, mode)` — build/plan.
- `list_tools()` — introspección de herramientas disponibles.

**Nuevos eventos** (`api.ts` listeners):

- `agent://token` — tokens en streaming (con flag reasoning).
- `agent://step` — inicio/fin de paso, tool-call ejecutándose.
- `agent://tool-result` — resultado de una tool (para render en UI).
- `agent://permission-request` — solicitud de confirmación.
- `agent://done` — turno terminado (motivo: done / max-steps / cancelled / error).

**Tipos** (`message.rs` ↔ `types.ts`): modelo de mensaje rico
`{ role, content, tool_calls?, tool_call_id?, tool_name? }`, sustituyendo/extendiendo el
`ChatMsg{role,content}` actual para el modo agente (el chat simple puede seguir usando el viejo).

---

## 10. Frontend (UI de agente)

- **`AgentView.svelte`** (evolución de `ChatView`): render de pasos —mensaje del modelo,
  tool-calls con sus argumentos, resultados colapsables, diffs de `edit`/`write`.
- **Selector de modo** build/plan (tipo Tab de opencode).
- **Modal/inline de permisos** que muestra el comando o el diff antes de aprobar.
- **Selector de carpeta-proyecto** (working dir de sesión) vía plugin `dialog` (ya presente).
- **Indicador de contexto** (uso/límite) y de pasos del turno.
- Mantener el chat simple existente como modo "Ask" sin herramientas.

---

## 11. Hoja de ruta por fases

### Fase 0 — Cimientos (runtime + tipos)
- Palancas de optimización en `Settings`/`server.rs` (KV-cache quant, mmap/mlock, batch).
- Auto-cálculo de `n_gpu_layers`.
- Definir `agent/message.rs` (tipos de mensaje ricos) y reflejarlos en `types.ts`.

### Fase 1 — Loop mínimo end-to-end (PoC)
- `agent/loop.rs` con tope de pasos.
- 1 herramienta: `read_file`. Trait `Tool` + registro.
- Tool-calling vía **gramática GBNF** (`grammar.rs`, ruta universal).
- Comando `agent_send` + eventos `agent://*`.
- `AgentView` básica que muestra pasos. **Criterio de éxito:** el agente lee un archivo
  que le pides y responde usándolo.

### Fase 2 — Conjunto de herramientas + permisos
- `list`, `glob`, `grep`, `write_file`, `edit`, `bash`.
- `permissions.rs` con allow/ask/deny + modal de confirmación en UI.
- Working dir de sesión y restricción de paths.
- Modos build/plan.

### Fase 3 — Robustez (lo que separa de un juguete)
- Gestión de contexto: conteo, truncado, compactación, lectura paginada.
- Detección de bucles y validación de argumentos con feedback al modelo.
- Ruta A de tool-calling nativo (`tools`/`--jinja`) además de la gramática.
- Prompts afinados por familia de modelo.

### Fase 4 — Pulido profesional
- Speculative decoding y sugerencias de modelo/cuantización por hardware.
- Persistencia de sesiones (historial en disco).
- Subagentes (búsqueda/tareas complejas) estilo opencode `@general`.
- Métricas: tok/s, pasos, contexto usado.

---

## 12. Riesgos y mitigaciones

| Riesgo | Impacto | Mitigación |
|--------|---------|-----------|
| Modelo local demasiado débil para agente | **Alto** (mata el producto) | Gramática GBNF, prompts de alta señal, foco en Coder/MoE 14B–32B, optimización de runtime para que entren modelos mayores |
| Bucles / no termina | Medio | Tope de pasos, detección de repetición, notas de sistema |
| Contexto desbordado | Medio | Truncado, compactación, lectura paginada |
| Ejecución peligrosa (bash/write) | **Alto** (seguridad) | Permisos ask/deny, working dir restringido, lista negra, modo plan por defecto |
| Tool-calls mal formados | Medio | Gramática + validación de schema + feedback de error al modelo |
| Latencia (loop multi-paso lento) | Medio | KV-cache reuse, speculative decoding, modelos MoE |

---

## 13. Próximo paso propuesto

Arrancar por **Fase 1** (PoC end-to-end): loop mínimo + `read_file` + gramática GBNF +
`AgentView` básica. Es el menor esfuerzo que demuestra que el concepto funciona con un
modelo local, y a partir de ahí se itera con confianza.
