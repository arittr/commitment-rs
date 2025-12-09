pub mod claude;

use crate::error::AgentError;

// Agent module - stubs for Phase 1

// Agent enum placeholder
pub enum Agent {
    Claude(claude::ClaudeAgent),
}

impl Agent {
    pub async fn execute(&self, _prompt: &str) -> Result<String, AgentError> {
        todo!("implement in Phase 2")
    }
}

// Response cleaning function stub
pub fn clean_ai_response(_response: &str) -> String {
    todo!("implement in Phase 2")
}
