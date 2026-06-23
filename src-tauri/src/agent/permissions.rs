use crate::agent::tools::Risk;

/// Modo de operación del agente, al estilo de opencode.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    /// Acceso completo: lee, escribe y ejecuta (pidiendo confirmación según riesgo).
    Build,
    /// Solo exploración: lee libremente, niega escrituras, pide confirmación para ejecutar.
    Plan,
}

impl AgentMode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "plan" => AgentMode::Plan,
            _ => AgentMode::Build,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentMode::Build => "build",
            AgentMode::Plan => "plan",
        }
    }
}

/// Decisión de la política de permisos para una herramienta.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

/// Política por defecto en función del modo y el riesgo de la herramienta.
pub fn decide(mode: AgentMode, risk: Risk) -> Decision {
    match (mode, risk) {
        (_, Risk::Read) => Decision::Allow,
        (AgentMode::Build, Risk::Write) => Decision::Ask,
        (AgentMode::Plan, Risk::Write) => Decision::Deny,
        (AgentMode::Build, Risk::Exec) => Decision::Ask,
        (AgentMode::Plan, Risk::Exec) => Decision::Ask,
    }
}
