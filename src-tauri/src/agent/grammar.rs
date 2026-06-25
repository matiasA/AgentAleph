use crate::agent::tools::{ParamSpec, ParamType, ToolDoc};

/// Primitivas JSON compartidas por todas las reglas de args.
const JSON_PRIMS: &str = r###"string ::= "\"" char* "\"" ws
char ::= [^"\\\x7F\x00-\x1F] | "\\" (["\\/bfnrt] | "u" hex hex hex hex)
hex ::= [0-9a-fA-F]
number ::= "-"? ("0" | [1-9] [0-9]*) ("." [0-9]+)? ws
boolean ::= ("true" | "false") ws
ws ::= [ \t\n]*
"###;

// Tokens literales GBNF para la estructura JSON.
fn lit(s: &str) -> String {
    // Literal GBNF que coincide con el token JSON string «"s"» (con comillas).
    format!("\"\\\"{s}\\\"\"")
}
const LB: &str = "\"{\"";
const RB: &str = "\"}\"";
const COLON: &str = "\":\"";
const COMMA: &str = "\",\"";

fn type_rule(t: ParamType) -> &'static str {
    match t {
        ParamType::Str => "string",
        ParamType::Int => "number",
        ParamType::Bool => "boolean",
    }
}

fn pair(p: &ParamSpec) -> String {
    format!("{} ws {COLON} ws {}", lit(p.name), type_rule(p.ty))
}

/// Regla de gramática para el objeto `args` de una herramienta, según su esquema.
fn args_object(params: &[ParamSpec]) -> String {
    if params.is_empty() {
        return format!("{LB} ws {RB} ws");
    }
    let required: Vec<&ParamSpec> = params.iter().filter(|p| p.required).collect();
    let optional: Vec<&ParamSpec> = params.iter().filter(|p| !p.required).collect();

    let mut body = String::new();
    if !required.is_empty() {
        let reqs: Vec<String> = required.iter().map(|p| pair(p)).collect();
        body.push_str(&reqs.join(&format!(" ws {COMMA} ws ")));
        // Cada opcional, en orden, precedido por coma y entre paréntesis opcionales.
        for p in &optional {
            body.push_str(&format!(" ( ws {COMMA} ws {} )?", pair(p)));
        }
    } else {
        // Solo opcionales: el primero ancla el grupo; los siguientes son anidados-opcionales.
        let mut grp = pair(optional[0]);
        for p in &optional[1..] {
            grp.push_str(&format!(" ( ws {COMMA} ws {} )?", pair(p)));
        }
        body.push_str(&format!("( {grp} )?"));
    }
    format!("{LB} ws {body} ws {RB} ws")
}

/// Genera una gramática GBNF que liga CADA nombre de herramienta a su esquema de args exacto,
/// de modo que el modelo no puede emitir argumentos inválidos. Incluye la pseudo-herramienta
/// `final` con `{ "text": string }`.
pub fn tool_call_grammar(docs: &[ToolDoc]) -> String {
    let mut rules = String::new();
    let mut alts: Vec<String> = Vec::new();

    let emit = |name: &str, params: &[ParamSpec], rules: &mut String, alts: &mut Vec<String>| {
        // Los identificadores de regla GBNF no admiten '_' (solo letras, dígitos y '-').
        let rname = name.replace('_', "-");
        let call = format!("{rname}-call");
        let args = format!("{rname}-args");
        alts.push(call.clone());
        rules.push_str(&format!(
            "{call} ::= {LB} ws {tool_k} ws {COLON} ws {tool_v} ws {COMMA} ws {args_k} ws {COLON} ws {args} ws {RB} ws\n",
            tool_k = lit("tool"),
            tool_v = lit(name),
            args_k = lit("args"),
        ));
        rules.push_str(&format!("{args} ::= {}\n", args_object(params)));
    };

    for d in docs {
        emit(d.name, &d.params, &mut rules, &mut alts);
    }
    emit(
        "final",
        &[ParamSpec::req("text", ParamType::Str)],
        &mut rules,
        &mut alts,
    );

    let root = format!("root ::= ( {} ) ws\n", alts.join(" | "));
    format!("{root}{rules}{JSON_PRIMS}")
}
