pub mod claude;
pub mod codex;
pub mod gemini;

use crate::error::AgentError;
use crate::types::AgentName;
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

// Regex patterns for response cleaning (compiled once with Lazy)
static MARKER_EXTRACT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<<<COMMIT_MESSAGE_START>>>([\s\S]*?)<<<COMMIT_MESSAGE_END>>>")
        .expect("valid regex pattern")
});

static CODE_BLOCK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"```(?:[a-z]*\n)?([\s\S]*?)```").expect("valid regex pattern"));

static PREAMBLE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(here is|here's|the commit message|commit message).*?:\s*")
        .expect("valid regex pattern")
});

static THINKING_TAGS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<thinking>[\s\S]*?</thinking>").expect("valid regex pattern"));

static MULTIPLE_NEWLINES: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\n{3,}").expect("valid regex pattern"));

/// Shared timeout for all agents (120 seconds)
pub(crate) const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Check if a CLI command exists in PATH
///
/// Uses `which` to check for command availability.
/// Returns NotFound error with the specified agent name if command doesn't exist.
pub(crate) async fn check_command_exists(
    command: &str,
    agent: AgentName,
) -> Result<(), AgentError> {
    let result = Command::new("which").arg(command).output().await;

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(AgentError::NotFound { agent }),
    }
}

/// Run a command with prompt via stdin
///
/// Spawns a process with the given command and arguments, writes the prompt
/// to stdin, and captures stdout. Used by Claude and Codex agents.
///
/// # Arguments
/// * `command` - The command to execute (e.g., "claude", "codex")
/// * `args` - Command-line arguments for the command
/// * `prompt` - The prompt text to write to stdin
/// * `agent` - The agent name for error reporting
///
/// # Returns
/// The stdout output from the command if successful
pub(crate) async fn run_command_with_stdin(
    command: &str,
    args: &[&str],
    prompt: &str,
    agent: AgentName,
) -> Result<String, AgentError> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| AgentError::ExecutionFailed {
            agent,
            stderr: e.to_string(),
        })?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent,
                stderr: format!("failed to write to stdin: {}", e),
            })?;
        stdin
            .flush()
            .await
            .map_err(|e| AgentError::ExecutionFailed {
                agent,
                stderr: format!("failed to flush stdin: {}", e),
            })?;
    }

    // Wait for process to complete
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| AgentError::ExecutionFailed {
            agent,
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AgentError::ExecutionFailed { agent, stderr });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}

/// Agent enum - closed set of supported AI agents
///
/// Uses enum dispatch (not trait objects) for:
/// - No heap allocation
/// - Exhaustive matching
/// - No async_trait needed
pub enum Agent {
    Claude(claude::ClaudeAgent),
    Codex(codex::CodexAgent),
    Gemini(gemini::GeminiAgent),
}

impl Agent {
    /// Execute the agent with the given prompt
    ///
    /// Dispatches to the appropriate agent implementation
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        match self {
            Self::Claude(agent) => agent.execute(prompt).await,
            Self::Codex(agent) => agent.execute(prompt).await,
            Self::Gemini(agent) => agent.execute(prompt).await,
        }
    }

    /// Get the name of this agent
    pub fn name(&self) -> AgentName {
        match self {
            Self::Claude(_) => AgentName::Claude,
            Self::Codex(_) => AgentName::Codex,
            Self::Gemini(_) => AgentName::Gemini,
        }
    }
}

impl From<AgentName> for Agent {
    fn from(name: AgentName) -> Self {
        match name {
            AgentName::Claude => Self::Claude(claude::ClaudeAgent),
            AgentName::Codex => Self::Codex(codex::CodexAgent),
            AgentName::Gemini => Self::Gemini(gemini::GeminiAgent),
        }
    }
}

/// Clean AI response by removing common artifacts
///
/// Pipeline (order matters):
/// 1. Extract between <<<COMMIT_MESSAGE_START>>> and <<<COMMIT_MESSAGE_END>>> markers
/// 2. Remove markdown code blocks (```...```)
/// 3. Remove preambles ("Here is the commit message:", etc.)
/// 4. Remove thinking tags (<thinking>...</thinking>)
/// 5. Collapse 3+ newlines to 2
/// 6. Trim whitespace
pub fn clean_ai_response(raw: &str) -> String {
    let mut cleaned = raw.to_string();

    // Step 1: Extract between markers if present
    if let Some(captures) = MARKER_EXTRACT.captures(&cleaned)
        && let Some(content) = captures.get(1)
    {
        cleaned = content.as_str().to_string();
    }

    // Step 2: Remove markdown code blocks (extract content inside)
    cleaned = CODE_BLOCK.replace_all(&cleaned, "$1").to_string();

    // Step 3: Remove preambles
    cleaned = PREAMBLE.replace_all(&cleaned, "").to_string();

    // Step 4: Remove thinking tags
    cleaned = THINKING_TAGS.replace_all(&cleaned, "").to_string();

    // Step 5: Collapse multiple newlines
    cleaned = MULTIPLE_NEWLINES.replace_all(&cleaned, "\n\n").to_string();

    // Step 6: Trim whitespace
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_ai_response_with_markers() {
        let input = "Some preamble\n<<<COMMIT_MESSAGE_START>>>feat: add feature<<<COMMIT_MESSAGE_END>>>\nSome postamble";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_code_blocks() {
        let input = "```\nfeat: add feature\n```";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_inline_code_block() {
        let input = "```feat: add feature```";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_language_code_block() {
        let input = "```text\nfeat: add feature\n```";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_preamble_here_is() {
        let input = "Here is the commit message:\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_preamble_heres() {
        let input = "Here's the commit message:\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_preamble_the_commit() {
        let input = "The commit message:\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_preamble_case_insensitive() {
        let input = "HERE IS THE COMMIT MESSAGE:\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_thinking_tags() {
        let input = "<thinking>Let me analyze this diff...</thinking>\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_thinking_multiline() {
        let input =
            "<thinking>\nLet me analyze...\nThis is a feature\n</thinking>\nfeat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_with_multiple_newlines() {
        let input = "feat: add feature\n\n\n\nSome body text";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature\n\nSome body text");
    }

    #[test]
    fn clean_ai_response_collapses_many_newlines() {
        let input = "feat: add feature\n\n\n\n\n\nSome body";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature\n\nSome body");
    }

    #[test]
    fn clean_ai_response_preserves_double_newlines() {
        let input = "feat: add feature\n\nSome body text";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature\n\nSome body text");
    }

    #[test]
    fn clean_ai_response_trims_whitespace() {
        let input = "  \n  feat: add feature  \n  ";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_complex_combination() {
        let input = r#"
<thinking>
Let me analyze this diff to create a commit message.
</thinking>

Here is the commit message:

```
<<<COMMIT_MESSAGE_START>>>
feat(api): add user authentication



This implements JWT-based authentication.
<<<COMMIT_MESSAGE_END>>>
```
"#;
        let result = clean_ai_response(input);
        assert_eq!(
            result,
            "feat(api): add user authentication\n\nThis implements JWT-based authentication."
        );
    }

    #[test]
    fn clean_ai_response_plain_message() {
        let input = "feat: add feature";
        let result = clean_ai_response(input);
        assert_eq!(result, "feat: add feature");
    }

    #[test]
    fn clean_ai_response_empty_string() {
        let input = "";
        let result = clean_ai_response(input);
        assert_eq!(result, "");
    }

    #[test]
    fn clean_ai_response_whitespace_only() {
        let input = "   \n   \n   ";
        let result = clean_ai_response(input);
        assert_eq!(result, "");
    }

    #[test]
    fn agent_name_returns_correct_variant_claude() {
        let agent = Agent::Claude(claude::ClaudeAgent);
        assert_eq!(agent.name(), AgentName::Claude);
    }

    #[test]
    fn agent_name_returns_correct_variant_codex() {
        let agent = Agent::Codex(codex::CodexAgent);
        assert_eq!(agent.name(), AgentName::Codex);
    }

    #[test]
    fn agent_name_returns_correct_variant_gemini() {
        let agent = Agent::Gemini(gemini::GeminiAgent);
        assert_eq!(agent.name(), AgentName::Gemini);
    }

    #[test]
    fn agent_from_agent_name_claude() {
        let agent = Agent::from(AgentName::Claude);
        assert_eq!(agent.name(), AgentName::Claude);
    }

    #[test]
    fn agent_from_agent_name_codex() {
        let agent = Agent::from(AgentName::Codex);
        assert_eq!(agent.name(), AgentName::Codex);
    }

    #[test]
    fn agent_from_agent_name_gemini() {
        let agent = Agent::from(AgentName::Gemini);
        assert_eq!(agent.name(), AgentName::Gemini);
    }

    #[test]
    fn agent_timeout_constant() {
        // Verify timeout is 120 seconds as specified
        assert_eq!(AGENT_TIMEOUT.as_secs(), 120);
    }

    #[tokio::test]
    async fn check_command_exists_with_existing_command() {
        // Test with a command that definitely exists
        let result = check_command_exists("echo", AgentName::Claude).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn check_command_exists_with_nonexistent_command() {
        // Test with a command that definitely doesn't exist
        let result = check_command_exists(
            "this-command-definitely-does-not-exist-99999",
            AgentName::Codex,
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::NotFound { agent } => assert_eq!(agent, AgentName::Codex),
            _ => panic!("expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn run_command_with_stdin_success() {
        // Test with echo command which will succeed
        let result = run_command_with_stdin("cat", &[], "test input", AgentName::Claude).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test input");
    }

    #[tokio::test]
    async fn run_command_with_stdin_command_not_found() {
        // Test with nonexistent command
        let result = run_command_with_stdin(
            "this-command-does-not-exist-12345",
            &[],
            "test",
            AgentName::Gemini,
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::ExecutionFailed { agent, .. } => assert_eq!(agent, AgentName::Gemini),
            _ => panic!("expected ExecutionFailed error"),
        }
    }

    #[tokio::test]
    async fn run_command_with_stdin_command_failure() {
        // Test with command that exits with error (ls on nonexistent directory)
        let result = run_command_with_stdin(
            "ls",
            &["/this/path/does/not/exist/surely"],
            "",
            AgentName::Claude,
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentError::ExecutionFailed { agent, stderr } => {
                assert_eq!(agent, AgentName::Claude);
                assert!(!stderr.is_empty());
            }
            _ => panic!("expected ExecutionFailed error"),
        }
    }
}
