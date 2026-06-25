use crate::agent::skills::SkillDoc;
use crate::agent::tools::{ParamType, ToolDoc};
use std::path::Path;

/// Archivos de instrucciones del repo que se inyectan al system prompt, en orden de preferencia
/// (igual que opencode/Claude Code: el proyecto puede guiar al agente).
const PROJECT_DOC_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md"];
/// Tope de caracteres por archivo de contexto para no desbordar el contexto local.
const PROJECT_DOC_MAX: usize = 8000;

/// Lee las instrucciones del proyecto (AGENTS.md/CLAUDE.md) del working dir, si existen.
/// Devuelve el bloque ya formateado para el prompt, o `None` si no hay ninguno.
fn project_context(working_dir: &str) -> Option<String> {
    let root = Path::new(working_dir);
    let mut out = String::new();
    for name in PROJECT_DOC_FILES {
        let path = root.join(name);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        let clipped: String = text.chars().take(PROJECT_DOC_MAX).collect();
        let truncated = if clipped.len() < text.len() {
            "\n[… truncado …]"
        } else {
            ""
        };
        out.push_str(&format!("\n### {name}\n{clipped}{truncated}\n"));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn type_name(t: ParamType) -> &'static str {
    match t {
        ParamType::Str => "string",
        ParamType::Int => "number",
        ParamType::Bool => "bool",
    }
}

/// Firma tipada de una herramienta, p.ej. `read_file(path: string, offset?: number)`.
fn signature(d: &ToolDoc) -> String {
    let ps: Vec<String> = d
        .params
        .iter()
        .map(|p| {
            let sep = if p.required { ": " } else { "?: " };
            format!("{}{sep}{}", p.name, type_name(p.ty))
        })
        .collect();
    format!("{}({})", d.name, ps.join(", "))
}

/// System prompt del agente. `native` selecciona el contrato: en la ruta nativa el modelo usa
/// las herramientas vía tool-calls del propio formato y termina con texto plano; en la ruta
/// GBNF emite un único objeto JSON por paso (forzado por la gramática).
pub fn system_prompt(working_dir: &str, docs: &[ToolDoc], native: bool, skills: &[SkillDoc]) -> String {
    let mut tools_desc = String::new();
    for d in docs {
        tools_desc.push_str(&format!("- {} — {}\n", signature(d), d.description));
    }
    let project = match project_context(working_dir) {
        Some(ctx) => format!(
            "\n\nInstrucciones del proyecto (del repositorio; tienen prioridad sobre tus \
             preferencias por defecto):{ctx}"
        ),
        None => String::new(),
    };
    let skills_block = if skills.is_empty() {
        String::new()
    } else {
        let mut s = String::from(
            "\n\nConocimiento especializado (skills activas; aplícalas cuando sean pertinentes):",
        );
        for sk in skills {
            s.push_str(&format!("\n\n### {}\n{}", sk.name, sk.body));
        }
        s
    };
    let head = format!(
        "Eres un agente de programación que opera localmente en la máquina del usuario.\n\
         Directorio del proyecto: {working_dir}{project}{skills_block}"
    );
    if native {
        format!(
            r#"{head}

Dispones de herramientas para inspeccionar y modificar el proyecto. Invócalas mediante las llamadas a herramienta del sistema (no escribas JSON a mano). Cuando ya tengas la respuesta o hayas completado la tarea, responde en TEXTO PLANO, sin llamar más herramientas.

Herramientas disponibles:
{tools_desc}
Reglas:
- No inventes el contenido de los archivos: léelos con read_file antes de afirmar nada sobre ellos.
- No repitas una herramienta con los mismos argumentos: el resultado sería idéntico.
- En cuanto tengas la información pedida, da tu respuesta final en texto plano. No sigas explorando.
- Nunca anuncies en tu respuesta final un paso que aún no ejecutaste ("a continuación haré...", "luego generaré..."). Si la tarea requiere más pasos, ejecútalos con herramientas ahora; tu respuesta final solo describe lo que ya hiciste.
- Reporta los resultados con honestidad: si algo falló o quedó sin hacer, dilo explícitamente; no afirmes que algo está hecho o verificado si no lo comprobaste con una herramienta."#
        )
    } else {
        format!(
            r#"{head}

Trabajas en pasos. En CADA turno respondes con UN ÚNICO objeto JSON, sin texto antes ni después, con esta forma exacta:
{{"tool": "<nombre>", "args": {{ ... }}}}

Respeta los tipos y los argumentos requeridos de cada herramienta (los opcionales llevan "?").

Herramientas disponibles:
{tools_desc}- final(text: string) — termina la tarea y entrega la respuesta final al usuario.

Reglas:
- Usa exactamente una herramienta por paso. Tras ver su resultado, decide el siguiente paso.
- No inventes el contenido de los archivos: léelos con read_file antes de afirmar nada sobre ellos.
- NUNCA repitas una herramienta con los mismos argumentos: el resultado sería idéntico.
- En cuanto el resultado de una herramienta ya contenga la información pedida, responde de inmediato con la herramienta "final". No sigas explorando si ya tienes la respuesta.
- "final" significa que la tarea YA está terminada. Nunca lo uses para anunciar pasos futuros ("a continuación generaré...", "luego convertiré..."): si faltan pasos, ejecútalos primero con las herramientas correspondientes y solo llama a "final" cuando el trabajo esté hecho de verdad.
- Reporta los resultados con honestidad: si algo falló o quedó sin hacer, dilo explícitamente en el texto de "final"; no afirmes que algo está hecho o verificado si no lo comprobaste con una herramienta."#
        )
    }
}
