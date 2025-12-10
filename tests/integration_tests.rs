use commitment_rs::*;

/// Mock git provider for integration testing
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

/// Mock agent for integration testing
struct MockAgent {
    response: String,
}

impl MockAgent {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
        }
    }

    async fn execute(&self, _prompt: &str) -> Result<String, AgentError> {
        Ok(self.response.clone())
    }
}

#[tokio::test]
async fn full_generation_flow() {
    // Setup
    let git = MockGitProvider::new();

    // Create a mock agent that returns a valid conventional commit
    let mock_agent = MockAgent::new("feat: add test function");

    // Get the diff
    let diff = git.get_staged_diff().unwrap();

    // Build the prompt
    let prompt = build_prompt(&diff);

    // Verify prompt contains expected sections
    assert!(prompt.contains("=== FILE STATISTICS ==="));
    assert!(prompt.contains("=== FILE STATUS ==="));
    assert!(prompt.contains("=== FULL DIFF ==="));

    // Execute mock agent
    let raw_response = mock_agent.execute(&prompt).await.unwrap();

    // Clean the response
    let cleaned = clean_ai_response(&raw_response);

    // Validate
    let commit = ConventionalCommit::validate(&cleaned).unwrap();
    assert_eq!(commit.as_str(), "feat: add test function");
}

#[tokio::test]
async fn truncation_message_appears_for_large_diffs() {
    // Create a diff that exceeds 8000 characters
    let large_diff = "x".repeat(8500);

    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "1 file changed, 1000 insertions(+)".to_string(),
        name_status: "M\tsrc/large_file.rs".to_string(),
        diff: large_diff,
    });

    // Build prompt
    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Verify truncation message appears
    assert!(
        prompt.contains("... (diff truncated)"),
        "Expected truncation message for diff > 8000 chars"
    );

    // Verify the prompt is not excessively large
    // Should be around 8000 chars for diff + overhead for other sections
    assert!(
        prompt.len() < 10000,
        "Prompt should be truncated to reasonable size"
    );
}

#[tokio::test]
async fn change_summary_appears_in_prompt() {
    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "3 files changed, 50 insertions(+), 20 deletions(-)".to_string(),
        name_status: "M\tsrc/a.rs\nA\tsrc/b.rs\nD\tsrc/c.rs".to_string(),
        diff: "@@ -1,3 +1,4 @@\n+new code".to_string(),
    });

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Verify change summary section exists
    assert!(
        prompt.contains("=== CHANGE SUMMARY ==="),
        "Expected CHANGE SUMMARY section"
    );

    // Verify it contains parsed stats
    assert!(
        prompt.contains("Files changed: 3"),
        "Expected file count in summary"
    );
    assert!(
        prompt.contains("Lines added: 50"),
        "Expected lines added in summary"
    );
    assert!(
        prompt.contains("Lines removed: 20"),
        "Expected lines removed in summary"
    );

    // Verify change summary appears before other sections
    let summary_pos = prompt.find("=== CHANGE SUMMARY ===").unwrap();
    let stats_pos = prompt.find("=== FILE STATISTICS ===").unwrap();
    assert!(
        summary_pos < stats_pos,
        "CHANGE SUMMARY should appear before FILE STATISTICS"
    );
}

#[tokio::test]
async fn staged_files_display_formatting() {
    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "3 files changed, 25 insertions(+), 5 deletions(-)".to_string(),
        name_status: "M\tsrc/main.rs\nA\tsrc/new.rs\nD\tsrc/old.rs".to_string(),
        diff: "@@ diff content".to_string(),
    });

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Verify FILE STATUS section exists
    assert!(
        prompt.contains("=== FILE STATUS ==="),
        "Expected FILE STATUS section"
    );

    // Verify it shows the name-status output with proper formatting
    assert!(
        prompt.contains("M\tsrc/main.rs"),
        "Expected modified file status"
    );
    assert!(
        prompt.contains("A\tsrc/new.rs"),
        "Expected added file status"
    );
    assert!(
        prompt.contains("D\tsrc/old.rs"),
        "Expected deleted file status"
    );

    // Verify the stat section also appears
    assert!(
        prompt.contains("=== FILE STATISTICS ==="),
        "Expected FILE STATISTICS section"
    );
    assert!(
        prompt.contains("3 files changed, 25 insertions(+), 5 deletions(-)"),
        "Expected stat summary"
    );
}

#[tokio::test]
async fn git_diff_flags_produce_expected_output() {
    // This test verifies the integration behavior of diff output
    // The actual git diff flags (--unified=3, --ignore-space-change) are tested
    // in git.rs, but here we verify the output integrates correctly into the prompt

    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "1 file changed, 5 insertions(+), 2 deletions(-)".to_string(),
        name_status: "M\tsrc/lib.rs".to_string(),
        diff: "@@ -10,7 +10,10 @@ fn example() {\n-old line 1\n-old line 2\n+new line 1\n+new line 2\n+new line 3\n+new line 4\n+new line 5\n context line 1\n context line 2\n context line 3".to_string(),
    });

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Verify the diff appears in the prompt
    assert!(
        prompt.contains("@@ -10,7 +10,10 @@ fn example()"),
        "Expected unified diff header"
    );

    // Verify added/removed lines are present
    assert!(prompt.contains("-old line 1"), "Expected removed lines");
    assert!(prompt.contains("+new line 1"), "Expected added lines");

    // Verify context lines are present (from --unified=3)
    assert!(
        prompt.contains("context line"),
        "Expected context lines from unified diff"
    );
}

#[tokio::test]
async fn no_staged_changes_error() {
    let git = MockGitProvider::with_no_changes();

    // Verify has_staged_changes returns false
    let has_changes = git.has_staged_changes().unwrap();
    assert!(!has_changes);

    // Verify get_staged_diff returns error
    let result = git.get_staged_diff();
    assert!(matches!(result, Err(GitError::NoStagedChanges)));
}

#[tokio::test]
async fn response_cleaning_integration() {
    let git = MockGitProvider::new();

    // Test various AI response formats
    let test_cases = vec![
        (
            "<<<COMMIT_MESSAGE_START>>>\nfeat: add feature\n<<<COMMIT_MESSAGE_END>>>",
            "feat: add feature",
        ),
        ("```\nfix: resolve bug\n```", "fix: resolve bug"),
        (
            "<thinking>Analyzing...</thinking>\nrefactor: improve code",
            "refactor: improve code",
        ),
        (
            "Here's the commit:\n\ndocs: update readme",
            "docs: update readme",
        ),
    ];

    for (raw_response, expected_cleaned) in test_cases {
        let mock_agent = MockAgent::new(raw_response);
        let diff = git.get_staged_diff().unwrap();
        let prompt = build_prompt(&diff);

        // Execute and clean
        let response = mock_agent.execute(&prompt).await.unwrap();
        let cleaned = clean_ai_response(&response);

        assert_eq!(
            cleaned, expected_cleaned,
            "Failed to clean: {}",
            raw_response
        );

        // Verify it validates as conventional commit
        let commit = ConventionalCommit::validate(&cleaned);
        assert!(
            commit.is_ok(),
            "Cleaned response should be valid conventional commit: {}",
            cleaned
        );
    }
}

#[tokio::test]
async fn signature_appending_integration() {
    let git = MockGitProvider::new();
    let mock_agent = MockAgent::new("feat: add feature");

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);
    let response = mock_agent.execute(&prompt).await.unwrap();
    let cleaned = clean_ai_response(&response);

    // Append signature
    let signature = "Co-Authored-By: AI <ai@example.com>";
    let final_message = format!("{}\n\n{}", cleaned, signature);

    // Validate
    let commit = ConventionalCommit::validate(&final_message).unwrap();

    // Verify signature is present
    assert!(commit.as_str().contains("Co-Authored-By:"));
    assert!(commit.as_str().contains("feat: add feature"));
}

#[tokio::test]
async fn realistic_flow_with_multiple_files() {
    // Simulate a realistic multi-file change
    let git = MockGitProvider::with_diff(StagedDiff {
        stat: " src/agents/mod.rs | 50 ++++++++++++++++++++++++++++++++++++++++++++++++++\n src/lib.rs         | 25 +++++++++++++++++++++++++\n 2 files changed, 75 insertions(+)".to_string(),
        name_status: "M\tsrc/agents/mod.rs\nM\tsrc/lib.rs".to_string(),
        diff: r#"@@ -1,8 +1,58 @@ src/agents/mod.rs
+pub fn generate() -> Result<String, AgentError> {
+    // Implementation
+}
@@ -1,5 +1,30 @@ src/lib.rs
+pub use agents::generate;
"#.to_string(),
    });

    let diff = git.get_staged_diff().unwrap();

    // Verify change summary
    let prompt = build_prompt(&diff);
    assert!(prompt.contains("Files changed: 2"));
    assert!(prompt.contains("Lines added: 75"));
    assert!(prompt.contains("Lines removed: 0"));

    // Verify both files appear in status
    assert!(prompt.contains("M\tsrc/agents/mod.rs"));
    assert!(prompt.contains("M\tsrc/lib.rs"));

    // Verify stat appears
    assert!(prompt.contains("75 insertions(+)"));

    // Verify diff content appears
    assert!(prompt.contains("pub fn generate()"));
    assert!(prompt.contains("pub use agents::generate"));

    // Simulate agent response
    let mock_agent = MockAgent::new(
        "feat(agents): add generate function\n\n- Add public generate API\n- Export from lib.rs",
    );
    let response = mock_agent.execute(&prompt).await.unwrap();
    let cleaned = clean_ai_response(&response);

    // Validate
    let commit = ConventionalCommit::validate(&cleaned);
    assert!(commit.is_ok());

    let commit = commit.unwrap();
    assert!(commit.as_str().contains("feat(agents)"));
    assert!(commit.as_str().contains("generate function"));
}

#[tokio::test]
async fn empty_diff_handling() {
    let git = MockGitProvider::with_diff(StagedDiff::default());

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Should have placeholders for empty sections
    assert!(prompt.contains("=== FILE STATISTICS ==="));
    assert!(prompt.contains("(no changes)"));
    assert!(prompt.contains("=== FILE STATUS ==="));
    assert!(prompt.contains("=== FULL DIFF ==="));

    // Should still be valid prompt structure
    assert!(prompt.contains("conventional commits"));
    assert!(prompt.contains("<<<COMMIT_MESSAGE_START>>>"));
}

#[tokio::test]
async fn validation_errors_propagate() {
    // Test various invalid commit formats
    let invalid_responses = vec![
        "",                     // Empty
        "just some text",       // No type
        "FEAT: description",    // Uppercase type
        "feat description",     // Missing colon
        "invalid: description", // Invalid type
    ];

    for invalid in invalid_responses {
        let result = ConventionalCommit::validate(invalid);
        assert!(
            result.is_err(),
            "Should reject invalid commit: '{}'",
            invalid
        );
    }

    // Verify valid formats work
    let valid_responses = vec![
        "feat: add feature",
        "fix: resolve bug",
        "docs: update readme",
        "feat(scope): add feature",
        "fix(api): resolve bug\n\n- Additional detail",
    ];

    for valid in valid_responses {
        let result = ConventionalCommit::validate(valid);
        assert!(result.is_ok(), "Should accept valid commit: '{}'", valid);
    }
}

#[tokio::test]
async fn utf8_handling_in_diffs() {
    // Test that UTF-8 characters in diffs are handled correctly
    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "1 file changed, 3 insertions(+)".to_string(),
        name_status: "M\tsrc/main.rs".to_string(),
        diff: "@@ -1,2 +1,5 @@\n+println!(\"Hello 世界 🦀\");\n+// Comment with émojis".to_string(),
    });

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Verify UTF-8 content is preserved
    assert!(prompt.contains("世界"));
    assert!(prompt.contains("🦀"));
    assert!(prompt.contains("émojis"));

    // Should not panic or corrupt the prompt
    assert!(prompt.contains("=== FULL DIFF ==="));
}

#[tokio::test]
async fn truncation_respects_utf8_boundaries() {
    // Create a diff with UTF-8 characters near the truncation boundary
    let mut large_diff = "a".repeat(7998);
    large_diff.push_str("🦀"); // 4-byte UTF-8 char
    large_diff.push_str(&"b".repeat(100));

    let git = MockGitProvider::with_diff(StagedDiff {
        stat: "1 file changed, 1 insertion(+)".to_string(),
        name_status: "M\tsrc/test.rs".to_string(),
        diff: large_diff,
    });

    let diff = git.get_staged_diff().unwrap();
    let prompt = build_prompt(&diff);

    // Should not panic on UTF-8 boundary
    assert!(prompt.contains("... (diff truncated)"));

    // Should be valid UTF-8
    assert!(std::str::from_utf8(prompt.as_bytes()).is_ok());
}
