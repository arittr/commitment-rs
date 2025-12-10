use super::{AGENT_TIMEOUT, check_command_exists, run_command_with_stdin};
use crate::error::AgentError;
use crate::types::AgentName;

/// Codex AI agent implementation
#[derive(Default)]
pub struct CodexAgent;

impl CodexAgent {
    /// Execute Codex CLI with the given prompt
    ///
    /// Uses `codex exec --skip-git-repo-check` to bypass git repository checks.
    /// The prompt is passed via stdin.
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        let agent = AgentName::Codex;
        check_command_exists(agent.command_name(), agent).await?;

        tokio::time::timeout(
            AGENT_TIMEOUT,
            run_command_with_stdin(
                agent.command_name(),
                &["exec", "--skip-git-repo-check"],
                prompt,
                agent,
            ),
        )
        .await
        .map_err(|_| AgentError::Timeout {
            agent,
            timeout_secs: AGENT_TIMEOUT.as_secs(),
        })?
    }
}
