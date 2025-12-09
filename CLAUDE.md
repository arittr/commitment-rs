# CLAUDE.md

This file provides guidance to Claude Code and other AI coding agents when working with code in this repository.

## Project Overview

**commitment-rs** is a Rust port of [commitment](https://github.com/arittr/commitment) - an AI-powered commit message generator. It uses local AI CLI tools (Claude, Codex, or Gemini) to generate conventional commit messages from git diffs.

**Design Philosophy:** Functions over structs. Enum dispatch over trait objects. Newtypes for validation. Let the AI analyze diffs (no complex pattern detection in code).

**Constitution:** See `@docs/constitutions/current/` for architectural rules and patterns.

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Run
cargo run                # Run with default args
cargo run -- --help      # Show help
cargo run -- --agent claude --dry-run

# Test
cargo test               # Run all tests
cargo test -- --nocapture  # Show println output
cargo test <name>        # Run specific test

# Code Quality
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting (CI)
cargo clippy             # Lint
cargo clippy -- -D warnings  # Fail on warnings (CI)

# Check
cargo check              # Fast type check
```

## Architecture Overview

### Layered Architecture

```
┌─────────────────────────────────────────┐
│              CLI Layer                  │  main.rs, cli.rs
│  • clap argument parsing                │
│  • Error formatting for users           │
│  • Progress display (indicatif)         │
└─────────────┬───────────────────────────┘
              │ validated args
              ▼
┌─────────────────────────────────────────┐
│           Core Layer                    │  lib.rs
│  • generate_commit_message()            │
│  • Orchestrates git → prompt → agent    │
└─────────────┬───────────────────────────┘
              │
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────────┐  ┌─────────────────────────┐
│ Git Module  │  │    Agent Module         │
│ git.rs      │  │ agents/mod.rs           │
│ • GitProvider│  │ • Agent enum            │
│   trait     │  │ • generate() function   │
│ • StagedDiff│  │ • clean_ai_response()   │
└─────────────┘  └─────────────────────────┘
```

### Module Structure

```
src/
├── main.rs          # Entry point, #[tokio::main]
├── lib.rs           # Public API: generate_commit_message()
├── cli.rs           # clap args + command handlers
├── agents/
│   ├── mod.rs       # Agent enum, generate(), clean_ai_response()
│   └── claude.rs    # ClaudeAgent (codex.rs, gemini.rs later)
├── git.rs           # GitProvider trait, RealGitProvider, StagedDiff
├── prompt.rs        # build_prompt() - simple template
├── types.rs         # AgentName enum, ConventionalCommit newtype
├── error.rs         # AgentError, GitError, GeneratorError
└── hooks/
    ├── mod.rs       # HookManager enum + detect/install
    └── managers.rs  # Per-manager install logic
```

## Key Patterns

### 1. Enum Dispatch (Not Trait Objects)

Agents use enum dispatch - no `dyn Agent`, no `async_trait`, no boxing:

```rust
pub enum Agent {
    Claude(ClaudeAgent),
    Codex(CodexAgent),
    Gemini(GeminiAgent),
}

impl Agent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        match self {
            Self::Claude(a) => a.execute(prompt).await,
            Self::Codex(a) => a.execute(prompt).await,
            Self::Gemini(a) => a.execute(prompt).await,
        }
    }
}
```

**Why:** Known variants, no heap allocation, exhaustive matching catches missing cases.

### 2. Newtype Validation ("Parse, Don't Validate")

Invalid states are unrepresentable:

```rust
pub struct ConventionalCommit {
    raw: String,  // Private - can only construct via validate()
}

impl ConventionalCommit {
    pub fn validate(msg: &str) -> Result<Self, CommitValidationError> {
        // Regex check for conventional commit format
    }
}
```

**Why:** Once you have a `ConventionalCommit`, it's guaranteed valid. No runtime checks downstream.

### 3. Functions Over Structs

No `Generator` class - just functions:

```rust
pub async fn generate_commit_message(
    git: &impl GitProvider,
    agent: &Agent,
    signature: Option<&str>,
) -> Result<ConventionalCommit, GeneratorError>
```

**Why:** Minimal state. Rust idiom is "just use a function" unless you need encapsulation.

### 4. Sync Git, Async Agent

- `GitProvider` is sync - git operations are fast and local
- `Agent::execute()` is async - waiting on AI CLI takes time

```rust
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;  // sync
    fn has_staged_changes(&self) -> Result<bool, GitError>;     // sync
    fn commit(&self, message: &str) -> Result<(), GitError>;    // sync
}

impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        // tokio::process::Command
    }
}
```

### 5. Error Handling: thiserror + anyhow

- Domain errors use `thiserror` (structured, matchable)
- CLI boundary uses `anyhow` (easy error chaining)

```rust
// Domain error (src/error.rs)
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent `{agent}` not found in PATH")]
    NotFound { agent: AgentName },
}

// CLI layer (src/cli.rs)
pub async fn run_generate(args: GenerateArgs) -> anyhow::Result<()> {
    // AgentError converts to anyhow::Error via ?
}
```

## Code Style

### Naming

- `snake_case` for functions, variables, modules, files
- `PascalCase` for types, enums, structs
- `SCREAMING_SNAKE_CASE` for constants

### Formatting

Run `cargo fmt` before committing. CI checks with `cargo fmt -- --check`.

### Linting

Run `cargo clippy`. CI fails on warnings with `cargo clippy -- -D warnings`.

### Comments

Explain WHY, not WHAT:

```rust
// Claude requires --print flag to output without interactive confirmation
.arg("--print")
```

## Testing

### Unit Tests

Co-locate with source using `#[cfg(test)]` modules:

```rust
// src/types.rs
pub struct AgentName { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_agent() {
        assert!("claude".parse::<AgentName>().is_ok());
    }
}
```

### Integration Tests

Place in `tests/` directory for tests that span modules.

### Mocking

Use traits for dependency injection:

```rust
pub trait GitProvider { ... }

#[cfg(test)]
struct MockGitProvider { ... }
```

## Error Messages

Errors describe WHAT happened. CLI layer adds HOW to fix:

```rust
// Error type (factual)
#[error("agent `{agent}` not found in PATH")]
NotFound { agent: AgentName }

// CLI layer (user-friendly)
Err(GeneratorError::Agent(AgentError::NotFound { agent })) => {
    eprintln!("{}: agent `{}` not found", style("error").red(), agent);
    eprintln!("  → Install: https://docs.anthropic.com/claude-cli");
}
```

## Dependencies

Core dependencies (from Cargo.toml):

- `tokio` - async runtime (process spawning)
- `clap` - CLI parsing (derive macros)
- `thiserror` - domain error types
- `anyhow` - CLI error handling
- `console` + `indicatif` - terminal UX
- `regex` + `once_cell` - response cleaning
- `serde` + `serde_json` + `serde_yaml` - hook config parsing

## Git Workflow

### Commits

Use commitment itself (once implemented) or follow conventional commits:

```
<type>(<scope>): <description>

Types: feat, fix, docs, style, refactor, test, chore, perf, build, ci
```

### Branches

Use descriptive branch names:

```bash
git checkout -b feat/add-codex-agent
git checkout -b fix/timeout-handling
```

## Adding a New Agent

1. Create `src/agents/<name>.rs` with struct + execute method
2. Add variant to `Agent` enum in `src/agents/mod.rs`
3. Add variant to `AgentName` enum in `src/types.rs`
4. Update `From<AgentName> for Agent` impl
5. Update match arms in `Agent::execute()`, `Agent::name()`, etc.
6. Add tests

Example agent (~30 lines):

```rust
// src/agents/codex.rs
pub struct CodexAgent;

impl CodexAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        check_command_exists("codex").await?;
        run_command("codex", &["exec", "--skip-git-repo-check"], Some(prompt), TIMEOUT).await
    }
}
```

## Self-Dogfooding

Once implemented, use commitment-rs for its own commits:

```bash
git add .
cargo run  # generates commit message
```
