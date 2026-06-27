use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Rol semántico de un mensaje en la conversación del agente.
///
/// A diferencia del `ChatMsg{role,content}` plano del chat simple, distingue los
/// resultados de herramienta (`Tool`) y las notas/errores del propio harness (`System`
/// con `harness = true`) del verdadero diálogo usuario/asistente. Esto habilita:
/// render diferenciado en la UI, compactación que sabe qué es cada mensaje, y la futura
/// ruta de tool-calling nativo (que necesita rol `tool` + `tool_call_id`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Una llamada a herramienta emitida por el asistente. Hoy el harness usa la ruta GBNF
/// (una sola llamada por paso, parseada del contenido); este tipo prepara la ruta nativa
/// (`delta.tool_calls`) sin volver a tocar el modelo de mensajes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// Mensaje rico de la conversación del agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMsg {
    pub role: Role,
    pub content: String,
    /// Nombre de la herramienta para mensajes con rol `Tool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// Enlaza un resultado de herramienta con la llamada que lo originó (ruta nativa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Llamadas a herramienta emitidas por el asistente (ruta nativa).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// El mensaje representa un error (resultado de tool fallido o nota de error del harness).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
    /// Generado por el harness (nudge, error de parseo, marcador de compactación), no por el
    /// usuario real. La UI puede distinguirlo y la compactación tratarlo aparte.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub harness: bool,
}

impl AgentMsg {
    /// System prompt del agente (posición 0 de la conversación).
    pub fn system(content: impl Into<String>) -> Self {
        Self::bare(Role::System, content.into())
    }

    /// Mensaje real del usuario.
    pub fn user(content: impl Into<String>) -> Self {
        Self::bare(Role::User, content.into())
    }

    /// Respuesta textual del asistente (en la ruta GBNF, el JSON crudo de la tool-call).
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::bare(Role::Assistant, content.into())
    }

    /// Resultado de ejecutar una herramienta (éxito o error).
    pub fn tool_result(tool: &str, content: impl Into<String>, is_error: bool) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_name: Some(tool.to_string()),
            tool_call_id: None,
            tool_calls: Vec::new(),
            is_error,
            harness: false,
        }
    }

    /// Turno del asistente que emite llamadas a herramienta (ruta nativa).
    pub fn assistant_calls(content: impl Into<String>, calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_name: None,
            tool_call_id: None,
            tool_calls: calls,
            is_error: false,
            harness: false,
        }
    }

    /// Enlaza este mensaje (resultado de tool) con la llamada que lo originó (ruta nativa).
    pub fn with_call_id(mut self, id: Option<String>) -> Self {
        self.tool_call_id = id;
        self
    }

    /// Nota dirigida del harness al modelo (p.ej. nudge anti-bucle). No es el usuario.
    pub fn harness_note(content: impl Into<String>) -> Self {
        let mut m = Self::bare(Role::System, content.into());
        m.harness = true;
        m
    }

    /// Error del harness sobre la respuesta del modelo (p.ej. JSON no parseable).
    pub fn harness_error(content: impl Into<String>) -> Self {
        let mut m = Self::bare(Role::System, content.into());
        m.harness = true;
        m.is_error = true;
        m
    }

    fn bare(role: Role, content: String) -> Self {
        Self {
            role,
            content,
            tool_name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
            is_error: false,
            harness: false,
        }
    }
}

/// Mensaje en el formato que acepta el endpoint `/v1/chat/completions` de llama-server.
#[derive(Serialize)]
pub struct WireMsg {
    pub role: &'static str,
    pub content: String,
}

/// Convierte la conversación rica al formato de cable para la ruta GBNF actual
/// (llama-server **sin** `--jinja`, plantilla legacy).
///
/// El servidor legacy solo renderiza con fiabilidad los roles `system`/`user`/`assistant`
/// en todas las plantillas del catálogo (Mistral/Gemma no tienen ranura para un rol `tool`
/// arbitrario), por eso aquí:
/// - `Tool` se renderiza como `user` con el encuadre `«Resultado de {tool}:»` (formato
///   probado, no cambia la calidad para el modelo) — pero internamente sigue siendo un
///   mensaje `Tool` de primera clase para la UI, la compactación y la ruta nativa.
/// - Las notas/errores del harness (`harness == true`) van como `user` con una etiqueta
///   `[Aviso del sistema, no es el usuario]`, nunca como `system`: muchas plantillas Jinja
///   (Gemma incluida) exigen que el ÚNICO mensaje `system` sea el primero del array y
///   lanzan una excepción si aparece otro más adelante.
///
/// Cuando se active la ruta nativa (con `--jinja`), el rol `Tool` pasará a emitirse como
/// `tool` + `tool_call_id` real; este es el único punto a cambiar.
pub fn to_wire(msgs: &[AgentMsg]) -> Vec<WireMsg> {
    msgs.iter()
        .map(|m| {
            if m.harness {
                let tag = if m.is_error { "System error" } else { "System notice" };
                return WireMsg {
                    role: "user",
                    content: format!("[{tag}, not the user]:\n{}", m.content),
                };
            }
            match m.role {
                Role::System => WireMsg {
                    role: "system",
                    content: m.content.clone(),
                },
                Role::User => WireMsg {
                    role: "user",
                    content: m.content.clone(),
                },
                Role::Assistant => WireMsg {
                    role: "assistant",
                    content: m.content.clone(),
                },
                Role::Tool => {
                    let name = m.tool_name.as_deref().unwrap_or("tool");
                    let err = if m.is_error { " (ERROR)" } else { "" };
                    WireMsg {
                        role: "user",
                        content: format!("Result from {name}{err}:\n{}", m.content),
                    }
                }
            }
        })
        .collect()
}

/// Convierte la conversación rica al formato de cable para la **ruta nativa** (llama-server
/// **con** `--jinja`): usa el rol `tool` real con `tool_call_id`, y los turnos del asistente
/// llevan sus `tool_calls` en formato OpenAI. Los argumentos de cada llamada se serializan como
/// string JSON, que es lo que esperan las plantillas de chat.
pub fn to_wire_native(msgs: &[AgentMsg]) -> Vec<serde_json::Value> {
    msgs.iter()
        .map(|m| {
            if m.harness {
                let tag = if m.is_error { "System error" } else { "System notice" };
                return serde_json::json!({
                    "role": "user",
                    "content": format!("[{tag}, not the user]:\n{}", m.content),
                });
            }
            match m.role {
                Role::System => serde_json::json!({ "role": "system", "content": m.content }),
                Role::User => serde_json::json!({ "role": "user", "content": m.content }),
                Role::Assistant if !m.tool_calls.is_empty() => {
                    let calls: Vec<serde_json::Value> = m
                        .tool_calls
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "id": c.id,
                                "type": "function",
                                "function": {
                                    "name": c.name,
                                    "arguments": c.args.to_string(),
                                }
                            })
                        })
                        .collect();
                    serde_json::json!({
                        "role": "assistant",
                        "content": m.content,
                        "tool_calls": calls,
                    })
                }
                Role::Assistant => serde_json::json!({ "role": "assistant", "content": m.content }),
                Role::Tool => {
                    let mut v = serde_json::json!({ "role": "tool", "content": m.content });
                    if let Some(id) = &m.tool_call_id {
                        v["tool_call_id"] = serde_json::Value::String(id.clone());
                    }
                    if let Some(name) = &m.tool_name {
                        v["name"] = serde_json::Value::String(name.clone());
                    }
                    v
                }
            }
        })
        .collect()
}

/// Estimación conservadora de tokens del CONTENIDO de un mensaje. Combina dos heurísticas
/// porque ningún chars→tokens fijo sirve para ambos mundos que conviven en una sesión:
/// - Prosa (~3–4 caracteres/token): `bytes/3`.
/// - Código/números/símbolos (~1–2 caracteres/token, largamente subestimados por `bytes/3`):
///   se aporta `lines` (cada línea arranca un contexto de tokenización).
///
/// `bytes/3` solo subestimaba hasta ~1.6× en salidas de herramientas muy numéricas
/// (p.ej. enumeraciones `línea 10: valor_10 = 10 * 2 = 20`), lo que hacía que
/// `budget_view` dejara pasar conversaciones que el KV-cache rechazaba con
/// "exceeds context size" (verificado en eval con Qwen3.5-4B). Al sumar el conteo de
/// líneas se compensa la granularidad del tokenizer sin desperdiciar mucho contexto en
/// prosa. Tiende a sobreestimar ligeramente (preferible: poda antes vs. desbordar).
///
/// No incluye el overhead de envoltura por mensaje (rol, separadores de plantilla): eso lo
/// suma el caller como `+ 4` por mensaje.
pub fn approx_tokens(s: &str) -> usize {
    let lines = s.lines().count().max(1);
    s.len() / 3 + lines
}
