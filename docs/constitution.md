# Constitution v1

Architectural rules for commitment-rs. Violations require either refactoring or a new constitution version.

## Core Principles

1. **Idiomatic Rust** - Follow Rust conventions, not TypeScript patterns
2. **Functions over structs** - Use structs only when state is meaningful
3. **Parse, don't validate** - Invalid states should be unrepresentable
4. **Let AI analyze** - No complex diff analysis in code; AI sees the diff

## Architectural Layers

```
CLI (main.rs, cli.rs)
    ↓ validated args
Core (lib.rs)
    ↓
┌───┴───┐
Git     Agents
```

**Dependency rule:** Upper layers may depend on lower layers. Never the reverse.

## Module Responsibilities

### CLI (`main.rs`, `cli.rs`)

**Allowed:**
- Parse clap arguments
- Format errors for users (add "how to fix" hints)
- Display progress (spinner, colors)
- Call `generate_commit_message()` from lib

**Forbidden:**
- Business logic
- Direct git operations
- Agent instantiation (use `Agent::from(name)`)

### Core (`lib.rs`)

**Allowed:**
- Orchestrate: git diff → prompt → agent → validate
- Export public API

**Forbidden:**
- CLI concerns (output formatting, exit codes)
- Implementation details of git/agents

### Agents (`agents/`)

**Allowed:**
- Execute AI CLI commands
- Clean AI responses
- Agent-specific error handling

**Forbidden:**
- Git operations
- Prompt construction
- Business logic

### Git (`git.rs`)

**Allowed:**
- Run git commands
- Parse git output into structs
- Abstract via `GitProvider` trait

**Forbidden:**
- AI operations
- Prompt construction

## Type Patterns

### Newtypes for Validation

Domain types that require validation use the newtype pattern:

```rust
// Private inner field - can only construct via validate()
pub struct ConventionalCommit {
    raw: String,
}

impl ConventionalCommit {
    pub fn validate(msg: &str) -> Result<Self, Error> { ... }
    pub fn as_str(&self) -> &str { &self.raw }
}
```

**Apply to:** `ConventionalCommit`, any validated domain type

**Don't apply to:** Simple data carriers like `StagedDiff`

### Enums for Closed Sets

When variants are known at compile time, use enums:

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

### Traits for Abstraction Boundaries

Use traits when you need:
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

## Error Patterns

### Domain Errors (`thiserror`)

Structured errors that can be matched:

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
}
```

**Rules:**
- Errors describe WHAT happened, not how to fix
- Use `#[source]` to chain underlying errors
- Use `#[from]` for automatic conversion

### CLI Errors (`anyhow`)

At the CLI boundary, convert to `anyhow::Result`:

```rust
pub async fn run_generate(args: Args) -> anyhow::Result<()> {
    let commit = generate_commit_message(...).await?;
    Ok(())
}
```

**Rules:**
- Add user-friendly context with `.context()`
- Match on domain errors to add "how to fix" hints

## Async Patterns

### Sync for Local Operations

Git operations are fast and local - no async needed:

```rust
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;  // sync
}
```

### Async for External Processes

AI CLI execution takes time - use async:

```rust
impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        tokio::process::Command::new("claude")...
    }
}
```

### Tokio Runtime

Use `#[tokio::main]` in main.rs:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli::run(cli).await
}
```

## Function Patterns

### Prefer Functions to Methods

When there's no meaningful state, use free functions:

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

### Generic Over Traits

Use `impl Trait` for static dispatch:

```rust
pub fn build_prompt(diff: &StagedDiff) -> String  // concrete
pub async fn generate(git: &impl GitProvider, ...)  // generic
```

Use `&dyn Trait` only when runtime polymorphism is required.

## Testing Patterns

### Co-located Unit Tests

Tests live with their source:

```rust
// src/types.rs
pub enum AgentName { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude() { ... }
}
```

### Trait-based Mocking

Use traits for dependencies, mock in tests:

```rust
#[cfg(test)]
mod tests {
    struct MockGitProvider {
        staged_diff: StagedDiff,
    }

    impl GitProvider for MockGitProvider {
        fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
            Ok(self.staged_diff.clone())
        }
    }
}
```

### Async Tests

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn generates_commit_message() {
    let result = generate_commit_message(&mock_git, &agent).await;
    assert!(result.is_ok());
}
```

## File Organization

### Module Files

- `mod.rs` for module roots (or `module_name.rs` with `module_name/` dir)
- One primary type per file when >100 lines
- Group related items in single file when <100 lines

### Naming

- `snake_case.rs` for files
- `snake_case` for modules
- `PascalCase` for types
- `SCREAMING_SNAKE_CASE` for constants

### Visibility

- `pub` only for public API
- `pub(crate)` for crate-internal items
- Private by default

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

## Evolution

This is constitution v1. To change:

1. **Clarifications:** Edit in place (non-breaking)
2. **New patterns:** Propose in PR, update constitution
3. **Breaking changes:** Create v2, update symlink

When in doubt: Follow the constitution, or propose an amendment.
