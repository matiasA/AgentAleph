use crate::agent::tools::{ParamType, ToolDoc};

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

/// System prompt del agente: describe el contrato JSON y las herramientas disponibles.
pub fn system_prompt(working_dir: &str, docs: &[ToolDoc]) -> String {
    let mut tools_desc = String::new();
    for d in docs {
        tools_desc.push_str(&format!("- {} — {}\n", signature(d), d.description));
    }
    format!(
        r#"Eres un agente de programación que opera localmente en la máquina del usuario.
Directorio del proyecto: {working_dir}

Trabajas en pasos. En CADA turno respondes con UN ÚNICO objeto JSON, sin texto antes ni después, con esta forma exacta:
{{"tool": "<nombre>", "args": {{ ... }}}}

Respeta los tipos y los argumentos requeridos de cada herramienta (los opcionales llevan "?").

Herramientas disponibles:
{tools_desc}- final(text: string) — termina la tarea y entrega la respuesta final al usuario.

Reglas:
- Usa exactamente una herramienta por paso. Tras ver su resultado, decide el siguiente paso.
- No inventes el contenido de los archivos: léelos con read_file antes de afirmar nada sobre ellos.
- NUNCA repitas una herramienta con los mismos argumentos: el resultado sería idéntico.
- En cuanto el resultado de una herramienta ya contenga la información pedida, responde de inmediato con la herramienta "final". No sigas explorando si ya tienes la respuesta."#
    )
}
