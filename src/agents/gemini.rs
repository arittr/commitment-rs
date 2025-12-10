use super::{AGENT_TIMEOUT, check_command_exists};
use crate::error::AgentError;
use crate::types::AgentName;
use tokio::process::Command;

/// Gemini AI agent implementation
#[derive(Default)]
pub struct GeminiAgent;

impl GeminiAgent {
    /// Execute Gemini CLI with the given prompt
    ///
    /// Uses `gemini -p "<prompt>"` to pass the prompt as a command-line argument.
    /// Unlike Claude and Codex, Gemini takes the prompt as an argument, not stdin.
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        let agent = AgentName::Gemini;
        check_command_exists(agent.command_name(), agent).await?;

        tokio::time::timeout(AGENT_TIMEOUT, Self::run_command(agent, prompt))
            .await
            .map_err(|_| AgentError::Timeout {
                agent,
                timeout_secs: AGENT_TIMEOUT.as_secs(),
            })?
    }

    /// Run gemini command with prompt as command-line argument
    async fn run_command(agent: AgentName, prompt: &str) -> Result<String, AgentError> {
        let output = Command::new(agent.command_name())
            .arg("-p")
            .arg(prompt)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AgentError::ExecutionFailed { agent, stderr });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
