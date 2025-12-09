use crate::error::AgentError;

// ClaudeAgent - stub for Phase 1

pub struct ClaudeAgent;

impl ClaudeAgent {
    pub async fn execute(&self, _prompt: &str) -> Result<String, AgentError> {
        todo!("implement in Phase 2")
    }
}
