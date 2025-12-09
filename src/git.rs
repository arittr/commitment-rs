use crate::error::GitError;
use crate::types::StagedDiff;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git operations abstraction
///
/// Trait enables dependency injection for testing without mock libraries.
/// Git operations are SYNC - fast, local operations don't need async.
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;
    fn has_staged_changes(&self) -> Result<bool, GitError>;
    fn commit(&self, message: &str) -> Result<(), GitError>;
}

/// Production git provider using real git commands
pub struct RealGitProvider {
    cwd: PathBuf,
}

impl RealGitProvider {
    /// Create a new git provider with the given working directory
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd }
    }

    /// Helper to run git commands with consistent error handling
    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.cwd)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: format!("git {}", args.join(" ")),
                stderr,
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Resolve git directory path, handling worktrees
    ///
    /// In a worktree, .git is a file containing: gitdir: <path>
    /// In a regular repo, .git is a directory
    #[allow(dead_code)] // Used by hook installation in later phases
    fn resolve_git_dir(&self) -> Result<PathBuf, GitError> {
        let git_path = self.cwd.join(".git");

        if git_path.is_dir() {
            return Ok(git_path);
        }

        if git_path.is_file() {
            let content = std::fs::read_to_string(&git_path)?;
            // Format: "gitdir: /path/to/worktree\n"
            if let Some(gitdir_line) = content.lines().next()
                && let Some(path) = gitdir_line.strip_prefix("gitdir: ")
            {
                let resolved = if Path::new(path).is_absolute() {
                    PathBuf::from(path)
                } else {
                    self.cwd.join(path)
                };
                return Ok(resolved);
            }
        }

        Err(GitError::WorktreeResolution {
            path: git_path.display().to_string(),
        })
    }
}

impl GitProvider for RealGitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
        // Check if there are any staged changes first
        if !self.has_staged_changes()? {
            return Err(GitError::NoStagedChanges);
        }

        // Get the three components of the diff
        let stat = self.run_git(&["diff", "--cached", "--stat"])?;
        let name_status = self.run_git(&["diff", "--cached", "--name-status"])?;
        let diff = self.run_git(&["diff", "--cached"])?;

        Ok(StagedDiff {
            stat,
            name_status,
            diff,
        })
    }

    fn has_staged_changes(&self) -> Result<bool, GitError> {
        // git diff --cached --quiet exits with 1 if there are changes
        let status = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&self.cwd)
            .status()?;

        // Exit code 0 = no changes, 1 = has changes
        Ok(!status.success())
    }

    fn commit(&self, message: &str) -> Result<(), GitError> {
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.cwd)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(GitError::CommandFailed {
                command: "git commit".to_string(),
                stderr,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock git provider for testing
    ///
    /// Builder pattern allows flexible test setup without external mock libraries
    struct MockGitProvider {
        staged_diff: Option<StagedDiff>,
        has_changes: bool,
        commit_should_fail: bool,
    }

    impl MockGitProvider {
        fn new() -> Self {
            Self {
                staged_diff: Some(StagedDiff::default()),
                has_changes: true,
                commit_should_fail: false,
            }
        }

        fn with_diff(mut self, diff: StagedDiff) -> Self {
            self.staged_diff = Some(diff);
            self.has_changes = true;
            self
        }

        fn with_no_changes(mut self) -> Self {
            self.has_changes = false;
            self
        }

        fn with_commit_failure(mut self) -> Self {
            self.commit_should_fail = true;
            self
        }
    }

    impl GitProvider for MockGitProvider {
        fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
            if !self.has_changes {
                return Err(GitError::NoStagedChanges);
            }
            self.staged_diff.clone().ok_or(GitError::NoStagedChanges)
        }

        fn has_staged_changes(&self) -> Result<bool, GitError> {
            Ok(self.has_changes)
        }

        fn commit(&self, _message: &str) -> Result<(), GitError> {
            if self.commit_should_fail {
                return Err(GitError::CommandFailed {
                    command: "git commit".to_string(),
                    stderr: "commit failed".to_string(),
                });
            }
            Ok(())
        }
    }

    #[test]
    fn mock_provider_returns_diff() {
        let mock = MockGitProvider::new().with_diff(StagedDiff {
            stat: "1 file changed".to_string(),
            name_status: "M\ttest.rs".to_string(),
            diff: "@@ test diff".to_string(),
        });

        let result = mock.get_staged_diff();
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert_eq!(diff.stat, "1 file changed");
        assert_eq!(diff.name_status, "M\ttest.rs");
        assert_eq!(diff.diff, "@@ test diff");
    }

    #[test]
    fn mock_provider_no_changes() {
        let mock = MockGitProvider::new().with_no_changes();

        assert!(!mock.has_staged_changes().unwrap());
        assert!(matches!(
            mock.get_staged_diff(),
            Err(GitError::NoStagedChanges)
        ));
    }

    #[test]
    fn mock_provider_commit_success() {
        let mock = MockGitProvider::new();
        let result = mock.commit("test: commit message");
        assert!(result.is_ok());
    }

    #[test]
    fn mock_provider_commit_failure() {
        let mock = MockGitProvider::new().with_commit_failure();
        let result = mock.commit("test: commit message");
        assert!(matches!(result, Err(GitError::CommandFailed { .. })));
    }

    #[test]
    fn real_provider_construction() {
        let provider = RealGitProvider::new(PathBuf::from("/tmp"));
        assert_eq!(provider.cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn resolve_git_dir_handles_worktree() {
        use std::io::Write;
        use tempfile::TempDir;

        // Create a temporary directory with a .git file (worktree style)
        let temp_dir = TempDir::new().unwrap();
        let git_file = temp_dir.path().join(".git");
        let mut file = std::fs::File::create(&git_file).unwrap();
        writeln!(file, "gitdir: /path/to/worktree/.git/worktrees/test").unwrap();

        let provider = RealGitProvider::new(temp_dir.path().to_path_buf());
        let result = provider.resolve_git_dir();

        assert!(result.is_ok());
        let git_dir = result.unwrap();
        assert_eq!(
            git_dir,
            PathBuf::from("/path/to/worktree/.git/worktrees/test")
        );
    }

    #[test]
    fn resolve_git_dir_handles_relative_path() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let git_file = temp_dir.path().join(".git");
        let mut file = std::fs::File::create(&git_file).unwrap();
        writeln!(file, "gitdir: ../main/.git/worktrees/test").unwrap();

        let provider = RealGitProvider::new(temp_dir.path().to_path_buf());
        let result = provider.resolve_git_dir();

        assert!(result.is_ok());
        // Should resolve relative to cwd
        let git_dir = result.unwrap();
        assert!(git_dir.to_string_lossy().contains("worktrees/test"));
    }

    #[test]
    fn resolve_git_dir_handles_missing_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let provider = RealGitProvider::new(temp_dir.path().to_path_buf());
        let result = provider.resolve_git_dir();

        assert!(matches!(result, Err(GitError::WorktreeResolution { .. })));
    }

    #[test]
    fn staged_diff_parsing() {
        // Test that StagedDiff can hold realistic git output
        let diff = StagedDiff {
            stat: " src/git.rs | 100 +++++++++++++++++++++++++++++++++++++++++++++++++\n 1 file changed, 100 insertions(+)".to_string(),
            name_status: "M\tsrc/git.rs".to_string(),
            diff: "@@ -1,5 +1,105 @@\n use crate::error::GitError;\n+use std::process::Command;".to_string(),
        };

        assert!(diff.stat.contains("src/git.rs"));
        assert!(diff.stat.contains("100 insertions"));
        assert!(diff.name_status.starts_with("M\t"));
        assert!(diff.diff.contains("@@"));
    }

    #[test]
    fn trait_allows_generic_functions() {
        // Verify trait enables generic functions for testing
        fn uses_git_provider(git: &impl GitProvider) -> Result<bool, GitError> {
            git.has_staged_changes()
        }

        let mock = MockGitProvider::new();
        assert!(uses_git_provider(&mock).unwrap());

        let mock_no_changes = MockGitProvider::new().with_no_changes();
        assert!(!uses_git_provider(&mock_no_changes).unwrap());
    }

    #[test]
    fn error_types_are_structured() {
        let err = GitError::NoStagedChanges;
        assert!(err.to_string().contains("no staged changes"));

        let err = GitError::CommandFailed {
            command: "git diff".to_string(),
            stderr: "fatal: not a git repository".to_string(),
        };
        assert!(err.to_string().contains("git diff"));
        assert!(err.to_string().contains("fatal"));
    }
}
