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
| Agent abstraction | Trait with defaults | Idiomatic Rust; replaces TS class inheritance |
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

## Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> AgentName;

    async fn execute(&self, prompt: &str) -> Result<String, AgentError>;

    fn clean_response(&self, raw: &str) -> String {
        clean_ai_response(raw)
    }
}

/// Template method as free function (not overridable)
pub async fn generate(
    agent: &dyn Agent,
    prompt: &str,
) -> Result<ConventionalCommit, AgentError> {
    let raw = agent.execute(prompt).await?;
    let cleaned = agent.clean_response(&raw);
    ConventionalCommit::validate(&cleaned)
        .map_err(|e| AgentError::MalformedResponse { raw: cleaned })
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
    agent: &dyn Agent,
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
    agent: &dyn Agent,
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
async-trait = "0.1"
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
