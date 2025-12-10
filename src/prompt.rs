use crate::types::StagedDiff;

/// Build AI prompt from staged git diff
///
/// Creates a template with:
/// - Instructions for conventional commit format
/// - File statistics (--stat)
/// - File name/status (--name-status)
/// - Full diff content
/// - Marker tags for response extraction
///
/// No diff analysis - AI handles pattern detection
pub fn build_prompt(diff: &StagedDiff) -> String {
    let mut prompt = String::new();

    // Instructions for conventional commit format
    prompt.push_str("Generate a professional commit message based on the actual code changes.\n\n");
    prompt.push_str("Requirements:\n");
    prompt.push_str("1. ANALYZE THE ACTUAL CODE CHANGES - don't guess based on file names\n");
    prompt.push_str(
        "2. Clear, descriptive title (50 chars or less) following conventional commits\n",
    );
    prompt.push_str("   - Start with type: feat, fix, docs, style, refactor, test, chore, perf, build, ci, or revert\n");
    prompt.push_str("   - Optional scope in parentheses: type(scope): description\n");
    prompt.push_str("3. Be CONCISE - match detail level to scope of changes:\n");
    prompt.push_str("   - Single file/method: 2-4 bullet points max\n");
    prompt.push_str("   - Multiple files: 4-6 bullet points max\n");
    prompt.push_str("   - Major refactor: 6+ bullet points as needed\n");
    prompt.push_str("4. Use imperative mood (\"Add feature\" not \"Added feature\")\n");
    prompt.push_str("5. Format: Title + blank line + bullet point details (use - prefix)\n");
    prompt.push_str("6. Focus on the most important changes from the diff:\n");
    prompt.push_str("   - Key functionality added/modified/removed\n");
    prompt.push_str("   - Significant logic or behavior changes\n");
    prompt.push_str("   - Important architectural changes\n");
    prompt.push_str("7. Avoid over-describing implementation details for small changes\n");
    prompt.push_str("8. DO NOT include preamble like \"Looking at the changes\"\n");
    prompt.push_str("9. Start directly with the action (\"Add\", \"Fix\", \"Update\", etc.)\n");
    prompt.push_str("10. Quality over quantity - fewer, more meaningful bullet points\n\n");

    prompt.push_str("Example format:\n");
    prompt.push_str("feat: add user authentication system\n\n");
    prompt.push_str("- Implement JWT-based authentication flow\n");
    prompt.push_str("- Add login/logout endpoints in auth routes\n");
    prompt.push_str("- Create user session management middleware\n\n");

    prompt.push_str("Return ONLY the commit message content between these markers:\n");
    prompt.push_str("<<<COMMIT_MESSAGE_START>>>\n");
    prompt.push_str("(commit message goes here)\n");
    prompt.push_str("<<<COMMIT_MESSAGE_END>>>\n\n");

    // File statistics section
    prompt.push_str("=== FILE STATISTICS ===\n");
    if diff.stat.is_empty() {
        prompt.push_str("(no changes)\n");
    } else {
        prompt.push_str(&diff.stat);
        prompt.push('\n');
    }
    prompt.push('\n');

    // File name/status section
    prompt.push_str("=== FILE STATUS ===\n");
    if diff.name_status.is_empty() {
        prompt.push_str("(no changes)\n");
    } else {
        prompt.push_str(&diff.name_status);
        prompt.push('\n');
    }
    prompt.push('\n');

    // Full diff section
    prompt.push_str("=== FULL DIFF ===\n");
    if diff.diff.is_empty() {
        prompt.push_str("(no changes)\n");
    } else {
        prompt.push_str(&diff.diff);
        prompt.push('\n');
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_conventional_commit_instructions() {
        let diff = StagedDiff::default();
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("conventional commits"));
        assert!(prompt.contains("feat, fix, docs"));
        assert!(prompt.contains("type(scope): description"));
        // New format requirements
        assert!(prompt.contains("bullet point"));
        assert!(prompt.contains("imperative mood"));
        assert!(prompt.contains("Quality over quantity"));
    }

    #[test]
    fn includes_marker_tags() {
        let diff = StagedDiff::default();
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("<<<COMMIT_MESSAGE_START>>>"));
        assert!(prompt.contains("<<<COMMIT_MESSAGE_END>>>"));
    }

    #[test]
    fn includes_all_diff_sections() {
        let diff = StagedDiff::default();
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("=== FILE STATISTICS ==="));
        assert!(prompt.contains("=== FILE STATUS ==="));
        assert!(prompt.contains("=== FULL DIFF ==="));
    }

    #[test]
    fn handles_empty_diff_gracefully() {
        let diff = StagedDiff::default();
        let prompt = build_prompt(&diff);

        // Should have placeholders for empty sections
        assert!(prompt.contains("(no changes)"));
    }

    #[test]
    fn includes_stat_section() {
        let diff = StagedDiff {
            stat: "1 file changed, 10 insertions(+)".to_string(),
            name_status: String::new(),
            diff: String::new(),
        };
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("=== FILE STATISTICS ==="));
        assert!(prompt.contains("1 file changed, 10 insertions(+)"));
    }

    #[test]
    fn includes_name_status_section() {
        let diff = StagedDiff {
            stat: String::new(),
            name_status: "A\tsrc/test.rs\nM\tsrc/lib.rs".to_string(),
            diff: String::new(),
        };
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("=== FILE STATUS ==="));
        assert!(prompt.contains("A\tsrc/test.rs"));
        assert!(prompt.contains("M\tsrc/lib.rs"));
    }

    #[test]
    fn includes_full_diff_section() {
        let diff = StagedDiff {
            stat: String::new(),
            name_status: String::new(),
            diff: "@@ -1,3 +1,4 @@\n fn main() {\n+    println!(\"hello\");\n }".to_string(),
        };
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("=== FULL DIFF ==="));
        assert!(prompt.contains("@@ -1,3 +1,4 @@"));
        assert!(prompt.contains("println!(\"hello\")"));
    }

    #[test]
    fn builds_complete_prompt_with_all_sections() {
        let diff = StagedDiff {
            stat: "1 file changed, 1 insertion(+)".to_string(),
            name_status: "M\tsrc/main.rs".to_string(),
            diff: "@@ -1 +1,2 @@\n fn main() {\n+    println!(\"test\");\n }".to_string(),
        };
        let prompt = build_prompt(&diff);

        // Verify all sections present
        assert!(prompt.contains("conventional commits"));
        assert!(prompt.contains("<<<COMMIT_MESSAGE_START>>>"));
        assert!(prompt.contains("<<<COMMIT_MESSAGE_END>>>"));
        assert!(prompt.contains("=== FILE STATISTICS ==="));
        assert!(prompt.contains("1 file changed, 1 insertion(+)"));
        assert!(prompt.contains("=== FILE STATUS ==="));
        assert!(prompt.contains("M\tsrc/main.rs"));
        assert!(prompt.contains("=== FULL DIFF ==="));
        assert!(prompt.contains("println!(\"test\")"));
        // Verify example format present
        assert!(prompt.contains("Example format:"));
        assert!(prompt.contains("- Implement JWT"));
    }

    #[test]
    fn prompt_format_is_stable() {
        let diff = StagedDiff {
            stat: "test stat".to_string(),
            name_status: "test status".to_string(),
            diff: "test diff".to_string(),
        };
        let prompt = build_prompt(&diff);

        // Verify section order (important for AI understanding)
        let stats_pos = prompt.find("=== FILE STATISTICS ===").unwrap();
        let status_pos = prompt.find("=== FILE STATUS ===").unwrap();
        let diff_pos = prompt.find("=== FULL DIFF ===").unwrap();

        assert!(stats_pos < status_pos);
        assert!(status_pos < diff_pos);
    }

    #[test]
    fn marker_placement_before_sections() {
        let diff = StagedDiff::default();
        let prompt = build_prompt(&diff);

        let marker_pos = prompt.find("<<<COMMIT_MESSAGE_START>>>").unwrap();
        let stats_pos = prompt.find("=== FILE STATISTICS ===").unwrap();

        // Markers should come before diff sections
        assert!(marker_pos < stats_pos);
    }

    #[test]
    fn no_extra_whitespace_at_end() {
        let diff = StagedDiff {
            stat: "test".to_string(),
            name_status: "test".to_string(),
            diff: "test".to_string(),
        };
        let prompt = build_prompt(&diff);

        // Should not have excessive trailing whitespace
        assert!(!prompt.ends_with("\n\n\n"));
    }
}
