use crate::error::AgentError;
use crate::types::AgentName;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Timeout for agent execution (120 seconds)
const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Codex AI agent implementation
pub struct CodexAgent;

impl CodexAgent {
    /// Execute Codex CLI with the given prompt
    ///
    /// Uses `codex exec --skip-git-repo-check` to bypass git repository checks.
    /// The prompt is passed via stdin.
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        // Check if codex CLI exists in PATH
        check_command_exists("codex").await?;

        // Run codex with timeout
        let output = tokio::time::timeout(AGENT_TIMEOUT, run_codex_command(prompt))
            .await
            .map_err(|_| AgentError::Timeout {
                agent: AgentName::Codex,
                timeout_secs: AGENT_TIMEOUT.as_secs(),
            })?
            .map_err(|e| match e {
                AgentError::NotFound { .. } => e,
                AgentError::ExecutionFailed { .. } => e,
                _ => AgentError::ExecutionFailed {
                    agent: AgentName::Codex,
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
            agent: AgentName::Codex,
        }),
    }
}

/// Run codex command with prompt via stdin
async fn run_codex_command(prompt: &str) -> Result<String, AgentError> {
    let mut child = Command::new("codex")
        .arg("exec")
        // Codex requires --skip-git-repo-check to work in git worktrees
        .arg("--skip-git-repo-check")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AgentError::ExecutionFailed {
            agent: AgentName::Codex,
            stderr: e.to_string(),
        })?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent: AgentName::Codex,
                stderr: format!("failed to write to stdin: {}", e),
            })?;
        stdin
            .flush()
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent: AgentName::Codex,
                stderr: format!("failed to flush stdin: {}", e),
            })?;
    }

    // Wait for process to complete
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| AgentError::ExecutionFailed {
            agent: AgentName::Codex,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AgentError::ExecutionFailed {
            agent: AgentName::Codex,
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
