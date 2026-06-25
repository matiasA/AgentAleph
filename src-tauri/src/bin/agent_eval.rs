//! Harness de evaluación del harness de agente contra un modelo GGUF real.
//!
//! Levanta `llama-server` (CPU/GPU), construye un proyecto-fixture pequeño y corre
//! un set de ~50 tareas variadas contra el MISMO `run_inner` que usa la app Tauri
//! (vía `agent_aleph_lib::agent::run_turn`), con permisos auto-aprobados. Probamos
//! ambas rutas de tool-calling (`grammar`/GBNF y `native`/--jinja) y logueamos por
//! tarea: pasos, llamadas a herramienta, reason final y veredicto del checker.
//!
//! Uso:
//!   agent_eval --model /ruta/Qwen3.5-4B-Q4_K_M.gguf [--routes grammar,native] \
//!             [--out ./eval-out] [--context-size 8192] [--max-tokens 1024]

use agent_aleph_lib::agent::{run_turn, LoopSink};
use agent_aleph_lib::inference::server::{find_free_port, llama_binary_path, llama_lib_dir, wait_for_ready};
use agent_aleph_lib::settings::Settings;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;

// ----------------------------------------------------------------------
// Parámetros de línea de comandos (parser minimalista, sin deps extra).
// ----------------------------------------------------------------------

struct Cli {
    model: PathBuf,
    routes: Vec<String>,
    out: PathBuf,
    context_size: u32,
    max_tokens: u32,
    threads: u32,
    temperature: f32,
    /// Limita a las primeras N tareas por ruta (0 = todas). Smoke test.
    limit: usize,
    /// Filtra tareas por id (separados por coma). Vacío = todas.
    only: Vec<String>,
}

fn parse_cli() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    let mut model = std::env::var("AGENT_EVAL_MODEL").ok().map(PathBuf::from);
    let mut routes = vec!["grammar".to_string(), "native".to_string()];
    let mut out = PathBuf::from("./eval-out");
    let mut context_size: u32 = 8192;
    let mut max_tokens: u32 = 1024;
    let mut threads: u32 = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(8);
    let mut temperature: f32 = 0.4;
    let mut limit: usize = 0;
    let mut only: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--model" => {
                model = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--routes" => {
                routes = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                i += 2;
            }
            "--out" => {
                out = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--context-size" => {
                context_size = args[i + 1].parse().unwrap_or(context_size);
                i += 2;
            }
            "--max-tokens" => {
                max_tokens = args[i + 1].parse().unwrap_or(max_tokens);
                i += 2;
            }
            "--threads" => {
                threads = args[i + 1].parse().unwrap_or(threads);
                i += 2;
            }
            "--temperature" => {
                temperature = args[i + 1].parse().unwrap_or(temperature);
                i += 2;
            }
            "--limit" => {
                limit = args[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "--only" => {
                only = args[i + 1].split(',').map(|s| s.trim().to_string()).collect();
                i += 2;
            }
            _ => {
                eprintln!("arg desconocido: {a}");
                i += 1;
            }
        }
    }
    let model = model.unwrap_or_else(|| {
        eprintln!("Uso: agent_eval --model /ruta/modelo.gguf [--routes grammar,native] [--out dir]");
        std::process::exit(2);
    });
    Cli {
        model,
        routes,
        out,
        context_size,
        max_tokens,
        threads,
        temperature,
        limit,
        only,
    }
}

// ----------------------------------------------------------------------
// Fixture: proyecto-mini reproducible. Se crea una vez y se copia por tarea.
// ----------------------------------------------------------------------

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).ok();
    fs::write(path, content).unwrap();
}

fn build_fixture(root: &Path) {
    // package.json
    write(
        &root.join("package.json"),
        r#"{
  "name": "fixture-app",
  "version": "1.4.2",
  "description": "Proyecto de ejemplo para evaluar al agente",
  "main": "src/index.js",
  "scripts": {
    "start": "node src/index.js",
    "test": "node tests/math.test.js"
  },
  "author": "agent-aleph",
  "license": "MIT"
}
"#,
    );
    // README
    write(
        &root.join("README.md"),
        r#"# Fixture App

Proyecto de ejemplo para evaluar al agente.

## Scripts
- `npm start`: arranca la app.
- `npm test`: corre los tests de `math`.

## Estructura
- `src/index.js`: punto de entrada.
- `src/math.js`: operaciones matemáticas.
- `src/utils.js`: utilidades varias.
- `tests/math.test.js`: tests de math.

## Bug conocido
`add(a, b)` en `src/math.js` resta en vez de sumar. Hay que arreglarlo.

TODO: añadir soporte para multiplicación y división.
TODO: validar entradas negativas.
"#,
    );
    // AGENTS.md (contexto de proyecto inyectado al system prompt)
    write(
        &root.join("AGENTS.md"),
        "# Fixture — AGENTS.md\n\nEl bug a arreglar está en `src/math.js`: `add` hace una resta.\n",
    );
    // src/index.js
    write(
        &root.join("src/index.js"),
        r#"const { add } = require("./math");
console.log("2 + 3 =", add(2, 3));
"#,
    );
    // src/math.js con el bug
    write(
        &root.join("src/math.js"),
        r#"// Operaciones matematicas basicas.
function add(a, b) {
  return a - b; // BUG: deberia sumar
}
function sub(a, b) {
  return a - b;
}
module.exports = { add, sub };
"#,
    );
    // src/utils.js
    write(
        &root.join("src/utils.js"),
        r#"function isValid(n) {
  return typeof n === "number" && !isNaN(n);
}
const VERSION = "0.9.0";
module.exports = { isValid, VERSION };
"#,
    );
    // tests/math.test.js
    write(
        &root.join("tests/math.test.js"),
        r#"const { add } = require("../src/math");
let pass = 0, fail = 0;
function assert(cond, msg) {
  if (cond) { console.log("ok -", msg); pass++; }
  else { console.log("fail -", msg); fail++; }
}
assert(add(2, 3) === 5, "add(2,3) === 5");
assert(add(-1, 1) === 0, "add(-1,1) === 0");
console.log(`\n${pass} pass, ${fail} fail`);
process.exit(fail ? 1 : 0);
"#,
    );
    // docs/notes.md
    write(
        &root.join("docs/notes.md"),
        "# Notas internas\n\nPrioridad actual: arreglar `add`.\nA revisar: soporte de fracciones.\n",
    );
    // config.json (Vals para ejercitar lectura con offset/limit)
    let big: String = (0..256)
        .map(|i| format!("línea {i}: valor_{i} = {i} * 2 = {}", i * 2))
        .collect::<Vec<_>>()
        .join("\n");
    write(&root.join("data.txt"), &format!("{big}\n"));
    // archivo vacío (para probar read_file de archivo vacío)
    write(&root.join("empty.txt"), "");
    // un .ts para el glob *.ts (no debe matchear en fixture, prueba negativa de glob)
    write(&root.join("src/types.d.ts"), "export type ID = string;\n");
    // //.binario simulado para grep (debe saltarse)
    write(&root.join("blob.bin"), "\x00\x01\x02\x03BINARYNOLEER");
}

// ----------------------------------------------------------------------
// Tareas y checkers.
// ----------------------------------------------------------------------

#[derive(Clone)]
struct Task {
    id: &'static str,
    prompt: &'static str,
    /// Herramientas que esperamos ver usadas (solo para logging/diagnóstico, no estricto).
    #[allow(dead_code)]
    expect_tools: &'static [&'static str],
    /// Checker: recibe (working_dir, texto_final, llamadas) y devuelve None=pass,
    /// Some(motivo)=fail. Las llamadas incluyen los args parseados y el resultado.
    check: fn(&Path, &str, &[ToolRecord]) -> Option<String>,
}

#[derive(Clone, Serialize)]
struct ToolRecord {
    step: usize,
    tool: String,
    args: String,
    result: String,
    is_error: bool,
}

#[derive(Clone, Serialize)]
struct TaskRecord {
    id: String,
    route: String,
    prompt: String,
    steps: usize,
    tool_calls: Vec<ToolRecord>,
    reason: String,
    text: String,
    elapsed_ms: u64,
    ok: bool,
    fail_reason: Option<String>,
}

// Helpers para checkers.
fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_default()
}
fn contains_ins(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}
// ¿El agente llamó a una herramienta concreta al menos una vez (sin error)?
fn called(records: &[ToolRecord], tool: &str) -> bool {
    records.iter().any(|r| r.tool == tool && !r.is_error)
}
// ¿El texto final menciona `needle` (case-insensitive)?
fn text_has(text: &str, needle: &str) -> bool {
    contains_ins(text, needle)
}

fn tasks() -> Vec<Task> {
    use std::path::Path;
    vec![
        // ---- read_file (8) ----
        Task { id: "r1", prompt: "Leé el archivo README.md y decime cuál es el script para correr los tests.", expect_tools: &["read_file"], check: |_, text, recs| {
            if !called(recs, "read_file") { return Some("no llamó read_file".into()); }
            if !text_has(text, "npm test") && !text_has(text, "test") { return Some("no menciona el script de tests".into()); }
            None
        }},
        Task { id: "r2", prompt: "Leé src/math.js y decime cuántas funciones exporta (cuántas `function` hay).", expect_tools: &["read_file"], check: |_, text, recs| {
            if !called(recs, "read_file") { return Some("no llamó read_file".into()); }
            // hay 2 functions (add, sub)
            if !text_has(text, "2") && !text_has(text, "dos") { return Some("no cuenta 2 funciones".into()); }
            None
        }},
        Task { id: "r3", prompt: "Leé data.txt desde la línea 10 (offset=10) y decime qué número de línea arranca la salida.", expect_tools: &["read_file"], check: |_, text, _recs| {
            if !text_has(text, "11") && !text_has(text, "línea 11") { return Some("no identificó offset+1".into()); }
            None
        }},
        Task { id: "r4", prompt: "Leé data.txt con limit=5 y decime si al final te avisa que quedan más líneas.", expect_tools: &["read_file"], check: |_, text, _recs| {
            if !text_has(text, "más") && !text_has(text, "offset") { return Some("no detectó el aviso de paginado".into()); }
            None
        }},
        Task { id: "r5", prompt: "Intentá leer un archivo que NO existe: 'no_existe.xyz'. Reportá el error que devuelva la herramienta.", expect_tools: &["read_file"], check: |_, text, recs| {
            // debe haber un read_file con error
            let saw_err = recs.iter().any(|r| r.tool == "read_file" && r.is_error);
            if !saw_err { return Some("no emitió read_file con error".into()); }
            if !text_has(text, "no") && !text_has(text, "encontr") && !text_has(text, "error") { return Some("no describe el error".into()); }
            None
        }},
        Task { id: "r6", prompt: "Leé el archivo empty.txt y decime qué pasa (¿está vacío?).", expect_tools: &["read_file"], check: |_, text, _recs| {
            if !text_has(text, "vac") && !text_has(text, "empty") && !text_has(text, "sin contenido") { return Some("no indica vacío".into()); }
            None
        }},
        Task { id: "r7", prompt: "Leé docs/notes.md y decime cuál es la prioridad actual.", expect_tools: &["read_file"], check: |_, text, recs| {
            if !called(recs, "read_file") { return Some("no llamó read_file".into()); }
            if !text_has(text, "add") { return Some("no identifica add como prioridad".into()); }
            None
        }},
        Task { id: "r8", prompt: "Leé package.json y decime la versión del proyecto.", expect_tools: &["read_file"], check: |_, text, recs| {
            if !called(recs, "read_file") { return Some("no llamó read_file".into()); }
            if !text_has(text, "1.4.2") { return Some("no reporta la versión 1.4.2".into()); }
            None
        }},

        // ---- list (6) ----
        Task { id: "l1", prompt: "Listá el contenido del directorio raíz del proyecto.", expect_tools: &["list"], check: |_, text, recs| {
            if !called(recs, "list") { return Some("no llamó list".into()); }
            if !text_has(text, "src") || !text_has(text, "package.json") && !text_has(text, "package") { return Some("lista incompleta".into()); }
            None
        }},
        Task { id: "l2", prompt: "Listá el contenido del subdirectorio src.", expect_tools: &["list"], check: |_, text, recs| {
            if !called(recs, "list") { return Some("no llamó list".into()); }
            if !text_has(text, "math.js") && !text_has(text, "math") { return Some("no lista math.js".into()); }
            None
        }},
        Task { id: "l3", prompt: "Listá tests/ y decime qué archivos hay.", expect_tools: &["list"], check: |_, text, recs| {
            if !called(recs, "list") { return Some("no llamó list".into()); }
            if !text_has(text, "math.test.js") && !text_has(text, "math.test") { return Some("no lista el test".into()); }
            None
        }},
        Task { id: "l4", prompt: "¿Qué hay dentro de docs/?", expect_tools: &["list"], check: |_, text, recs| {
            if !called(recs, "list") { return Some("no llamó list".into()); }
            if !text_has(text, "notes.md") && !text_has(text, "notes") { return Some("no lista notes.md".into()); }
            None
        }},
        Task { id: "l5", prompt: "Listá el directorio 'src/index.js' (que es un archivo, no un dir) y reportá el error.", expect_tools: &["list"], check: |_, _text, recs| {
            // esperamos un list con error (no es directorio)
            if !recs.iter().any(|r| r.tool == "list" && r.is_error) { return Some("no hubo list con error".into()); }
            None
        }},
        Task { id: "l6", prompt: "Usá list dos veces: raíz y luego src. Decime cuántos .js hay en src.", expect_tools: &["list"], check: |_, text, recs| {
            if !called(recs, "list") { return Some("no llamó list".into()); }
            // src tiene index.js, math.js, utils.js, types.d.ts => 3 .js
            if !text_has(text, "3") && !text_has(text, "tres") { return Some("no cuenta 3 .js".into()); }
            None
        }},

        // ---- glob (6) ----
        Task { id: "g1", prompt: "Buscá todos los archivos .js con glob (patrón '*.js') y decime cuántos hay.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            // index.js, math.js, utils.js, math.test.js = 4 .js (types.d.ts no es .js)
            if !text_has(text, "4") && !text_has(text, "cuatro") { return Some("no cuenta 4 .js".into()); }
            None
        }},
        Task { id: "g2", prompt: "Buscá archivos con glob 'src/*.js' y listalos.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "math.js") { return Some("no encuentra math.js".into()); }
            None
        }},
        Task { id: "g3", prompt: "Buscá con glob '*.ts' y decime qué archivo encontraste.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "types.d.ts") && !text_has(text, "types.d") { return Some("no encuentra types.d.ts".into()); }
            None
        }},
        Task { id: "g4", prompt: "Buscá con glob '*.bin' y decime si hay resultados.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "blob.bin") && !text_has(text, "blob") { return Some("no encuentra blob.bin".into()); }
            None
        }},
        Task { id: "g5", prompt: "Buscá con glob '*.xyz' y reportá el resultado (no debería haber coincidencias).", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "sin") && !text_has(text, "no") && !text_has(text, "0") { return Some("no reporta ausencia de coincidencias".into()); }
            None
        }},
        Task { id: "g6", prompt: "Con glob, buscá todos los 'package*.json' del proyecto.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "package.json") { return Some("no encuentra package.json".into()); }
            None
        }},

        // ---- grep (6) ----
        Task { id: "p1", prompt: "Con grep, buscá la cadena 'BUG' y decime en qué archivo aparece.", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            if !text_has(text, "math.js") { return Some("no ubica el archivo".into()); }
            None
        }},
        Task { id: "p2", prompt: "Con grep buscá 'TODO' y decime cuántas líneas con TODO hay.", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            // 2 TODOS en README
            if !text_has(text, "2") && !text_has(text, "dos") { return Some("no cuenta 2 TODO".into()); }
            None
        }},
        Task { id: "p3", prompt: "Con grep buscá 'function' (ignore_case=false) y listá las apariciones.", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            if !text_has(text, "math.js") { return Some("no ubica el archivo".into()); }
            None
        }},
        Task { id: "p4", prompt: "Con grep buscá 'zzznoinexistente' y reportá el resultado (debería no haber).", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            if !text_has(text, "sin") && !text_has(text, "no") { return Some("no reporta ausencia".into()); }
            None
        }},
        Task { id: "p5", prompt: "Con grep buscá 'module.exports' y decime en cuántos archivos aparece.", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            if !text_has(text, "math.js") && !text_has(text, "utils.js") { return Some("no encuentra archivos con module.exports".into()); }
            None
        }},
        Task { id: "p6", prompt: "Con grep buscá 'VERSION' y reportá su valor.", expect_tools: &["grep"], check: |_, text, recs| {
            if !called(recs, "grep") { return Some("no llamó grep".into()); }
            if !text_has(text, "0.9.0") { return Some("no reporta 0.9.0".into()); }
            None
        }},

        // ---- write_file (6) ----
        Task { id: "w1", prompt: "Creá un archivo nuevo llamado 'hola.txt' con el contenido 'Hola mundo'.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            if !Path::new(wd).join("hola.txt").exists() { return Some("no se creó hola.txt".into()); }
            if read(&wd.join("hola.txt")) != "Hola mundo" { return Some("contenido incorrecto".into()); }
            None
        }},
        Task { id: "w2", prompt: "Sobrescribí src/math.js con la versión corregida donde add suma (return a + b). Conservá el resto del archivo igual.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            let c = read(&wd.join("src/math.js"));
            if !c.contains("a + b") { return Some("add no suma".into()); }
            if !c.contains("function sub") { return Some("perdió sub".into()); }
            None
        }},
        Task { id: "w3", prompt: "Creá src/mul.js que exporte una función `mul(a, b)` que devuelva a*b, con module.exports.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            let c = read(&wd.join("src/mul.js"));
            if !contains_ins(&c, "mul") { return Some("no define mul".into()); }
            if !c.contains("a * b") { return Some("no multiplica".into()); }
            if !c.contains("module.exports") { return Some("no exporta".into()); }
            None
        }},
        Task { id: "w4", prompt: "Creá un archivo docs/checklist.md con 3 checkboxes (- [ ] ...)", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            let c = read(&wd.join("docs/checklist.md"));
            let count = c.matches("[ ]").count();
            if count < 3 { return Some(format!("solo {} checkboxes", count)); }
            None
        }},
        Task { id: "w5", prompt: "Creá config.json con un objeto {\"env\": \"test\", \"port\": 3000}.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            let c = read(&wd.join("config.json"));
            if !c.contains("test") || !c.contains("3000") { return Some("JSON incompleto".into()); }
            None
        }},
        Task { id: "w6", prompt: "Creá el archivo src/newdir/nested.txt con contenido 'profundo'.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") && !called(recs, "bash") {
                return Some("no llamó write_file ni bash".into());
            }
            let p = wd.join("src/newdir/nested.txt");
            if !p.exists() { return Some("archivo no creado".into()); }
            // Aceptamos un newline final razonable; el modelo suele añadirlo.
            if read(&p).trim_end() != "profundo" { return Some("contenido incorrecto".into()); }
            None
        }},

        // ---- edit (6) ----
        Task { id: "e1", prompt: "Editá src/math.js: cambiá la línea 'return a - b; // BUG: deberia sumar' por 'return a + b;'.", expect_tools: &["edit"], check: |wd, _text, recs| {
            if !called(recs, "edit") { return Some("no llamó edit".into()); }
            let c = read(&wd.join("src/math.js"));
            if !c.contains("a + b") { return Some("add no se corrigió".into()); }
            if c.contains("a - b; // BUG") { return Some("quedó el viejo BUG".into()); }
            None
        }},
        Task { id: "e2", prompt: "Editá src/utils.js para que VERSION sea '1.0.0' en vez de '0.9.0'.", expect_tools: &["edit"], check: |wd, _text, recs| {
            if !called(recs, "edit") { return Some("no llamó edit".into()); }
            let c = read(&wd.join("src/utils.js"));
            if !c.contains("1.0.0") { return Some("no actualizó VERSION".into()); }
            if c.contains("0.9.0") { return Some("quedó el valor viejo".into()); }
            None
        }},
        Task { id: "e3", prompt: "Editá README.md para cambiar 'Bug conocido' por 'Bug arreglado'.", expect_tools: &["edit"], check: |wd, _text, recs| {
            if !called(recs, "edit") { return Some("no llamó edit".into()); }
            let c = read(&wd.join("README.md"));
            if !c.contains("Bug arreglado") { return Some("no editó el título".into()); }
            if c.contains("Bug conocido") { return Some("quedó 'Bug conocido'".into()); }
            None
        }},
        Task { id: "e4", prompt: "Editá package.json cambiando la versión a '2.0.0'.", expect_tools: &["edit"], check: |wd, _text, recs| {
            if !called(recs, "edit") { return Some("no llamó edit".into()); }
            let c = read(&wd.join("package.json"));
            if !c.contains("2.0.0") { return Some("no cambió la versión".into()); }
            None
        }},
        Task { id: "e5", prompt: "Intentá editar src/math.js reemplazando 'ZZZ_NO_EXISTE' por 'x'. Reportá el error.", expect_tools: &["edit"], check: |_, _text, recs| {
            if !recs.iter().any(|r| r.tool == "edit" && r.is_error) { return Some("no hubo edit con error".into()); }
            None
        }},
        Task { id: "e6", prompt: "Intentá editar src/math.js reemplazando la cadena 'function' (que aparece varias veces) por 'fn'. Reportá el error de ambigüedad.", expect_tools: &["edit"], check: |_, _text, recs| {
            // Aceptamos dos comportamientos válidos del agente:
            //  (a) intenta el edit ambiguo y reporta el error (camino que pide la tarea), o
            //  (b) desambigua por contexto (>=2 edits exitosos con old_strings distintos) —
            //      comportamiento correcto de un agente pro, no se le debe penalizar.
            let ambig_err = recs.iter().any(|r| {
                r.tool == "edit" && r.is_error && (contains_ins(&r.result, "veces")
                    || contains_ins(&r.result, "varias") || contains_ins(&r.result, "ambig")
                    || contains_ins(&r.result, "varios") || contains_ins(&r.result, "más de"))
            });
            if ambig_err { return None; }
            let ok_edits: Vec<&ToolRecord> = recs.iter().filter(|r| r.tool == "edit" && !r.is_error).collect();
            if ok_edits.len() >= 2 { return None; }
            Some("no reportó ambigüedad ni desambiguó".into())
        }},

        // ---- bash (6) ----
        Task { id: "b1", prompt: "Con bash ejecutá 'ls' en el directorio del proyecto y reportá lo que veas.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            if !text_has(text, "src") && !text_has(text, "package") { return Some("no lista contenido".into()); }
            None
        }},
        Task { id: "b2", prompt: "Con bash ejecutá 'pwd' y decime cuál es el directorio de trabajo.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            if !text_has(text, "fixture") && !text_has(text, "/") { return Some("no reporta pwd".into()); }
            None
        }},
        Task { id: "b3", prompt: "Con bash ejecutá 'node tests/math.test.js' y decime cuántos tests pasan y cuántos fallan.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            // el test falla porque add resta. Aceptamos "fail"/"fails"/"failed" (EN) o cualquier forma
// del stem español "fall" (falla/fallan/falló/fallidos/fallo).
            let reports_fail = text_has(text, "fail") || text_has(text, "fall");
            if !reports_fail { return Some("no reporta falla".into()); }
            None
        }},
        Task { id: "b4", prompt: "Con bash ejecutá 'echo HOLA' y decime qué imprime.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            if !text_has(text, "hola") { return Some("no reporta HOLA".into()); }
            None
        }},
        Task { id: "b5", prompt: "Con bash contá cuántas líneas tiene README.md (usa wc -l).", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            // debe mencionar un número
            if !text.chars().any(|c| c.is_ascii_digit()) { return Some("no reporta número de líneas".into()); }
            None
        }},
        Task { id: "b6", prompt: "Con bash ejecutá 'grep -rn TODO README.md' y reportá las líneas encontradas.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            if !text_has(text, "todo") && !contains_ins(text, "TODO") { return Some("no reporta TODOS".into()); }
            None
        }},

        // ---- mixtos / realistas (10) ----
        Task { id: "m1", prompt: "Listá el proyecto, después leé package.json y decime la versión. (Usá list y read_file.)", expect_tools: &["list", "read_file"], check: |_, text, recs| {
            if !called(recs, "list") || !called(recs, "read_file") { return Some("no usó ambas herramientas".into()); }
            if !text_has(text, "1.4.2") { return Some("no reporta versión".into()); }
            None
        }},
        Task { id: "m2", prompt: "Buscá todos los .js con glob, después leé cada uno y decime cuál contiene la función 'mul'.", expect_tools: &["glob", "read_file"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            // ninguno tiene mul, debe decir que no found / no encontró
            if !text_has(text, "no") && !text_has(text, "ning") && !text_has(text, "0") { return Some("no concluye ausencia de mul".into()); }
            None
        }},
        Task { id: "m3", prompt: "Encontrá el bug del proyecto (está descripto en README.md y AGENTS.md), leé el archivo correspondiente y decime en qué línea está el bug y cuál es.", expect_tools: &["read_file", "grep"], check: |_, text, recs| {
            if !called(recs, "read_file") { return Some("no usó read_file".into()); }
            if !text_has(text, "math.js") { return Some("no ubica el archivo".into()); }
            if !text_has(text, "a - b") && !text_has(text, "resta") && !text_has(text, "rest") { return Some("no describe el bug".into()); }
            None
        }},
        Task { id: "m4", prompt: "Arreglá el bug: usá edit para que add sume. Después ejecutá los tests con bash y confirmame si ahora pasan.", expect_tools: &["edit", "bash"], check: |wd, text, recs| {
            let edit_ok = recs.iter().any(|r| r.tool == "edit" && !r.is_error);
            let bash_ok = recs.iter().any(|r| r.tool == "bash" && !r.is_error);
            if !edit_ok { return Some("no editó".into()); }
            if !bash_ok { return Some("no corrió tests".into()); }
            let c = read(&wd.join("src/math.js"));
            if !c.contains("a + b") { return Some("no se corrigió add".into()); }
            if !text_has(text, "pass") && !text_has(text, "pasan") && !text_has(text, "ok") { return Some("no confirma tests pasando".into()); }
            None
        }},
        Task { id: "m5", prompt: "¿Hay archivos .ts en el proyecto? Usá glob y respondé sí o no, con el nombre.", expect_tools: &["glob"], check: |_, text, recs| {
            if !called(recs, "glob") { return Some("no llamó glob".into()); }
            if !text_has(text, "types.d.ts") { return Some("no menciona el .ts".into()); }
            None
        }},
        Task { id: "m6", prompt: "Leé src/utils.js, después creá src/version.txt con el valor de VERSION que encontraste.", expect_tools: &["read_file", "write_file"], check: |wd, _text, recs| {
            if !called(recs, "read_file") || !called(recs, "write_file") { return Some("no usó read + write".into()); }
            let c = read(&wd.join("src/version.txt"));
            if !c.contains("0.9.0") { return Some("version.txt no tiene el valor correcto".into()); }
            None
        }},
        Task { id: "m7", prompt: "Con bash ejecutá 'ls -la' y después decime cuántos archivos .md hay en total usando grep sobre esa salida.", expect_tools: &["bash"], check: |_, text, recs| {
            if !called(recs, "bash") { return Some("no llamó bash".into()); }
            // README.md, notes.md => 2
            if !text_has(text, "2") && !text_has(text, "dos") { return Some("no cuenta 2 .md".into()); }
            None
        }},
        Task { id: "m8", prompt: "Contá cuántas veces aparece la palabra 'test' en README.md. Podés leer el archivo y buscar, o usar grep/bash.", expect_tools: &["grep", "read_file"], check: |_, text, recs| {
            // "test" aparece como "tests" en scripts y en structure y en 2 places + test dir => varias
            if !called(recs, "read_file") && !called(recs, "grep") && !called(recs, "bash") { return Some("no inspeccionó README".into()); }
            if !text.chars().any(|c| c.is_ascii_digit()) { return Some("no da un número".into()); }
            None
        }},
        Task { id: "m9", prompt: "Creá el archivo tests/extra.test.js con un assert que valide que 1+1 === 2.", expect_tools: &["write_file"], check: |wd, _text, recs| {
            if !called(recs, "write_file") { return Some("no llamó write_file".into()); }
            let c = read(&wd.join("tests/extra.test.js"));
            if !c.contains("1 + 1") && !c.contains("1+1") { return Some("no valida 1+1".into()); }
            None
        }},
        Task { id: "m10", prompt: "Editá package.json cambiando la versión a '1.5.0', después leélo de vuelta y confirmame el cambio.", expect_tools: &["edit", "read_file"], check: |wd, text, recs| {
            if !called(recs, "edit") || !called(recs, "read_file") { return Some("no usó edit + read".into()); }
            let c = read(&wd.join("package.json"));
            if !c.contains("1.5.0") { return Some("no quedó 1.5.0".into()); }
            if !text_has(text, "1.5.0") { return Some("no confirma el cambio en texto".into()); }
            None
        }},
    ]
}

// ----------------------------------------------------------------------
// Sink que registra todo en memoria para el checker.
// ----------------------------------------------------------------------

struct RecordSink {
    rec: Arc<Mutex<Vec<ToolRecord>>>,
    steps: Arc<Mutex<usize>>,
    tok_buf: Arc<Mutex<String>>,
}

impl RecordSink {
    fn new() -> Self {
        Self {
            rec: Arc::new(Mutex::new(Vec::new())),
            steps: Arc::new(Mutex::new(0)),
            tok_buf: Arc::new(Mutex::new(String::new())),
        }
    }
    fn reset(&self) {
        self.rec.lock().unwrap().clear();
        *self.steps.lock().unwrap() = 0;
        self.tok_buf.lock().unwrap().clear();
    }
    fn tool_records(&self) -> Vec<ToolRecord> {
        self.rec.lock().unwrap().clone()
    }
    fn steps(&self) -> usize {
        *self.steps.lock().unwrap()
    }
    #[allow(dead_code)]
    fn text(&self) -> String {
        self.tok_buf.lock().unwrap().clone()
    }
}

impl LoopSink for RecordSink {
    fn emit_step(&self, _sid: &str, step: usize, _phase: &str) {
        let mut s = self.steps.lock().unwrap();
        if step + 1 > *s {
            *s = step + 1;
        }
    }
    fn emit_token(&self, _sid: &str, token: &str, _is_reasoning: bool) {
        self.tok_buf.lock().unwrap().push_str(token);
    }
    fn emit_tool(
        &self,
        _sid: &str,
        step: usize,
        tool: &str,
        args: &str,
        result: &str,
        is_error: bool,
    ) {
        self.rec.lock().unwrap().push(ToolRecord {
            step,
            tool: tool.to_string(),
            args: args.to_string(),
            result: result.to_string(),
            is_error,
        });
    }
    fn emit_permission(
        &self,
        _sid: &str,
        _rid: &str,
        _tool: &str,
        _args: &str,
        _summary: &str,
    ) {
        // auto-allow: nunca deberíaemitirse.
    }
    fn emit_done(&self, _sid: &str, _text: &str, _reason: &str, _error: Option<&str>) {
        // el resultado lo leemos del return de run_turn.
    }
}

// ----------------------------------------------------------------------
// Lanzamiento de llama-server (espejo de inference::server::start_server sin AppHandle).
// ----------------------------------------------------------------------

struct Server {
    child: Child,
    port: u16,
}

fn spawn_server(model: &Path, settings: &Settings) -> std::io::Result<Server> {
    let bin = llama_binary_path().expect("binario llama-server");
    let lib_dir = llama_lib_dir().expect("lib dir");
    let port = find_free_port();

    let mut cmd = Command::new(&bin);
    cmd.env("LD_LIBRARY_PATH", &lib_dir);
    cmd.arg("--model").arg(model);
    cmd.arg("--port").arg(port.to_string());
    cmd.arg("--host").arg("127.0.0.1");
    cmd.arg("--ctx-size").arg(settings.context_size.to_string());
    cmd.arg("--temp").arg(settings.temperature.to_string());
    cmd.arg("--top-p").arg(settings.top_p.to_string());
    cmd.arg("--repeat-penalty").arg(settings.repeat_penalty.to_string());
    cmd.arg("--threads").arg(settings.threads.to_string());
    if !settings.gpu_layers_auto {
        cmd.arg("--n-gpu-layers").arg(settings.n_gpu_layers.to_string());
    }
    cmd.arg("--flash-attn").arg("auto");
    cmd.arg("--batch-size").arg(settings.n_batch.to_string());
    // `--jinja` solo en la ruta nativa; la gramática (GBNF) usa la plantilla legacy.
    if settings.tool_calling != "grammar" {
        cmd.arg("--jinja");
    }
    if !settings.cache_type_k.is_empty() && settings.cache_type_k != "f16" {
        cmd.arg("--cache-type-k").arg(&settings.cache_type_k);
    }
    if !settings.cache_type_v.is_empty() && settings.cache_type_v != "f16" {
        cmd.arg("--cache-type-v").arg(&settings.cache_type_v);
    }
    if !settings.use_mmap {
        cmd.arg("--no-mmap");
    }
    if settings.use_mlock {
        cmd.arg("--mlock");
    }
    if !settings.device.is_empty() && settings.device != "cpu" && settings.device != "auto" {
        cmd.arg("--device").arg(&settings.device);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());
    cmd.kill_on_drop(true);
    let child = cmd.spawn()?;
    Ok(Server { child, port })
}

async fn pipe_logs(child: &mut Child) {
    use tokio::io::AsyncBufReadExt;
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut buf = tokio::io::BufReader::new(stdout);
            let mut line = String::new();
            loop {
                line.clear();
                if buf.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                let t = line.trim();
                if !t.is_empty() {
                    eprintln!("[llama.out] {}", t);
                }
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut buf = tokio::io::BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                if buf.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                let t = line.trim();
                if !t.is_empty() {
                    eprintln!("[llama.err] {}", t);
                }
            }
        });
    }
}

// ----------------------------------------------------------------------
// Reporte.
// ----------------------------------------------------------------------

#[derive(Serialize)]
struct Report {
    model: String,
    routes: Vec<RouteReport>,
}

#[derive(Serialize, Clone)]
struct RouteReport {
    route: String,
    tasks: Vec<TaskRecord>,
    summary: Summary,
}

#[derive(Serialize, Clone)]
struct Summary {
    total: usize,
    ok: usize,
    fail: usize,
    by_reason: HashMap<String, usize>,
    avg_steps: f64,
    avg_ms: f64,
}

// ----------------------------------------------------------------------
// main.
// ----------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,agent_aleph_lib=info".into()),
        )
        .with_target(false)
        .init();

    let cli = parse_cli();
    if !cli.model.exists() {
        eprintln!("No existe el modelo: {}", cli.model.display());
        std::process::exit(2);
    }
    let active_model = cli
        .model
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("model")
        .to_string();

    fs::create_dir_all(&cli.out).expect("no se pudo crear --out");

    // Cargar settings del usuario y pisar los parámetros del CLI.
    let mut base = Settings::load();
    base.context_size = cli.context_size;
    base.max_tokens = cli.max_tokens;
    base.threads = cli.threads;
    base.temperature = cli.temperature;
    // Para eval queremos inferencia determinista y repetible: top_p 0.9, repeat_penalty 1.05.
    base.top_p = 0.9;
    base.repeat_penalty = 1.05;
    // El thinking apagado gasta menos tokens; lo dejamos como esté en settings (false).

    // Fixture base (prístino), copiado por tarea.
    let tmp = tempfile_dir();
    let pristine = tmp.join("fixture");
    fs::create_dir_all(&pristine).unwrap();
    build_fixture(&pristine);

    let mut tasks = tasks();
    if !cli.only.is_empty() {
        tasks.retain(|t| cli.only.iter().any(|o| o == t.id));
    }
    if cli.limit > 0 && tasks.len() > cli.limit {
        tasks.truncate(cli.limit);
    }
    eprintln!("== agent_eval ==");
    eprintln!("modelo: {} ({active_model})", cli.model.display());
    eprintln!("rutas : {:?}", cli.routes);
    eprintln!("tareas: {} (por ruta)", tasks.len());
    eprintln!("salida: {}", cli.out.display());

    let mut report = Report {
        model: active_model.clone(),
        routes: Vec::new(),
    };

    for route in &cli.routes {
        eprintln!("\n>>> Ruta '{route}': levantando llama-server ...");
        let mut settings = base.clone();
        settings.tool_calling = route.clone();

        let mut server = match spawn_server(&cli.model, &settings) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("No se pudo lanzar llama-server: {e}");
                std::process::exit(3);
            }
        };
        pipe_logs(&mut server.child).await;
        // La carga de modelos grandes excede el timeout corto: aquí damos margen holgado.
        if let Err(e) = wait_for_ready(server.port, Duration::from_secs(180)).await {
            eprintln!("llama-server no respondió: {e}");
            let _ = server.child.kill().await;
            std::process::exit(4);
        }
        eprintln!("llama-server listo en puerto {}", server.port);

        let mut route_report = RouteReport {
            route: route.clone(),
            tasks: Vec::new(),
            summary: Summary {
                total: tasks.len(),
                ok: 0,
                fail: 0,
                by_reason: HashMap::new(),
                avg_steps: 0.0,
                avg_ms: 0.0,
            },
        };

        let sink = Arc::new(RecordSink::new());

        for (idx, task) in tasks.iter().enumerate() {
            // Carpeta de trabajo fresca por tarea (copia del prístino).
            let work = tmp.join(format!("work-{route}-{:03}", idx));
            if work.exists() {
                fs::remove_dir_all(&work).ok();
            }
            copy_dir_all(&pristine, &work).expect("copiar fixture");

            sink.reset();
            let cancel = CancellationToken::new();
            let started = Instant::now();
            eprintln!(
                "  [{route}/{:02}/{:03}] {}",
                idx + 1,
                tasks.len(),
                task.id
            );
            let res = run_turn(
                sink.as_ref(),
                server.port,
                &settings,
                &active_model,
                work.to_str().unwrap(),
                "build",
                task.prompt.to_string(),
                cancel,
            )
            .await;
            let elapsed = started.elapsed().as_millis() as u64;

            let (reason, text) = match res {
                Ok((t, r)) => (r, t),
                Err(e) => ("error".to_string(), e.to_string()),
            };
            let recs = sink.tool_records();
            let fail_reason = (task.check)(&work, &text, &recs);
            let ok = reason == "done" && fail_reason.is_none();
            let steps = sink.steps();

            eprintln!(
                "      -> reason={reason} pasos={steps} tools={} ok={ok} ({}ms){}",
                recs.len(),
                elapsed,
                fail_reason
                    .as_ref()
                    .map(|m| format!(" | FAIL: {m}"))
                    .unwrap_or_default()
            );

            *route_report.summary.by_reason.entry(reason.clone()).or_insert(0) += 1;
            if ok {
                route_report.summary.ok += 1;
            } else {
                route_report.summary.fail += 1;
            }

            route_report.tasks.push(TaskRecord {
                id: task.id.to_string(),
                route: route.clone(),
                prompt: task.prompt.to_string(),
                steps,
                tool_calls: recs,
                reason,
                text,
                elapsed_ms: elapsed,
                ok,
                fail_reason,
            });

            // Guardamos un snapshot en vivo de la ruta (por si crashea/abortan).
            let _ = save_report(&cli.out, &report, route_report.clone(), &tasks);
        }

        // Stats finales de la ruta.
        let n = route_report.tasks.len();
        let steps: usize = route_report.tasks.iter().map(|t| t.steps).sum();
        let ms: u64 = route_report.tasks.iter().map(|t| t.elapsed_ms).sum();
        route_report.summary.avg_steps = if n > 0 { steps as f64 / n as f64 } else { 0.0 };
        route_report.summary.avg_ms = if n > 0 { ms as f64 / n as f64 } else { 0.0 };

        eprintln!(
            "  RESUMEN {route}: ok={} fail={} avg_pasos={:.1} avg_ms={:.0}",
            route_report.summary.ok, route_report.summary.fail, route_report.summary.avg_steps, route_report.summary.avg_ms
        );
        let _ = save_report(&cli.out, &report, route_report.clone(), &tasks);

        // Volcar también los detalles de la ruta (transcripción de tools por tarea) en markdown.
        let _ = save_markdown(&cli.out, route_report.clone());

        report.routes.push(route_report);
        let _ = save_full(&cli.out, &report);

        eprintln!("  matando llama-server ...");
        let _ = server.child.kill().await;
        let _ = server.child.wait().await;
        // pequeño respiro para que libere el puerto/GPU.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    eprintln!("\nListo. Reporte en {}", cli.out.display());
}

// ----------------------------------------------------------------------
// Utilidades FS.
// ----------------------------------------------------------------------

fn tempfile_dir() -> PathBuf {
    // Usamos /tmp/opencode (pre-aprobado) para el fixture y los working dirs.
    let p = PathBuf::from("/tmp/opencode/agent-eval");
    if p.exists() {
        fs::remove_dir_all(&p).ok();
    }
    fs::create_dir_all(&p).unwrap();
    p
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn save_report(out: &Path, _report: &Report, route: RouteReport, _tasks: &[Task]) -> std::io::Result<()> {
    let p = out.join(format!("route-{}.json", route.route));
    let s = serde_json::to_string_pretty(&route).unwrap();
    fs::create_dir_all(out)?;
    fs::write(p, s)
}

fn save_full(out: &Path, report: &Report) -> std::io::Result<()> {
    let p = out.join("report.json");
    fs::write(p, serde_json::to_string_pretty(report).unwrap())
}

fn save_markdown(out: &Path, route: RouteReport) -> std::io::Result<()> {
    let mut s = String::new();
    s.push_str(&format!(
        "# Ruta `{}`\n\nok={} fail={} avg_pasos={:.1} avg_ms={:.0}\n\n",
        route.route, route.summary.ok, route.summary.fail, route.summary.avg_steps, route.summary.avg_ms
    ));
    s.push_str("| id | ok | reason | pasos | tools | fail | prompt |\n");
    s.push_str("|----|----|--------|-------|-------|------|--------|\n");
    for t in &route.tasks {
        let tools: Vec<String> = t.tool_calls.iter().map(|r| {
            if r.is_error { format!("{}!", r.tool) } else { r.tool.clone() }
        }).collect();
        let prompt_short: String = t.prompt.chars().take(60).collect();
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} |\n",
            t.id,
            if t.ok { "✓" } else { "✗" },
            t.reason,
            t.steps,
            tools.join(","),
            t.fail_reason.as_deref().unwrap_or(""),
            prompt_short.replace('|', "/"),
        ));
    }
    s.push_str("\n## Detalle por tarea\n\n");
    for t in &route.tasks {
        s.push_str(&format!("### {} — {}\n- ok: {}\n- reason: {}\n- pasos: {}\n- prompt: {}\n\n",
            t.id, t.route, t.ok, t.reason, t.steps, t.prompt));
        s.push_str("**Tool calls:**\n\n");
        for r in &t.tool_calls {
            s.push_str(&format!(
                "- paso {}: `{}` args=`{}` error={} → `{}`\n",
                r.step, r.tool, r.args, r.is_error, r.result.replace('\n', " ").chars().take(200).collect::<String>()
            ));
        }
        s.push_str(&format!("\n**Texto final:**\n\n```\n{}\n```\n\n", t.text));
    }
    fs::write(out.join(format!("route-{}.md", route.route)), s)
}