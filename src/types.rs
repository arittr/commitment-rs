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
}
