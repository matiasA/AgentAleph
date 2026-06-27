use super::{ParamSpec, ParamType, Risk, Tool, ToolCtx};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Stdio;
use tokio::process::Command;

const TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT: usize = 8000;

/// Dangerous command patterns rejected outright.
const BLACKLIST: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "mkfs",
    ":(){",      // fork bomb
    "dd if=",
    "> /dev/sda",
    "chmod -r 000",
    "shutdown",
    "reboot",
    "sudo ",
];

pub struct Bash;

#[async_trait]
impl Tool for Bash {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a shell command in the project directory."
    }

    fn risk(&self) -> Risk {
        Risk::Exec
    }

    fn params(&self) -> Vec<ParamSpec> {
        vec![ParamSpec::req("command", ParamType::Str)]
    }

    async fn execute(&self, args: &Value, ctx: &ToolCtx) -> AppResult<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Other("bash requires 'command'".into()))?;

        let lc = command.to_lowercase();
        if BLACKLIST.iter().any(|p| lc.contains(p)) {
            return Err(AppError::Other(
                "command rejected: it matches a destructive blacklist pattern".into(),
            ));
        }

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&ctx.working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let fut = cmd.output();
        let output = match tokio::time::timeout(std::time::Duration::from_secs(TIMEOUT_SECS), fut)
            .await
        {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Err(AppError::Other(format!("could not execute: {e}"))),
            Err(_) => {
                return Err(AppError::Other(format!(
                    "command exceeded the {TIMEOUT_SECS}s limit and was aborted"
                )))
            }
        };

        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut out = format!("[exit {code}]\n");
        if !stdout.trim().is_empty() {
            out.push_str("stdout:\n");
            out.push_str(&truncate(&stdout));
            out.push('\n');
        }
        if !stderr.trim().is_empty() {
            out.push_str("stderr:\n");
            out.push_str(&truncate(&stderr));
            out.push('\n');
        }
        Ok(out)
    }
}

fn truncate(s: &str) -> String {
    if s.len() <= MAX_OUTPUT {
        s.to_string()
    } else {
        let head: String = s.chars().take(MAX_OUTPUT).collect();
        format!("{head}\n... [output truncated]")
    }
}
