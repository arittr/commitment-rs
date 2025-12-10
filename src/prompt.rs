use crate::types::{CONVENTIONAL_COMMIT_TYPES, StagedDiff};
use once_cell::sync::Lazy;
use regex::Regex;

/// Maximum length for diff content before truncation
const MAX_DIFF_LENGTH: usize = 8000;

/// Truncate diff content to prevent token limit issues
///
/// If diff exceeds MAX_DIFF_LENGTH, truncates at a character boundary
/// and appends a truncation indicator.
fn truncate_diff(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_LENGTH {
        return diff.to_string();
    }

    // Find character boundary to avoid panicking on UTF-8
    let mut boundary = MAX_DIFF_LENGTH;
    while boundary > 0 && !diff.is_char_boundary(boundary) {
        boundary -= 1;
    }

    format!("{}\n... (diff truncated)", &diff[..boundary])
}

/// Parse change summary from git stat and name-status output
///
/// Extracts:
/// - File count from name_status (line count)
/// - Lines added/removed from stat output
fn parse_change_summary(stat: &str, name_status: &str) -> String {
    // Count files from name_status (each line is a file)
    let file_count = name_status.lines().filter(|line| !line.is_empty()).count();

    // Regex to extract insertions and deletions from stat
    // Example: "1 file changed, 10 insertions(+), 5 deletions(-)"
    static INSERTIONS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(\d+) insertion[s]?\(\+\)").unwrap());
    static DELETIONS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(\d+) deletion[s]?\(-\)").unwrap());

    let lines_added = INSERTIONS_RE
        .captures(stat)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .unwrap_or(0);

    let lines_removed = DELETIONS_RE
        .captures(stat)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<usize>().ok())
        .unwrap_or(0);

    format!(
        "Files changed: {}\nLines added: {}\nLines removed: {}",
        file_count, lines_added, lines_removed
    )
}

/// Build AI prompt from staged git diff
///
/// Creates a template with:
/// - Instructions for conventional commit format
/// - Change summary (file count, lines added/removed)
/// - File statistics (--stat)
/// - File name/status (--name-status)
/// - Full diff content (truncated if over 8000 chars)
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
    prompt.push_str(&format!(
        "   - Start with type: {}\n",
        CONVENTIONAL_COMMIT_TYPES.join(", ")
    ));
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

    // Change summary section
    prompt.push_str("=== CHANGE SUMMARY ===\n");
    prompt.push_str(&parse_change_summary(&diff.stat, &diff.name_status));
    prompt.push_str("\n\n");

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

    // Full diff section (with truncation)
    prompt.push_str("=== FULL DIFF ===\n");
    if diff.diff.is_empty() {
        prompt.push_str("(no changes)\n");
    } else {
        let truncated = truncate_diff(&diff.diff);
        prompt.push_str(&truncated);
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

    // Truncation tests
    #[test]
    fn truncate_diff_within_limit() {
        let short_diff = "a".repeat(7999);
        let result = truncate_diff(&short_diff);
        assert_eq!(result, short_diff);
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn truncate_diff_at_exact_limit() {
        let exact_diff = "a".repeat(8000);
        let result = truncate_diff(&exact_diff);
        assert_eq!(result, exact_diff);
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn truncate_diff_over_limit() {
        let long_diff = "a".repeat(8001);
        let result = truncate_diff(&long_diff);
        // Result should be truncated original + message
        assert!(result.contains("... (diff truncated)"));
        assert!(result.starts_with("aaa"));
        // Verify the diff portion is truncated to 8000 or less
        let diff_portion = result.trim_end_matches("\n... (diff truncated)");
        assert!(diff_portion.len() <= MAX_DIFF_LENGTH);
    }

    #[test]
    fn truncate_diff_message_appended() {
        let long_diff = "x".repeat(10000);
        let result = truncate_diff(&long_diff);
        assert!(result.ends_with("... (diff truncated)"));
    }

    #[test]
    fn truncate_diff_respects_utf8_boundaries() {
        // Create a string with multibyte UTF-8 character near boundary
        let mut diff = "a".repeat(7998);
        diff.push('🦀'); // 4-byte UTF-8 char
        diff.push_str(&"b".repeat(100));

        let result = truncate_diff(&diff);
        // Should not panic and should be valid UTF-8
        assert!(result.len() <= 8000 + "... (diff truncated)".len() + 10);
    }

    // Change summary tests
    #[test]
    fn parse_change_summary_with_insertions_and_deletions() {
        let stat = "2 files changed, 10 insertions(+), 5 deletions(-)";
        let name_status = "M\tsrc/file1.rs\nA\tsrc/file2.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(
            result,
            "Files changed: 2\nLines added: 10\nLines removed: 5"
        );
    }

    #[test]
    fn parse_change_summary_only_insertions() {
        let stat = "1 file changed, 15 insertions(+)";
        let name_status = "A\tsrc/new.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(
            result,
            "Files changed: 1\nLines added: 15\nLines removed: 0"
        );
    }

    #[test]
    fn parse_change_summary_only_deletions() {
        let stat = "1 file changed, 8 deletions(-)";
        let name_status = "D\tsrc/old.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(result, "Files changed: 1\nLines added: 0\nLines removed: 8");
    }

    #[test]
    fn parse_change_summary_singular_insertion() {
        let stat = "1 file changed, 1 insertion(+)";
        let name_status = "M\tsrc/file.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(result, "Files changed: 1\nLines added: 1\nLines removed: 0");
    }

    #[test]
    fn parse_change_summary_singular_deletion() {
        let stat = "1 file changed, 1 deletion(-)";
        let name_status = "M\tsrc/file.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(result, "Files changed: 1\nLines added: 0\nLines removed: 1");
    }

    #[test]
    fn parse_change_summary_multiple_files() {
        let stat = "5 files changed, 100 insertions(+), 50 deletions(-)";
        let name_status = "M\tsrc/a.rs\nM\tsrc/b.rs\nA\tsrc/c.rs\nD\tsrc/d.rs\nM\tsrc/e.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(
            result,
            "Files changed: 5\nLines added: 100\nLines removed: 50"
        );
    }

    #[test]
    fn parse_change_summary_empty_stat() {
        let stat = "";
        let name_status = "M\tsrc/file.rs";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(result, "Files changed: 1\nLines added: 0\nLines removed: 0");
    }

    #[test]
    fn parse_change_summary_empty_name_status() {
        let stat = "1 file changed, 5 insertions(+)";
        let name_status = "";
        let result = parse_change_summary(stat, name_status);

        assert_eq!(result, "Files changed: 0\nLines added: 5\nLines removed: 0");
    }

    #[test]
    fn parse_change_summary_handles_whitespace_lines() {
        let stat = "2 files changed, 10 insertions(+), 5 deletions(-)";
        let name_status = "M\tsrc/a.rs\n\nM\tsrc/b.rs\n";
        let result = parse_change_summary(stat, name_status);

        // Should ignore empty lines
        assert_eq!(
            result,
            "Files changed: 2\nLines added: 10\nLines removed: 5"
        );
    }

    #[test]
    fn prompt_includes_change_summary_section() {
        let diff = StagedDiff {
            stat: "2 files changed, 10 insertions(+), 5 deletions(-)".to_string(),
            name_status: "M\tsrc/a.rs\nA\tsrc/b.rs".to_string(),
            diff: "test diff".to_string(),
        };
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("=== CHANGE SUMMARY ==="));
        assert!(prompt.contains("Files changed: 2"));
        assert!(prompt.contains("Lines added: 10"));
        assert!(prompt.contains("Lines removed: 5"));
    }

    #[test]
    fn prompt_change_summary_before_file_stats() {
        let diff = StagedDiff {
            stat: "1 file changed, 5 insertions(+)".to_string(),
            name_status: "M\tsrc/test.rs".to_string(),
            diff: "diff".to_string(),
        };
        let prompt = build_prompt(&diff);

        let summary_pos = prompt.find("=== CHANGE SUMMARY ===").unwrap();
        let stats_pos = prompt.find("=== FILE STATISTICS ===").unwrap();

        assert!(summary_pos < stats_pos);
    }

    #[test]
    fn prompt_truncates_large_diff() {
        let diff = StagedDiff {
            stat: "1 file changed, 1 insertion(+)".to_string(),
            name_status: "M\tsrc/test.rs".to_string(),
            diff: "x".repeat(10000),
        };
        let prompt = build_prompt(&diff);

        assert!(prompt.contains("... (diff truncated)"));
    }
}
