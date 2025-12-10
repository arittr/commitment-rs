use crate::error::AgentError;
use crate::types::AgentName;
use std::time::Duration;
use tokio::process::Command;

/// Timeout for agent execution (120 seconds)
const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Gemini AI agent implementation
pub struct GeminiAgent;

impl GeminiAgent {
    /// Execute Gemini CLI with the given prompt
    ///
    /// Uses `gemini -p "<prompt>"` to pass the prompt as a command-line argument.
    /// Unlike Claude and Codex, Gemini takes the prompt as an argument, not stdin.
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        // Check if gemini CLI exists in PATH
        check_command_exists("gemini").await?;

        // Run gemini with timeout
        let output = tokio::time::timeout(AGENT_TIMEOUT, run_gemini_command(prompt))
            .await
            .map_err(|_| AgentError::Timeout {
                agent: AgentName::Gemini,
                timeout_secs: AGENT_TIMEOUT.as_secs(),
            })?
            .map_err(|e| match e {
                AgentError::NotFound { .. } => e,
                AgentError::ExecutionFailed { .. } => e,
                _ => AgentError::ExecutionFailed {
                    agent: AgentName::Gemini,
                    stderr: format!("unexpected error: {}", e),
                },
            })?;

        Ok(output)
    }
}

/// Check if a command exists in PATH
async fn check_command_exists(command: &str) -> Result<(), AgentError> {
    let result = Command::new("which").arg(command).output().await;

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(AgentError::NotFound {
            agent: AgentName::Gemini,
        }),
    }
}

/// Run gemini command with prompt as command-line argument
async fn run_gemini_command(prompt: &str) -> Result<String, AgentError> {
    let output = Command::new("gemini")
        .arg("-p")
        // Gemini takes the prompt as a command-line argument
        .arg(prompt)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|e| AgentError::ExecutionFailed {
            agent: AgentName::Gemini,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AgentError::ExecutionFailed {
            agent: AgentName::Gemini,
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn check_command_exists_for_common_command() {
        // Test with a command that definitely exists
        let result = check_command_exists("echo").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_command_exists_for_nonexistent_command() {
        // Test with a command that definitely doesn't exist
        let result = check_command_exists("this-command-definitely-does-not-exist-12345").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AgentError::NotFound { .. }));
    }

    #[tokio::test]
    async fn agent_timeout_value() {
        // Verify timeout is 120 seconds as specified
        assert_eq!(AGENT_TIMEOUT.as_secs(), 120);
    }
}
