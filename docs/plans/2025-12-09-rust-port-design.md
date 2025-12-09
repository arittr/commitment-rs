# commitment-rs Design

Port of [commitment](https://github.com/arittr/commitment) from TypeScript to Rust.

## Goals (in priority order)

1. **Learning** - Embrace idiomatic Rust patterns
2. **Distribution** - Single static binary, no runtime dependencies
3. **Performance** - Faster startup (bonus, not a driver)

## Scope

- Full port minus the eval framework
- Claude agent first, then Codex and Gemini
- Hook manager integration (lefthook, husky, simple-git-hooks, plain)

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Async runtime | Tokio | Learning goal; industry standard |
| CLI parsing | clap (derive) | Most popular, good docs, derive teaches macro patterns |
| Error handling | thiserror + anyhow | thiserror for domain errors, anyhow at CLI boundary |
| Agent abstraction | Enum dispatch | Known variants; no boxing; exhaustive matching |
| Validation | Newtype pattern | "Parse, don't validate" - invalid states unrepresentable |
| Terminal output | console + indicatif | Same author, work well together, widely used |
| Crate structure | lib.rs + main.rs | Standard pattern; testable, documentable |
| GitProvider | Sync trait | Git ops are fast/local; async overhead unnecessary |
| Generator | Functions, not struct | Minimal state; functions are more idiomatic |

## Project Structure

```
commitment-rs/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point (thin wrapper)
│   ├── lib.rs               # Public API, re-exports
│   │
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs          # Clap structs (Cli, Commands)
│   │   └── run.rs           # Command handlers
│   │
│   ├── agents/
│   │   ├── mod.rs           # Agent trait + re-exports
│   │   ├── claude.rs        # ClaudeAgent
│   │   ├── codex.rs         # (later)
│   │   ├── gemini.rs        # (later)
│   │   └── utils.rs         # clean_ai_response, validate_conventional_commit
│   │
│   ├── generator/
│   │   ├── mod.rs           # generate_commit_message() function
│   │   └── prompt.rs        # build_commit_message_prompt()
│   │
│   ├── git/
│   │   ├── mod.rs           # GitProvider trait + RealGitProvider
│   │   └── types.rs         # StagedDiff, GitStatus
│   │
│   ├── types/
│   │   ├── mod.rs
│   │   ├── agent_name.rs    # AgentName enum
│   │   └── commit.rs        # ConventionalCommit newtype
│   │
│   ├── hooks/
│   │   ├── mod.rs           # HookManager enum + detection
│   │   └── managers.rs      # Install functions per manager
│   │
│   └── error.rs             # AgentError, GitError, GeneratorError
```

## Core Types

### AgentName (enum)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentName {
    Claude,
    Codex,
    Gemini,
}

impl std::str::FromStr for AgentName {
    type Err = AgentNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            _ => Err(AgentNameError::Unknown(s.to_string())),
        }
    }
}
```

### ConventionalCommit (newtype)

```rust
#[derive(Debug, Clone)]
pub struct ConventionalCommit {
    raw: String,
}

impl ConventionalCommit {
    pub fn validate(msg: &str) -> Result<Self, CommitValidationError> {
        // Validate conventional commit format
        // type(scope): description
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn with_signature(self, sig: &str) -> Self {
        Self { raw: format!("{}\n\n{}", self.raw, sig) }
    }
}
```

## Error Types

```rust
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent `{agent}` not found in PATH")]
    NotFound { agent: AgentName },

    #[error("agent execution failed")]
    ExecutionFailed {
        #[source]
        source: std::io::Error,
        stderr: String,
    },

    #[error("agent returned malformed response")]
    MalformedResponse { raw: String },
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("not a git repository")]
    NotARepo,

    #[error("no staged changes")]
    NoStagedChanges,

    #[error("git command failed: {cmd}")]
    CommandFailed {
        cmd: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error(transparent)]
    Agent(#[from] AgentError),

    #[error(transparent)]
    Git(#[from] GitError),

    #[error("commit message validation failed: {0}")]
    ValidationFailed(#[from] CommitValidationError),
}
```

CLI layer handles error presentation with actionable hints.

## Agent Enum

Using an enum instead of trait objects - more idiomatic when variants are known upfront.
No `async_trait` crate needed, no boxing overhead, exhaustive matching.

```rust
// src/agents/mod.rs

pub struct ClaudeAgent;
pub struct CodexAgent;
pub struct GeminiAgent;

/// All supported AI agents
pub enum Agent {
    Claude(ClaudeAgent),
    Codex(CodexAgent),
    Gemini(GeminiAgent),
}

impl Agent {
    pub fn name(&self) -> AgentName {
        match self {
            Self::Claude(_) => AgentName::Claude,
            Self::Codex(_) => AgentName::Codex,
            Self::Gemini(_) => AgentName::Gemini,
        }
    }

    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        match self {
            Self::Claude(a) => a.execute(prompt).await,
            Self::Codex(a) => a.execute(prompt).await,
            Self::Gemini(a) => a.execute(prompt).await,
        }
    }

    pub fn clean_response(&self, raw: &str) -> String {
        // All agents use same cleaning for now
        clean_ai_response(raw)
    }
}

/// Orchestrates: execute → clean → validate
pub async fn generate(agent: &Agent, prompt: &str) -> Result<ConventionalCommit, AgentError> {
    let raw = agent.execute(prompt).await?;
    let cleaned = agent.clean_response(&raw);
    ConventionalCommit::validate(&cleaned)
        .map_err(|_| AgentError::MalformedResponse { raw: cleaned })
}

/// Factory from CLI arg
impl From<AgentName> for Agent {
    fn from(name: AgentName) -> Self {
        match name {
            AgentName::Claude => Self::Claude(ClaudeAgent),
            AgentName::Codex => Self::Codex(CodexAgent),
            AgentName::Gemini => Self::Gemini(GeminiAgent),
        }
    }
}
```

Each agent struct implements its own `execute`:

```rust
// src/agents/claude.rs

impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        // Check availability
        check_command_exists("claude").await?;

        // Run: echo "$prompt" | claude --print
        let output = tokio::process::Command::new("claude")
            .arg("--print")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        // Handle result
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(AgentError::ExecutionFailed {
                source: std::io::Error::other("non-zero exit"),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}
```

## GitProvider Trait

```rust
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;
    fn has_staged_changes(&self) -> Result<bool, GitError>;
    fn commit(&self, message: &str) -> Result<(), GitError>;
}

#[derive(Debug)]
pub struct StagedDiff {
    pub stat: String,
    pub name_status: String,
    pub diff: String,
}

pub struct RealGitProvider {
    cwd: PathBuf,
}

impl GitProvider for RealGitProvider {
    // Uses std::process::Command (sync)
}
```

## Core Functions

```rust
pub async fn generate_commit_message(
    git: &impl GitProvider,
    agent: &Agent,
    signature: Option<&str>,
) -> Result<ConventionalCommit, GeneratorError> {
    if !git.has_staged_changes()? {
        return Err(GeneratorError::Git(GitError::NoStagedChanges));
    }

    let diff = git.get_staged_diff()?;
    let prompt = build_commit_message_prompt(&diff);
    let mut commit = agents::generate(agent, &prompt).await?;

    if let Some(sig) = signature {
        commit = commit.with_signature(sig);
    }

    Ok(commit)
}

pub async fn generate_and_commit(
    git: &impl GitProvider,
    agent: &Agent,
    signature: Option<&str>,
) -> Result<(), GeneratorError> {
    let commit = generate_commit_message(git, agent, signature).await?;
    git.commit(commit.as_str())?;
    Ok(())
}
```

## CLI Structure

```rust
#[derive(Parser)]
#[command(name = "commitment")]
#[command(about = "AI-powered commit message generator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub generate: GenerateArgs,
}

#[derive(Subcommand)]
pub enum Commands {
    Init(InitArgs),
}

#[derive(clap::Args)]
pub struct GenerateArgs {
    #[arg(long, default_value = "claude")]
    pub agent: AgentName,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub message_only: bool,

    #[arg(long, short)]
    pub quiet: bool,

    #[arg(long, default_value = ".")]
    pub cwd: PathBuf,
}
```

## Hook Manager

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookManager {
    Lefthook,
    Husky,
    SimpleGitHooks,
    Plain,
}

impl HookManager {
    pub fn detect(project_dir: &Path) -> Option<Self>;
    pub fn install(&self, project_dir: &Path, agent: AgentName) -> Result<(), HookError>;
}
```

## Dependencies

```toml
[package]
name = "commitment"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "commitment"
path = "src/main.rs"

[lib]
path = "src/lib.rs"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process"] }
clap = { version = "4", features = ["derive"] }
thiserror = "2"
anyhow = "1"
console = "0.15"
indicatif = "0.17"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
serde_json = "1"

[dev-dependencies]
tokio-test = "0.4"
```

## Implementation Order

1. Project scaffolding (Cargo.toml, module structure)
2. Core types (AgentName, ConventionalCommit, errors)
3. GitProvider trait + RealGitProvider
4. Agent trait + ClaudeAgent
5. Prompt building
6. generate_commit_message() function
7. CLI (generate command)
8. Terminal UX (spinner, colors, error presentation)
9. Hook manager detection + installation (init command)
10. Add CodexAgent, GeminiAgent
