# Patterns

## Type Patterns

### Newtypes for Validation

**Mandatory:** Domain types requiring validation use the newtype pattern.

```rust
// Private inner field - can only construct via validate()
pub struct ConventionalCommit {
    raw: String,
}

impl ConventionalCommit {
    pub fn validate(msg: &str) -> Result<Self, Error> { ... }
    pub fn as_str(&self) -> &str { &self.raw }
}

// Ergonomic access via Deref and AsRef
impl Deref for ConventionalCommit {
    type Target = str;
    fn deref(&self) -> &Self::Target { &self.raw }
}

impl AsRef<str> for ConventionalCommit {
    fn as_ref(&self) -> &str { &self.raw }
}
```

**Apply to:** `ConventionalCommit`, any validated domain type

**Don't apply to:** Simple data carriers like `StagedDiff`

**Why:** Once you have a `ConventionalCommit`, it's guaranteed valid. No runtime checks downstream.

### Enums for Closed Sets

**Mandatory:** When variants are known at compile time, use enums.

```rust
pub enum AgentName {
    Claude,
    Codex,
    Gemini,
}

pub enum Agent {
    Claude(ClaudeAgent),
    Codex(CodexAgent),
    Gemini(GeminiAgent),
}
```

**Why:** Exhaustive matching catches missing cases. No heap allocation.

### Enum Methods for Variant-Specific Data

**Mandatory:** When each variant needs associated data, add methods to the enum.

```rust
impl AgentName {
    /// Human-readable name for display
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini",
        }
    }

    /// Installation URL for error messages
    pub fn install_url(&self) -> &'static str {
        match self {
            Self::Claude => "https://docs.anthropic.com/en/docs/claude-cli",
            Self::Codex => "https://github.com/phughk/codex",
            Self::Gemini => "https://github.com/google/generative-ai-cli",
        }
    }
}
```

**Why:** Centralizes variant-specific data. Adding a variant forces handling all cases.

### Traits for Abstraction Boundaries

**Use traits when you need:**
- Dependency injection (testing)
- Multiple implementations

```rust
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;
}

// Production
pub struct RealGitProvider { cwd: PathBuf }

// Test
#[cfg(test)]
pub struct MockGitProvider { diff: StagedDiff }
```

## Function Patterns

### Prefer Functions to Methods

**Mandatory:** When there's no meaningful state, use free functions.

```rust
// Preferred
pub async fn generate_commit_message(
    git: &impl GitProvider,
    agent: &Agent,
) -> Result<ConventionalCommit, GeneratorError>

// Avoid (unless state is meaningful)
impl Generator {
    pub async fn generate(&self) -> Result<...>
}
```

**Why:** Rust idiom is "just use a function" unless you need encapsulation.

### Shared Infrastructure in Parent Modules

**Mandatory:** Common utilities go in the parent module, not duplicated in children.

```rust
// agents/mod.rs - shared by all agents
pub(crate) const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) async fn check_command_exists(
    command: &str,
    agent: AgentName,
) -> Result<(), AgentError> { ... }

pub(crate) async fn run_command_with_stdin(
    command: &str,
    args: &[&str],
    prompt: &str,
    agent: AgentName,
) -> Result<String, AgentError> { ... }

// agents/claude.rs - uses shared infra
impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        check_command_exists("claude", AgentName::Claude).await?;
        let output = run_command_with_stdin(
            "claude", &["--print", "-p"], prompt, AgentName::Claude
        ).await?;
        Ok(clean_ai_response(&output))
    }
}
```

**Why:** DRY. Changes to timeout/error handling happen once. Agent files stay minimal (~20 lines).

### Generic Over Traits

**Use `impl Trait` for static dispatch:**

```rust
pub fn build_prompt(diff: &StagedDiff) -> String  // concrete
pub async fn generate(git: &impl GitProvider, ...)  // generic
```

**Use `&dyn Trait` only when runtime polymorphism is required.**

## Error Patterns

### Domain Errors (`thiserror`)

**Mandatory:** Structured errors that can be matched.

```rust
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent `{agent}` not found in PATH")]
    NotFound { agent: AgentName },

    #[error("agent execution failed")]
    ExecutionFailed {
        agent: AgentName,
        stderr: String,
    },
}
```

**Rules:**
- Errors describe WHAT happened, not how to fix
- Include relevant context (agent name, stderr)
- Use `#[source]` to chain underlying errors
- Use `#[from]` for automatic conversion

### CLI Errors (`anyhow`)

**Mandatory:** At the CLI boundary, convert to `anyhow::Result`.

```rust
pub async fn run_generate(args: Args) -> anyhow::Result<()> {
    let commit = generate_commit_message(...).await?;
    Ok(())
}
```

**Rules:**
- Add user-friendly context with `.context()`
- Match on domain errors to add "how to fix" hints
- Use `AgentName::install_url()` for installation help

## Async Patterns

### Sync for Local Operations

**Mandatory:** Git operations are sync (fast and local).

```rust
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;  // sync
}
```

### Async for External Processes

**Mandatory:** AI CLI execution is async (takes time).

```rust
impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        tokio::process::Command::new("claude")...
    }
}
```

### Tokio Runtime

**Use `#[tokio::main]` in main.rs:**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli::run(cli).await
}
```

## Prompt Patterns

### Diff Truncation

**Mandatory:** Truncate large diffs to prevent token limit issues.

```rust
const MAX_DIFF_LENGTH: usize = 8000;

fn truncate_diff(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_LENGTH {
        return diff.to_string();
    }
    // Find UTF-8 char boundary, append indicator
    format!("{}\n... (diff truncated)", &diff[..boundary])
}
```

**Why:** AI models have context limits. Better to truncate cleanly than fail.

### Change Summary

**Include structured summary before raw diff:**

```rust
fn parse_change_summary(stat: &str, name_status: &str) -> String {
    format!(
        "Files changed: {}\nLines added: {}\nLines removed: {}",
        file_count, lines_added, lines_removed
    )
}
```

**Why:** Gives AI quick overview before diving into diff details.

## Anti-Patterns

### No Complex Diff Analysis

The TypeScript version has `analyzeCodeChanges()` that detects patterns in diffs.
**We don't do this.** The AI sees the diff and can analyze it better than regex.

```rust
// WRONG - don't port this from TS
fn analyze_code_changes(diff: &str) -> ChangeAnalysis {
    // counting functions, detecting tests, etc.
}

// RIGHT - just build prompt with raw diff
fn build_prompt(diff: &StagedDiff) -> String {
    format!("Generate commit message for:\n{}", diff.diff)
}
```

### No Trait Objects for Agents

Agents are a closed set. Use enum dispatch:

```rust
// WRONG
fn generate(agent: &dyn Agent) -> Result<...>

// RIGHT
fn generate(agent: &Agent) -> Result<...>  // Agent is enum
```

### No Struct for Orchestration

Don't create a `Generator` struct just to hold dependencies:

```rust
// WRONG
struct Generator<G: GitProvider> {
    git: G,
    agent: Agent,
}

// RIGHT - just a function
async fn generate_commit_message(
    git: &impl GitProvider,
    agent: &Agent,
) -> Result<...>
```

### No `unwrap()` in Library Code

Use `?` or `expect()` with context:

```rust
// WRONG
let output = command.output().unwrap();

// RIGHT
let output = command.output()?;

// ACCEPTABLE (with context)
let regex = Regex::new(PATTERN).expect("valid regex pattern");
```

### No Duplicated Agent Logic

Don't copy-paste between agent files:

```rust
// WRONG - each agent has its own command runner
impl ClaudeAgent {
    async fn run_command(&self, ...) { /* 50 lines */ }
}
impl CodexAgent {
    async fn run_command(&self, ...) { /* same 50 lines */ }
}

// RIGHT - shared in parent module
// agents/mod.rs
pub(crate) async fn run_command_with_stdin(...) { /* once */ }
```
