use thiserror::Error;

// Domain errors - stubs for Phase 1

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent error placeholder")]
    Placeholder,
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("git error placeholder")]
    Placeholder,
}

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("generator error placeholder")]
    Placeholder,
}
