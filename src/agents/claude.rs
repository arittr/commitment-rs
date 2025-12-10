use super::{AGENT_TIMEOUT, check_command_exists, run_command_with_stdin};
use crate::error::AgentError;
use crate::types::AgentName;

/// Claude AI agent implementation
#[derive(Default)]
pub struct ClaudeAgent;

impl ClaudeAgent {
    /// Execute Claude CLI with the given prompt
    ///
    /// Uses `claude --print` to output without interactive confirmation.
    /// The prompt is passed via stdin.
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        let agent = AgentName::Claude;
        check_command_exists(agent.command_name(), agent).await?;

        tokio::time::timeout(
            AGENT_TIMEOUT,
            run_command_with_stdin(agent.command_name(), &["--print"], prompt, agent),
        )
        .await
        .map_err(|_| AgentError::Timeout {
            agent,
            timeout_secs: AGENT_TIMEOUT.as_secs(),
        })?
    }
}
