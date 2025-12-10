//! Test fixtures for integration tests
//!
//! Provides realistic git diff scenarios for testing commit message generation.

use commitment_rs::{GitError, GitProvider, StagedDiff};

/// Mock git provider that returns fixture data
pub struct MockGitProvider {
    diff: StagedDiff,
}

impl MockGitProvider {
    pub fn new(diff: StagedDiff) -> Self {
        Self { diff }
    }
}

impl GitProvider for MockGitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
        Ok(self.diff.clone())
    }

    fn has_staged_changes(&self) -> Result<bool, GitError> {
        Ok(true)
    }

    fn commit(&self, _message: &str) -> Result<(), GitError> {
        Ok(())
    }
}

/// Collection of realistic git diff fixtures
pub mod diffs {
    use commitment_rs::StagedDiff;

    /// Simple single-file addition: adds an `add` function
    pub fn simple_addition() -> StagedDiff {
        StagedDiff {
            stat: " src/lib.rs | 5 +++++\n 1 file changed, 5 insertions(+)".into(),
            name_status: "M\tsrc/lib.rs".into(),
            diff: include_str!("diffs/simple_addition.diff").into(),
        }
    }

    /// Multi-file change: adds math and string modules
    pub fn multi_file_feature() -> StagedDiff {
        StagedDiff {
            stat: " src/lib.rs  | 10 ++++++++++\n src/main.rs |  3 +++\n 2 files changed, 13 insertions(+)".into(),
            name_status: "M\tsrc/lib.rs\nM\tsrc/main.rs".into(),
            diff: include_str!("diffs/multi_file_feature.diff").into(),
        }
    }

    /// Bug fix: replaces deprecated method with idiomatic Rust
    pub fn bug_fix() -> StagedDiff {
        StagedDiff {
            stat: " src/parser.rs | 4 ++--\n 1 file changed, 2 insertions(+), 2 deletions(-)"
                .into(),
            name_status: "M\tsrc/parser.rs".into(),
            diff: include_str!("diffs/bug_fix.diff").into(),
        }
    }

    /// Refactor: consolidates two handlers into one with match
    pub fn refactor_handlers() -> StagedDiff {
        StagedDiff {
            stat: " src/handlers.rs | 20 +++++++++-----------\n 1 file changed, 9 insertions(+), 11 deletions(-)".into(),
            name_status: "M\tsrc/handlers.rs".into(),
            diff: include_str!("diffs/refactor_handlers.diff").into(),
        }
    }

    /// Documentation: adds module-level docs
    pub fn documentation() -> StagedDiff {
        StagedDiff {
            stat: " src/lib.rs | 8 ++++++++\n 1 file changed, 8 insertions(+)".into(),
            name_status: "M\tsrc/lib.rs".into(),
            diff: include_str!("diffs/documentation.diff").into(),
        }
    }

    /// Test addition: adds unit tests for existing function
    pub fn add_tests() -> StagedDiff {
        StagedDiff {
            stat: " src/lib.rs | 15 +++++++++++++++\n 1 file changed, 15 insertions(+)".into(),
            name_status: "M\tsrc/lib.rs".into(),
            diff: include_str!("diffs/add_tests.diff").into(),
        }
    }
}
