use crate::error::GitError;
use crate::types::StagedDiff;

// Git module - stubs for Phase 1

// GitProvider trait - stub
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
        todo!("implement in Phase 2")
    }

    fn has_staged_changes(&self) -> Result<bool, GitError> {
        todo!("implement in Phase 2")
    }

    fn commit(&self, _message: &str) -> Result<(), GitError> {
        todo!("implement in Phase 2")
    }
}

// RealGitProvider stub
pub struct RealGitProvider;

impl GitProvider for RealGitProvider {}
