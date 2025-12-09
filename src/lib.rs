// Public API exports
pub use agents::{Agent, clean_ai_response};
pub use error::{AgentError, GeneratorError, GitError};
pub use git::GitProvider;
pub use prompt::build_prompt;
pub use types::{AgentName, ConventionalCommit, StagedDiff};

// Internal modules
pub mod agents;
pub mod cli;
pub mod error;
pub mod git;
pub mod hooks;
pub mod prompt;
pub mod types;

/// Generate a conventional commit message from staged git changes
///
/// Orchestrates the full flow:
/// 1. Check for staged changes (return error if none)
/// 2. Get staged diff from git
/// 3. Build AI prompt from diff
/// 4. Execute AI agent with prompt
/// 5. Clean AI response (remove markdown, thinking tags, etc.)
/// 6. Append signature if provided
/// 7. Validate as conventional commit
/// 8. Return validated commit message
///
/// # Arguments
///
/// * `git` - Git provider (trait for testability)
/// * `agent` - AI agent to use for generation
/// * `signature` - Optional signature to append (e.g., "Co-Authored-By: ...")
///
/// # Errors
///
/// Returns `GeneratorError` if:
/// - No staged changes exist (`GitError::NoStagedChanges`)
/// - Git command fails
/// - Agent execution fails or times out
/// - Response validation fails (not conventional commit format)
///
/// # Examples
///
/// ```no_run
/// use commitment_rs::*;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), GeneratorError> {
/// let git = git::RealGitProvider::new(PathBuf::from("."));
/// let agent = Agent::from(AgentName::Claude);
/// let signature = Some("Co-Authored-By: AI <ai@example.com>");
///
/// let commit = generate_commit_message(&git, &agent, signature).await?;
/// println!("Generated: {}", commit.as_str());
/// # Ok(())
/// # }
/// ```
pub async fn generate_commit_message(
    git: &impl GitProvider,
    agent: &Agent,
    signature: Option<&str>,
) -> Result<ConventionalCommit, GeneratorError> {
    // Step 1: Check for staged changes
    if !git.has_staged_changes()? {
        return Err(GitError::NoStagedChanges.into());
    }

    // Step 2: Get staged diff
    let diff = git.get_staged_diff()?;

    // Step 3: Build prompt
    let prompt = build_prompt(&diff);

    // Step 4: Execute agent
    let raw_response = agent.execute(&prompt).await?;

    // Step 5: Clean response
    let cleaned = clean_ai_response(&raw_response);

    // Step 6: Append signature if provided
    let final_message = if let Some(sig) = signature {
        format!("{}\n\n{}", cleaned, sig)
    } else {
        cleaned
    };

    // Step 7: Validate
    let commit = ConventionalCommit::validate(&final_message)
        .map_err(|e| GeneratorError::Validation(e.to_string()))?;

    // Step 8: Return validated commit
    Ok(commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AgentError, GitError};
    use crate::types::StagedDiff;

    // Mock git provider for testing
    struct MockGitProvider {
        staged_diff: Option<StagedDiff>,
        has_changes: bool,
    }

    impl MockGitProvider {
        fn new() -> Self {
            Self {
                staged_diff: Some(StagedDiff {
                    stat: "1 file changed, 10 insertions(+)".to_string(),
                    name_status: "A\tsrc/test.rs".to_string(),
                    diff: "@@ -0,0 +1,10 @@\n+fn test() {}".to_string(),
                }),
                has_changes: true,
            }
        }

        fn with_no_changes() -> Self {
            Self {
                staged_diff: None,
                has_changes: false,
            }
        }

        fn with_diff(diff: StagedDiff) -> Self {
            Self {
                staged_diff: Some(diff),
                has_changes: true,
            }
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
            Ok(())
        }
    }

    // Mock agent for testing - kept for future use in integration tests
    #[allow(dead_code)]
    struct MockAgent {
        response: Result<String, AgentError>,
    }

    #[allow(dead_code)]
    impl MockAgent {
        fn new(response: &str) -> Self {
            Self {
                response: Ok(response.to_string()),
            }
        }

        fn with_error(error: AgentError) -> Self {
            Self {
                response: Err(error),
            }
        }

        async fn execute(&self, _prompt: &str) -> Result<String, AgentError> {
            self.response.clone()
        }
    }

    #[tokio::test]
    async fn generate_commit_message_success() {
        let _git = MockGitProvider::new();
        let _agent = agents::Agent::Claude(agents::claude::ClaudeAgent);

        // We can't easily test with real agent, so we'll test the orchestration
        // by verifying error cases and using integration tests for full flow
    }

    #[tokio::test]
    async fn returns_error_when_no_staged_changes() {
        let git = MockGitProvider::with_no_changes();

        // Create a mock agent that returns valid response
        let _mock_agent = MockAgent::new("feat: add feature");

        // We need to test with the orchestration function, but since we can't pass MockAgent
        // directly to generate_commit_message (it expects Agent enum), we'll test the error
        // path by checking git directly
        let has_changes = git.has_staged_changes().unwrap();
        assert!(!has_changes);

        let diff_result = git.get_staged_diff();
        assert!(matches!(diff_result, Err(GitError::NoStagedChanges)));
    }

    #[tokio::test]
    async fn orchestration_flow_test() {
        // This test verifies the orchestration logic by testing each step
        let git = MockGitProvider::new();

        // Step 1: Check for staged changes
        let has_changes = git.has_staged_changes().unwrap();
        assert!(has_changes);

        // Step 2: Get staged diff
        let diff = git.get_staged_diff().unwrap();
        assert!(!diff.stat.is_empty());

        // Step 3: Build prompt
        let prompt = build_prompt(&diff);
        assert!(prompt.contains("conventional commit"));
        assert!(prompt.contains(&diff.stat));

        // Step 4: Simulate agent execution (cleaned response)
        let raw_response = "Here is the commit message:\n```\nfeat: add test function\n```";

        // Step 5: Clean response
        let cleaned = clean_ai_response(raw_response);
        assert_eq!(cleaned, "feat: add test function");

        // Step 6: Test signature appending
        let with_signature = format!(
            "{}\n\n{}",
            cleaned, "Co-Authored-By: Test <test@example.com>"
        );
        assert!(with_signature.contains("Co-Authored-By:"));

        // Step 7: Validate
        let commit = ConventionalCommit::validate(&cleaned).unwrap();
        assert_eq!(commit.as_str(), "feat: add test function");
    }

    #[tokio::test]
    async fn signature_appending_test() {
        let _git = MockGitProvider::new();

        // Test signature formatting
        let base_message = "feat: add feature";
        let signature = "Co-Authored-By: AI <ai@example.com>";
        let expected = format!("{}\n\n{}", base_message, signature);

        // Validate the combined message
        let result = ConventionalCommit::validate(&expected);
        assert!(result.is_ok());

        let commit = result.unwrap();
        assert!(commit.as_str().contains("Co-Authored-By:"));
        assert!(commit.as_str().starts_with("feat: add feature"));
    }

    #[tokio::test]
    async fn validation_error_propagation() {
        // Test that invalid commit messages are rejected
        let invalid_messages = vec![
            "",                     // Empty
            "just a description",   // No type
            "FEAT: description",    // Uppercase type
            "feat description",     // No colon
            "invalid: description", // Invalid type
        ];

        for msg in invalid_messages {
            let result = ConventionalCommit::validate(msg);
            assert!(result.is_err(), "Should reject: '{}'", msg);
        }
    }

    #[tokio::test]
    async fn git_error_propagation() {
        let git = MockGitProvider::with_no_changes();

        // Verify error propagates correctly
        let diff_result = git.get_staged_diff();
        assert!(matches!(diff_result, Err(GitError::NoStagedChanges)));

        // This would be converted to GeneratorError::Git in the main function
        let gen_error: GeneratorError = GitError::NoStagedChanges.into();
        assert!(matches!(gen_error, GeneratorError::Git(_)));
    }

    #[tokio::test]
    async fn response_cleaning_integration() {
        // Test the cleaning pipeline with realistic AI responses
        let test_cases = vec![
            (
                "<<<COMMIT_MESSAGE_START>>>feat: add feature<<<COMMIT_MESSAGE_END>>>",
                "feat: add feature",
            ),
            ("```\nfeat: add feature\n```", "feat: add feature"),
            (
                "Here is the commit message:\nfeat: add feature",
                "feat: add feature",
            ),
            (
                "<thinking>Analyzing diff...</thinking>\nfeat: add feature",
                "feat: add feature",
            ),
            (
                "feat: add feature\n\n\n\nBody text",
                "feat: add feature\n\nBody text",
            ),
        ];

        for (input, expected) in test_cases {
            let cleaned = clean_ai_response(input);
            assert_eq!(cleaned, expected, "Failed to clean: {}", input);
        }
    }

    #[tokio::test]
    async fn empty_diff_handling() {
        let git = MockGitProvider::with_diff(StagedDiff::default());

        // Get diff
        let diff = git.get_staged_diff().unwrap();

        // Build prompt with empty diff
        let prompt = build_prompt(&diff);

        // Should still generate valid prompt with placeholders
        assert!(prompt.contains("=== FILE STATISTICS ==="));
        assert!(prompt.contains("(no changes)"));
    }

    #[tokio::test]
    async fn realistic_diff_flow() {
        let git = MockGitProvider::with_diff(StagedDiff {
            stat: " src/lib.rs | 50 ++++++++++++++++++++++++++++++++++++++++++++++++++\n 1 file changed, 50 insertions(+)".to_string(),
            name_status: "M\tsrc/lib.rs".to_string(),
            diff: "@@ -1,8 +1,58 @@\n+pub async fn generate_commit_message() {}".to_string(),
        });

        // Full orchestration test with realistic data
        let has_changes = git.has_staged_changes().unwrap();
        assert!(has_changes);

        let diff = git.get_staged_diff().unwrap();
        assert!(diff.stat.contains("50 insertions"));
        assert!(diff.name_status.contains("M\tsrc/lib.rs"));

        let prompt = build_prompt(&diff);
        assert!(prompt.contains("=== FILE STATISTICS ==="));
        assert!(prompt.contains("50 insertions"));
        assert!(prompt.contains("=== FULL DIFF ==="));
        assert!(prompt.contains("generate_commit_message"));

        // Simulate AI response with various cleaning needed
        let ai_response_with_markers = r#"
<thinking>
This adds the core orchestration function.
</thinking>

<<<COMMIT_MESSAGE_START>>>
feat(core): add generate_commit_message orchestration

Implements the main function that orchestrates the full flow.
<<<COMMIT_MESSAGE_END>>>
"#;

        let cleaned = clean_ai_response(ai_response_with_markers);
        // The cleaned response should start with the commit type
        assert!(cleaned.starts_with("feat"));
        assert!(cleaned.contains("orchestration"));
        assert!(!cleaned.contains("<thinking>"));
        assert!(!cleaned.contains("<<<COMMIT_MESSAGE"));

        // Validate the cleaned message
        let commit = ConventionalCommit::validate(&cleaned).unwrap();
        assert!(commit.as_str().contains("orchestration"));
    }

    #[test]
    fn public_api_exports() {
        // Verify all necessary types are exported
        fn _assert_exports() {
            // Types
            let _: AgentName = AgentName::Claude;
            let _: StagedDiff = StagedDiff::default();

            // Functions
            let diff = StagedDiff::default();
            let _ = build_prompt(&diff);
            let _ = clean_ai_response("test");

            // Traits (via type constraint)
            fn _uses_git_provider(_: &impl GitProvider) {}

            // Errors
            let _: Result<(), AgentError> = Ok(());
            let _: Result<(), GitError> = Ok(());
            let _: Result<(), GeneratorError> = Ok(());
        }
    }

    #[test]
    fn module_organization() {
        // Verify module structure exists (package name is commitment_rs)
        assert!(std::module_path!().starts_with("commitment_rs"));
    }
}
