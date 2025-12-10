use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use std::str::FromStr;

static CONVENTIONAL_COMMIT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(feat|fix|docs|style|refactor|test|chore|perf|build|ci|revert)(\([a-z0-9\-]+\))?: .+",
    )
    .expect("valid regex pattern")
});

/// Agent names - closed set of supported AI CLIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentName {
    Claude,
    Codex,
    Gemini,
}

impl AgentName {
    /// Get human-readable display name for the agent
    ///
    /// Returns the capitalized name suitable for display in commit signatures
    /// and user-facing messages.
    ///
    /// # Examples
    /// ```
    /// # use commitment_rs::types::AgentName;
    /// assert_eq!(AgentName::Claude.display_name(), "Claude");
    /// assert_eq!(AgentName::Codex.display_name(), "Codex");
    /// assert_eq!(AgentName::Gemini.display_name(), "Gemini");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini",
        }
    }

    /// Get installation URL for the agent
    ///
    /// Returns the official installation documentation URL for the agent,
    /// useful for error messages when the agent is not found.
    ///
    /// # Examples
    /// ```
    /// # use commitment_rs::types::AgentName;
    /// assert!(AgentName::Claude.install_url().contains("anthropic.com"));
    /// assert!(AgentName::Codex.install_url().contains("github.com"));
    /// assert!(AgentName::Gemini.install_url().contains("github.com"));
    /// ```
    pub fn install_url(&self) -> &'static str {
        match self {
            Self::Claude => "https://docs.anthropic.com/en/docs/claude-cli",
            Self::Codex => "https://github.com/phughk/codex",
            Self::Gemini => "https://github.com/google/generative-ai-cli",
        }
    }
}

impl FromStr for AgentName {
    type Err = AgentNameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            _ => Err(AgentNameParseError {
                invalid: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for AgentName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Codex => write!(f, "codex"),
            Self::Gemini => write!(f, "gemini"),
        }
    }
}

/// Error when parsing agent name from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNameParseError {
    invalid: String,
}

impl fmt::Display for AgentNameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid agent name '{}' (expected: claude, codex, gemini)",
            self.invalid
        )
    }
}

impl std::error::Error for AgentNameParseError {}

/// Validated conventional commit message
///
/// Can only be constructed via validate(). Once you have this type,
/// it's guaranteed to be a valid conventional commit format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConventionalCommit {
    raw: String, // Private - enforces validation on construction
}

impl ConventionalCommit {
    /// Validate and construct a conventional commit message
    ///
    /// Format: `<type>(<scope>): <description>`
    /// - type: feat, fix, docs, style, refactor, test, chore, perf, build, ci, revert
    /// - scope: optional, e.g., (api), (cli)
    /// - description: required
    #[must_use = "validation result should be checked"]
    pub fn validate(msg: &str) -> Result<Self, CommitValidationError> {
        let msg = msg.trim();

        if msg.is_empty() {
            return Err(CommitValidationError::Empty);
        }

        // Use static compiled regex pattern for performance
        if !CONVENTIONAL_COMMIT_PATTERN.is_match(msg) {
            return Err(CommitValidationError::InvalidFormat {
                message: msg.to_string(),
            });
        }

        Ok(Self {
            raw: msg.to_string(),
        })
    }

    /// Get the commit message as a string slice
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

/// Enables `&commit` to be used where `&str` is expected
///
/// Example: `fn process(s: impl AsRef<str>)` accepts `&commit`
impl AsRef<str> for ConventionalCommit {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

/// Enables transparent string access: `*commit` yields `&str`
///
/// This allows direct string operations without explicit conversion:
/// - `commit.len()` works directly
/// - `commit.contains("feat")` works directly
///
/// Note: Some prefer explicit `.as_str()` calls for clarity. We provide both:
/// - `Deref` for ergonomics
/// - `.as_str()` for explicitness
///
/// The private `raw` field still enforces validation-on-construction.
impl std::ops::Deref for ConventionalCommit {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

/// Error when validating commit message
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitValidationError {
    Empty,
    InvalidFormat { message: String },
}

impl fmt::Display for CommitValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "commit message is empty"),
            Self::InvalidFormat { message } => write!(
                f,
                "invalid conventional commit format: '{}'\nExpected: <type>(<scope>): <description>",
                message
            ),
        }
    }
}

impl std::error::Error for CommitValidationError {}

/// Staged git diff data carrier
///
/// Plain struct - no validation needed, just holds git output
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StagedDiff {
    /// Output of `git diff --cached --stat`
    pub stat: String,
    /// Output of `git diff --cached --name-status`
    pub name_status: String,
    /// Output of `git diff --cached`
    pub diff: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude_agent_name() {
        assert_eq!("claude".parse::<AgentName>().unwrap(), AgentName::Claude);
        assert_eq!("Claude".parse::<AgentName>().unwrap(), AgentName::Claude);
        assert_eq!("CLAUDE".parse::<AgentName>().unwrap(), AgentName::Claude);
    }

    #[test]
    fn parses_codex_agent_name() {
        assert_eq!("codex".parse::<AgentName>().unwrap(), AgentName::Codex);
        assert_eq!("Codex".parse::<AgentName>().unwrap(), AgentName::Codex);
    }

    #[test]
    fn parses_gemini_agent_name() {
        assert_eq!("gemini".parse::<AgentName>().unwrap(), AgentName::Gemini);
        assert_eq!("Gemini".parse::<AgentName>().unwrap(), AgentName::Gemini);
    }

    #[test]
    fn rejects_invalid_agent_name() {
        let result = "invalid".parse::<AgentName>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("claude, codex, gemini"));
    }

    #[test]
    fn rejects_empty_agent_name() {
        assert!("".parse::<AgentName>().is_err());
    }

    #[test]
    fn agent_name_display() {
        assert_eq!(AgentName::Claude.to_string(), "claude");
        assert_eq!(AgentName::Codex.to_string(), "codex");
        assert_eq!(AgentName::Gemini.to_string(), "gemini");
    }

    #[test]
    fn validates_feat_commit() {
        let result = ConventionalCommit::validate("feat: add new feature");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "feat: add new feature");
    }

    #[test]
    fn validates_fix_commit() {
        let result = ConventionalCommit::validate("fix: resolve bug");
        assert!(result.is_ok());
    }

    #[test]
    fn validates_commit_with_scope() {
        let result = ConventionalCommit::validate("feat(api): add endpoint");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "feat(api): add endpoint");
    }

    #[test]
    fn validates_commit_with_hyphenated_scope() {
        let result = ConventionalCommit::validate("fix(error-handling): improve validation");
        assert!(result.is_ok());
    }

    #[test]
    fn validates_all_conventional_types() {
        let types = vec![
            "feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "build", "ci",
            "revert",
        ];
        for type_ in types {
            let msg = format!("{}: test description", type_);
            assert!(
                ConventionalCommit::validate(&msg).is_ok(),
                "Failed to validate type: {}",
                type_
            );
        }
    }

    #[test]
    fn rejects_empty_commit() {
        let result = ConventionalCommit::validate("");
        assert!(matches!(result, Err(CommitValidationError::Empty)));
    }

    #[test]
    fn rejects_whitespace_only_commit() {
        let result = ConventionalCommit::validate("   \n  ");
        assert!(matches!(result, Err(CommitValidationError::Empty)));
    }

    #[test]
    fn rejects_commit_without_type() {
        let result = ConventionalCommit::validate("just a description");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_commit_without_colon() {
        let result = ConventionalCommit::validate("feat add feature");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_commit_without_description() {
        let result = ConventionalCommit::validate("feat:");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_invalid_type() {
        let result = ConventionalCommit::validate("invalid: description");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_uppercase_type() {
        let result = ConventionalCommit::validate("FEAT: description");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn rejects_uppercase_scope() {
        let result = ConventionalCommit::validate("feat(API): description");
        assert!(matches!(
            result,
            Err(CommitValidationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn trims_whitespace() {
        let result = ConventionalCommit::validate("  feat: test  \n");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "feat: test");
    }

    #[test]
    fn staged_diff_default() {
        let diff = StagedDiff::default();
        assert_eq!(diff.stat, "");
        assert_eq!(diff.name_status, "");
        assert_eq!(diff.diff, "");
    }

    #[test]
    fn staged_diff_construction() {
        let diff = StagedDiff {
            stat: "1 file changed, 10 insertions(+)".to_string(),
            name_status: "A\tsrc/test.rs".to_string(),
            diff: "@@ -0,0 +1,10 @@".to_string(),
        };
        assert_eq!(diff.stat, "1 file changed, 10 insertions(+)");
        assert_eq!(diff.name_status, "A\tsrc/test.rs");
        assert_eq!(diff.diff, "@@ -0,0 +1,10 @@");
    }

    #[test]
    fn staged_diff_clone() {
        let diff1 = StagedDiff {
            stat: "test".to_string(),
            name_status: "test".to_string(),
            diff: "test".to_string(),
        };
        let diff2 = diff1.clone();
        assert_eq!(diff1, diff2);
    }

    #[test]
    fn agent_name_display_name_claude() {
        assert_eq!(AgentName::Claude.display_name(), "Claude");
    }

    #[test]
    fn agent_name_display_name_codex() {
        assert_eq!(AgentName::Codex.display_name(), "Codex");
    }

    #[test]
    fn agent_name_display_name_gemini() {
        assert_eq!(AgentName::Gemini.display_name(), "Gemini");
    }

    #[test]
    fn agent_name_install_url_claude() {
        let url = AgentName::Claude.install_url();
        assert_eq!(url, "https://docs.anthropic.com/en/docs/claude-cli");
        assert!(url.contains("anthropic.com"));
    }

    #[test]
    fn agent_name_install_url_codex() {
        let url = AgentName::Codex.install_url();
        assert_eq!(url, "https://github.com/phughk/codex");
        assert!(url.contains("github.com"));
    }

    #[test]
    fn agent_name_install_url_gemini() {
        let url = AgentName::Gemini.install_url();
        assert_eq!(url, "https://github.com/google/generative-ai-cli");
        assert!(url.contains("github.com"));
    }

    #[test]
    fn agent_name_display_name_all_variants() {
        // Ensure all variants return non-empty display names
        let agents = vec![AgentName::Claude, AgentName::Codex, AgentName::Gemini];
        for agent in agents {
            assert!(!agent.display_name().is_empty());
        }
    }

    #[test]
    fn agent_name_install_url_all_variants() {
        // Ensure all variants return valid URLs (start with https://)
        let agents = vec![AgentName::Claude, AgentName::Codex, AgentName::Gemini];
        for agent in agents {
            let url = agent.install_url();
            assert!(url.starts_with("https://"));
            assert!(!url.is_empty());
        }
    }

    #[test]
    fn conventional_commit_deref_works() {
        // Test that Deref allows direct string operations
        let commit = ConventionalCommit::validate("feat: add feature").unwrap();

        // These work because of Deref<Target=str>
        assert_eq!(commit.len(), 17);
        assert!(commit.contains("feat"));
        assert!(commit.starts_with("feat:"));
        assert_eq!(&commit[0..4], "feat");
    }

    #[test]
    fn conventional_commit_as_ref_works() {
        // Test that AsRef<str> allows passing to generic functions
        let commit = ConventionalCommit::validate("fix: resolve bug").unwrap();

        // Helper function that accepts AsRef<str>
        fn process_string(s: impl AsRef<str>) -> usize {
            s.as_ref().len()
        }

        // Should work with &commit
        assert_eq!(process_string(&commit), 16);
        assert_eq!(process_string(commit.as_ref()), 16);
    }

    #[test]
    fn conventional_commit_trait_implementations() {
        // Verify all three ways to access the string work
        let commit = ConventionalCommit::validate("chore: update deps").unwrap();

        // as_str() - explicit method
        let s1: &str = commit.as_str();
        assert_eq!(s1, "chore: update deps");

        // as_ref() - AsRef trait
        let s2: &str = commit.as_ref();
        assert_eq!(s2, "chore: update deps");

        // deref - Deref trait (implicit via &*)
        let s3: &str = &*commit;
        assert_eq!(s3, "chore: update deps");

        // All should be equal
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }
}
