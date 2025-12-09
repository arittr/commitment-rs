use crate::types::AgentName;
use thiserror::Error;

/// Errors from AI agent execution
#[derive(Error, Debug)]
pub enum AgentError {
    /// Agent binary not found in PATH
    #[error("agent `{agent}` not found in PATH")]
    NotFound { agent: AgentName },

    /// Agent process execution failed
    #[error("agent `{agent}` execution failed: {stderr}")]
    ExecutionFailed { agent: AgentName, stderr: String },

    /// Agent process timed out
    #[error("agent `{agent}` timed out after {timeout_secs}s")]
    Timeout { agent: AgentName, timeout_secs: u64 },

    /// Agent returned invalid response
    #[error("invalid response from agent: {reason}")]
    InvalidResponse { reason: String },
}

/// Errors from git operations
#[derive(Error, Debug)]
pub enum GitError {
    /// No staged changes to commit
    #[error("no staged changes found")]
    NoStagedChanges,

    /// Git command failed
    #[error("git command `{command}` failed: {stderr}")]
    CommandFailed { command: String, stderr: String },

    /// Failed to resolve git worktree directory
    #[error("failed to resolve git worktree directory at: {path}")]
    WorktreeResolution { path: String },

    /// I/O error during git operation
    #[error("I/O error during git operation")]
    Io(#[from] std::io::Error),
}

/// Errors from commit message generation
#[derive(Error, Debug)]
pub enum GeneratorError {
    /// Agent error (with automatic conversion via #[from])
    #[error(transparent)]
    Agent(#[from] AgentError),

    /// Git error (with automatic conversion via #[from])
    #[error(transparent)]
    Git(#[from] GitError),

    /// Commit validation failed
    #[error("commit validation failed: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_error_not_found_display() {
        let err = AgentError::NotFound {
            agent: AgentName::Claude,
        };
        let msg = err.to_string();
        assert!(msg.contains("claude"));
        assert!(msg.contains("not found"));
        assert!(msg.contains("PATH"));
    }

    #[test]
    fn agent_error_execution_failed_display() {
        let err = AgentError::ExecutionFailed {
            agent: AgentName::Codex,
            stderr: "permission denied".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("codex"));
        assert!(msg.contains("execution failed"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn agent_error_timeout_display() {
        let err = AgentError::Timeout {
            agent: AgentName::Gemini,
            timeout_secs: 120,
        };
        let msg = err.to_string();
        assert!(msg.contains("gemini"));
        assert!(msg.contains("timed out"));
        assert!(msg.contains("120"));
    }

    #[test]
    fn agent_error_invalid_response_display() {
        let err = AgentError::InvalidResponse {
            reason: "empty response".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid response"));
        assert!(msg.contains("empty response"));
    }

    #[test]
    fn git_error_no_staged_changes_display() {
        let err = GitError::NoStagedChanges;
        let msg = err.to_string();
        assert!(msg.contains("no staged changes"));
    }

    #[test]
    fn git_error_command_failed_display() {
        let err = GitError::CommandFailed {
            command: "git diff".to_string(),
            stderr: "not a git repository".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("git diff"));
        assert!(msg.contains("failed"));
        assert!(msg.contains("not a git repository"));
    }

    #[test]
    fn git_error_worktree_resolution_display() {
        let err = GitError::WorktreeResolution {
            path: "/path/to/worktree".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("worktree"));
        assert!(msg.contains("/path/to/worktree"));
    }

    #[test]
    fn generator_error_from_agent_error() {
        let agent_err = AgentError::NotFound {
            agent: AgentName::Claude,
        };
        let gen_err: GeneratorError = agent_err.into();
        assert!(matches!(gen_err, GeneratorError::Agent(_)));
        assert!(gen_err.to_string().contains("claude"));
    }

    #[test]
    fn generator_error_from_git_error() {
        let git_err = GitError::NoStagedChanges;
        let gen_err: GeneratorError = git_err.into();
        assert!(matches!(gen_err, GeneratorError::Git(_)));
        assert!(gen_err.to_string().contains("no staged changes"));
    }

    #[test]
    fn generator_error_validation_display() {
        let err = GeneratorError::Validation("invalid format".to_string());
        let msg = err.to_string();
        assert!(msg.contains("validation failed"));
        assert!(msg.contains("invalid format"));
    }

    #[test]
    fn error_types_implement_error_trait() {
        // Verify all error types implement std::error::Error
        fn assert_error<T: std::error::Error>() {}
        assert_error::<AgentError>();
        assert_error::<GitError>();
        assert_error::<GeneratorError>();
    }

    #[test]
    fn error_types_implement_debug() {
        // Verify all error types implement Debug
        let agent_err = AgentError::NotFound {
            agent: AgentName::Claude,
        };
        assert!(format!("{:?}", agent_err).contains("NotFound"));

        let git_err = GitError::NoStagedChanges;
        assert!(format!("{:?}", git_err).contains("NoStagedChanges"));

        let gen_err = GeneratorError::Validation("test".to_string());
        assert!(format!("{:?}", gen_err).contains("Validation"));
    }
}
