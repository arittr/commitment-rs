---
runId: 0e67da
feature: parity-enhancements
created: 2025-12-09
status: draft
---

# Feature: TypeScript Parity Enhancements

**Status**: Draft
**Created**: 2025-12-09

## Problem Statement

**Current State:**
commitment-rs is functionally complete but lacks several UX and robustness features present in the TypeScript commitment project:
- No diff truncation (risks token limit issues on large changes)
- Git diff missing `--unified=3` and `--ignore-space-change` flags (more tokens, noisier diffs)
- No staged files display before generation
- Prompt lacks change summary metadata (file count, lines +/-)
- Lefthook installer overwrites existing hooks without warning

Additionally, code quality issues exist:
- `check_command_exists()` duplicated 3x across agent modules (~30 lines each)
- `AGENT_TIMEOUT` constant defined 3x identically
- `resolve_git_dir()` duplicated in `git.rs` and `hooks/managers.rs`
- Stdin writing pattern duplicated in Claude and Codex agents
- Agent metadata (display name, install URL) scattered across files

**Desired State:**
Full feature parity with TypeScript commitment CLI, improved commit message quality, and cleaner DRY-compliant codebase.

**Gap:**
Five enhancement areas plus code quality refactoring: diff truncation, git diff flags, staged files display, change summary, lefthook safety, and DRY compliance (~160 lines of duplication to eliminate).

## Requirements

> **Note**: All features must follow @docs/constitutions/current/

### Functional Requirements

- FR1: Truncate diff content to 8000 characters to prevent token limit issues
- FR2: Use optimized git diff flags: `--unified=3 --ignore-space-change` (reduces tokens, cleaner diffs)
- FR3: Display staged files list before generation (respecting --quiet/--message-only flags)
- FR4: Add change summary (file count, lines added/removed) to prompt
- FR5: Warn and skip when lefthook config already has prepare-commit-msg hook

### Non-Functional Requirements

- NFR1: No complex diff analysis (per @docs/constitutions/current/patterns.md anti-patterns)
- NFR2: Maintain existing test coverage - all new functions must have unit tests
- NFR3: No new dependencies - use existing regex, once_cell for parsing
- NFR4: Eliminate code duplication - shared utilities in appropriate modules
- NFR5: Each agent file should be ~15-20 lines (down from ~130 lines currently)
- NFR6: Follow Rust idioms - proper trait implementations, `#[must_use]`, documentation

## Architecture

> **Layer boundaries**: @docs/constitutions/current/architecture.md
> **Required patterns**: @docs/constitutions/current/patterns.md

### Components

**Modified Files:**

| File | Changes |
|------|---------|
| `src/git.rs` | Add `--unified=3 --ignore-space-change` flags; expose `resolve_git_dir()` publicly |
| `src/prompt.rs` | Add `truncate_diff()`, change summary section to `build_prompt()` |
| `src/cli.rs` | Add staged files display; use `AgentName::display_name()` for signature |
| `src/types.rs` | Add `AgentName::display_name()` and `AgentName::install_url()` methods |
| `src/agents/mod.rs` | Add shared `AGENT_TIMEOUT`, `check_command_exists()`, `run_command_with_stdin()` |
| `src/agents/claude.rs` | Reduce to ~15 lines using shared utilities |
| `src/agents/codex.rs` | Reduce to ~15 lines using shared utilities |
| `src/agents/gemini.rs` | Reduce to ~15 lines using shared utilities |
| `src/hooks/managers.rs` | Use `git::resolve_git_dir()`; check for existing hook before adding |

### Design Details

#### 1. Diff Truncation (prompt.rs)

```
Constant: MAX_DIFF_LENGTH = 8000

truncate_diff(diff: &str) -> String
  - If diff.len() <= MAX_DIFF_LENGTH: return unchanged
  - Else: return diff[..MAX_DIFF_LENGTH] + "\n... (diff truncated)"

Called from: build_prompt() before adding diff to prompt
```

#### 2. Git Diff Flags (git.rs)

Update `get_staged_diff()` in `RealGitProvider`:

```rust
// Current:
let diff = self.run_git(&["diff", "--cached"])?;

// Updated:
let diff = self.run_git(&["diff", "--cached", "--unified=3", "--ignore-space-change"])?;
```

**Why these flags:**
- `--unified=3`: Limits context to 3 lines around changes (default is 3, but explicit is clearer; TypeScript uses this)
- `--ignore-space-change`: Ignores whitespace-only changes, producing cleaner diffs

#### 3. Change Summary (prompt.rs)

Add section to prompt after instructions, before diff sections:

```
=== CHANGE SUMMARY ===
Files changed: {count from name_status}
Lines added: {parsed from stat}
Lines removed: {parsed from stat}
```

Parsing approach:
- File count: count lines in `name_status`
- Lines: parse from stat output (e.g., "10 insertions(+), 5 deletions(-)")

#### 4. Staged Files Display (cli.rs)

Before spinner in `run_generate()`:
```
if !args.quiet && !args.message_only {
    display_staged_files(&diff.name_status);
}
```

Output format:
```
Staged changes:
  M  src/prompt.rs
  A  src/new_file.rs
```

#### 5. Shared Agent Infrastructure (agents/mod.rs)

Extract shared utilities to eliminate ~120 lines of duplication:

```rust
use std::time::Duration;

/// Shared timeout for all agents (120 seconds)
pub(crate) const AGENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Check if a CLI command exists in PATH
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

/// Run a command with prompt via stdin (used by Claude, Codex)
pub(crate) async fn run_command_with_stdin(
    command: &str,
    args: &[&str],
    prompt: &str,
    agent: AgentName,
) -> Result<String, AgentError> {
    // Spawn process, write to stdin, wait for output
    // Handles all error mapping to AgentError
}
```

**After refactor**, each agent becomes ~15 lines:

```rust
// src/agents/claude.rs
pub struct ClaudeAgent;

impl ClaudeAgent {
    pub async fn execute(&self, prompt: &str) -> Result<String, AgentError> {
        check_command_exists("claude", AgentName::Claude).await?;

        tokio::time::timeout(
            AGENT_TIMEOUT,
            run_command_with_stdin("claude", &["--print"], prompt, AgentName::Claude)
        )
        .await
        .map_err(|_| AgentError::Timeout {
            agent: AgentName::Claude,
            timeout_secs: AGENT_TIMEOUT.as_secs(),
        })?
    }
}
```

#### 6. AgentName Methods (types.rs)

Add display name and install URL methods to centralize agent metadata:

```rust
impl AgentName {
    /// Human-readable display name for signatures
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini",
        }
    }

    /// Installation URL for error hints
    pub fn install_url(&self) -> &'static str {
        match self {
            Self::Claude => "https://docs.anthropic.com/en/docs/claude-cli",
            Self::Codex => "https://github.com/phughk/codex",
            Self::Gemini => "https://github.com/google/generative-ai-cli",
        }
    }
}
```

**Usage in cli.rs:**
```rust
// Before (scattered match):
let signature = format!("🤖 Generated with {} via commitment",
    match agent_name {
        AgentName::Claude => "Claude",
        // ...
    });

// After (centralized):
let signature = format!("🤖 Generated with {} via commitment", agent_name.display_name());
```

#### 7. Shared resolve_git_dir (git.rs)

Make `resolve_git_dir()` public and remove duplicate from hooks/managers.rs:

```rust
// src/git.rs - make public
pub fn resolve_git_dir(cwd: &Path) -> Result<PathBuf, GitError> { ... }

// src/hooks/managers.rs - use shared function
use crate::git::resolve_git_dir;

pub fn install_plain_git(cwd: &Path, agent: &AgentName) -> Result<(), HookError> {
    let git_dir = resolve_git_dir(cwd)
        .map_err(|_| HookError::GitDirResolutionFailed)?;
    // ...
}
```

#### 8. Lefthook Append Logic (hooks/managers.rs)

In `install_lefthook()`:
```
if existing_config.contains("prepare-commit-msg:") {
    warn user
    return Ok(()) // skip without error
}
```

#### 9. Rust Idioms Polish (types.rs)

Add idiomatic Rust trait implementations to `ConventionalCommit`:

```rust
impl ConventionalCommit {
    /// Validate and construct a conventional commit message
    #[must_use = "validation result should be checked"]
    pub fn validate(msg: &str) -> Result<Self, CommitValidationError> {
        // ... existing implementation
    }
}

/// Enables `&commit` to be used where `&str` is expected
impl AsRef<str> for ConventionalCommit {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

/// Enables transparent string access: `*commit` yields `&str`
impl std::ops::Deref for ConventionalCommit {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}
```

**Why these traits:**

| Trait | Purpose | Usage Example |
|-------|---------|---------------|
| `#[must_use]` | Compiler warns if `validate()` result is ignored | Prevents silent validation failures |
| `AsRef<str>` | Generic string compatibility | `fn process(s: impl AsRef<str>)` accepts `&commit` |
| `Deref<Target=str>` | Transparent access to inner string | `commit.len()`, `commit.contains("feat")` work directly |

**Educational note:** `Deref` is slightly controversial - some prefer explicit `.as_str()`. We implement both: `Deref` for ergonomics, `as_str()` for explicitness. The private `raw` field still enforces validation-on-construction.

#### 10. Default for Unit Structs (agents/*.rs)

Add `Default` derive to agent unit structs:

```rust
#[derive(Default)]
pub struct ClaudeAgent;

#[derive(Default)]
pub struct CodexAgent;

#[derive(Default)]
pub struct GeminiAgent;
```

**Why:** Enables `ClaudeAgent::default()` and is idiomatic for unit structs. Zero runtime cost.

### Dependencies

**No new packages required** - uses existing:
- `regex` + `once_cell` for stat parsing
- `console` for staged files display colors

## Acceptance Criteria

**Constitution compliance:**
- [ ] No complex diff analysis (only file count, line counts)
- [ ] Functions over structs pattern maintained
- [ ] Tests co-located with source per @docs/constitutions/current/testing.md

**Feature-specific:**
- [ ] Diffs over 8000 chars are truncated with indicator
- [ ] Git diff uses `--unified=3 --ignore-space-change` flags
- [ ] Staged files displayed before spinner (when not quiet)
- [ ] Prompt includes change summary section (file count, lines +/-)
- [ ] Lefthook warns on existing hook instead of overwriting

**Code quality (DRY compliance):**
- [ ] `AGENT_TIMEOUT` defined once in `agents/mod.rs`
- [ ] `check_command_exists()` shared function in `agents/mod.rs`
- [ ] `run_command_with_stdin()` shared function in `agents/mod.rs`
- [ ] Each agent file reduced to ~15-20 lines
- [ ] `resolve_git_dir()` exposed from `git.rs`, removed from `hooks/managers.rs`
- [ ] `AgentName::display_name()` and `install_url()` methods added
- [ ] cli.rs uses `AgentName` methods instead of inline matches

**Rust idioms (NFR6):**
- [ ] `ConventionalCommit::validate()` has `#[must_use]` attribute
- [ ] `ConventionalCommit` implements `AsRef<str>`
- [ ] `ConventionalCommit` implements `Deref<Target=str>`
- [ ] Agent structs derive `Default`
- [ ] All public functions have doc comments

**Verification:**
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] Manual test: large diff gets truncated message
- [ ] Manual test: staged files display correctly
- [ ] Manual test: whitespace-only changes filtered from diff
- [ ] Code review: no duplicate logic across agent files
- [ ] Code review: `#[must_use]` warning fires if validation result ignored

## Open Questions

None - design validated during brainstorming.

## References

- Architecture: @docs/constitutions/current/architecture.md
- Patterns: @docs/constitutions/current/patterns.md
- Testing: @docs/constitutions/current/testing.md
- Tech Stack: @docs/constitutions/current/tech-stack.md
