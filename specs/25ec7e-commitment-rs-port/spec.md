---
runId: 25ec7e
feature: commitment-rs-port
created: 2025-12-09
status: draft
source: https://github.com/arittr/commitment
---

# commitment-rs: Rust Port Specification

## Overview

Port of [commitment](https://github.com/arittr/commitment) from TypeScript to Rust. An AI-powered commit message generator that uses local AI CLI tools (Claude, Codex, Gemini) to generate conventional commit messages from git diffs.

## Goals (Priority Order)

1. **Learning** - Embrace idiomatic Rust patterns
2. **Distribution** - Single static binary, no runtime dependencies
3. **Performance** - Faster startup (bonus, not driver)

## Scope

### In Scope

- Full port of core functionality
- Claude agent (primary)
- Codex and Gemini agents (secondary)
- Hook manager integration (lefthook, husky, simple-git-hooks, plain git hooks)
- Git worktree support for hook installation

### Out of Scope

- Eval framework from TypeScript version
- Programmatic API beyond CLI
- Complex diff analysis (AI handles this)

## Functional Requirements

### FR-1: Commit Message Generation

**Given** staged git changes exist
**When** user runs `commitment`
**Then** generate a conventional commit message using the configured AI agent

Acceptance Criteria:
- Detects staged changes via `git diff --cached`
- Builds prompt with file stats, name/status, and diff content
- Executes AI CLI (claude/codex/gemini) with prompt
- Cleans AI response (removes markdown, preambles, thinking tags)
- Validates output against conventional commit format
- Commits with generated message (unless `--dry-run`)

### FR-2: Agent Selection

**Given** multiple AI agents are supported
**When** user specifies `--agent <name>`
**Then** use the specified agent for generation

Supported agents:
| Agent | CLI Command | Input Method |
|-------|-------------|--------------|
| Claude | `claude --print` | stdin |
| Codex | `codex exec --skip-git-repo-check` | stdin |
| Gemini | `gemini -p "<prompt>"` | argument |

### FR-3: Dry Run Mode

**Given** user wants to preview without committing
**When** user runs `commitment --dry-run`
**Then** output the generated message without creating a commit

### FR-4: Message-Only Mode

**Given** user wants raw message output (for piping)
**When** user runs `commitment --message-only`
**Then** output only the commit message, no formatting or status

### FR-5: Hook Installation

**Given** user wants automatic commit message generation
**When** user runs `commitment init`
**Then** install prepare-commit-msg hook using detected/specified hook manager

Detection order:
1. Lefthook (lefthook.yml, .lefthook.yml, etc.)
2. Husky (.husky directory)
3. simple-git-hooks (package.json)
4. Plain git hooks (fallback)

### FR-6: Git Worktree Support

**Given** project uses git worktrees
**When** installing plain git hooks
**Then** correctly resolve hook directory from `.git` file's `gitdir:` reference

## Non-Functional Requirements

### NFR-1: Response Time

- Git operations: < 100ms (local, sync)
- AI generation: < 120s timeout (external process)
- Startup time: < 50ms to first output

### NFR-2: Error Messages

- Domain errors describe WHAT happened (thiserror)
- CLI layer adds HOW to fix (anyhow context)
- Agent not found: provide installation URL

### NFR-3: Binary Size

- Release binary < 5MB (with LTO)
- No runtime dependencies

### NFR-4: Terminal UX

- Spinner during AI generation (indicatif)
- Colored output for errors/success (console)
- Quiet mode suppresses progress

## Architecture

See @docs/constitutions/current/architecture.md for layer design.

### Module Mapping

```
src/
├── main.rs          # Entry: #[tokio::main], parse CLI, call run()
├── lib.rs           # Public API: generate_commit_message()
├── cli.rs           # clap derive structs, command handlers
├── agents/
│   ├── mod.rs       # Agent enum, generate(), clean_ai_response()
│   └── claude.rs    # ClaudeAgent (codex.rs, gemini.rs later)
├── git.rs           # GitProvider trait, RealGitProvider, StagedDiff
├── prompt.rs        # build_prompt() - template only
├── types.rs         # AgentName enum, ConventionalCommit newtype
├── error.rs         # AgentError, GitError, GeneratorError
└── hooks/
    ├── mod.rs       # HookManager enum, detect(), install()
    └── managers.rs  # Per-manager installation logic
```

## Type Design

See @docs/constitutions/current/patterns.md for pattern rationale.

### Core Types

| Type | Pattern | Purpose |
|------|---------|---------|
| `AgentName` | Enum | Parse from CLI, closed set |
| `Agent` | Enum dispatch | No trait objects, exhaustive matching |
| `ConventionalCommit` | Newtype | Validation on construction |
| `StagedDiff` | Plain struct | Data carrier, no validation needed |
| `HookManager` | Enum | Closed set of supported managers |

### Error Types

| Error | Crate | Usage |
|-------|-------|-------|
| `AgentError` | thiserror | Agent execution failures |
| `GitError` | thiserror | Git command failures |
| `GeneratorError` | thiserror | Orchestration failures |
| `anyhow::Error` | anyhow | CLI boundary only |

## Response Cleaning Pipeline

Order matters - each step depends on previous:

1. **Extract markers** - `<<<COMMIT_MESSAGE_START>>>` to `<<<COMMIT_MESSAGE_END>>>`
2. **Remove code blocks** - Strip ` ```...``` ` wrappers
3. **Remove preambles** - "Here is the commit message:", etc.
4. **Remove thinking** - `<thinking>...</thinking>` tags
5. **Collapse newlines** - 3+ newlines → 2
6. **Trim** - Remove leading/trailing whitespace

## Testing Strategy

See @docs/constitutions/current/testing.md for patterns.

### Unit Tests (Co-located)

- `types.rs`: AgentName parsing, ConventionalCommit validation
- `prompt.rs`: build_prompt() output format
- `agents/mod.rs`: clean_ai_response() pipeline
- `git.rs`: StagedDiff parsing (with MockGitProvider)

### Integration Tests (`tests/`)

- `cli_integration.rs`: End-to-end CLI behavior
- `generation_flow.rs`: Full generate flow with mocks

### Mocking

- `GitProvider` trait enables `MockGitProvider` for tests
- No external mocking libraries (manual trait impl)

## Dependencies

See @docs/constitutions/current/tech-stack.md for versions and rationale.

### Runtime

- tokio (async process execution)
- clap (CLI parsing)
- thiserror + anyhow (error handling)
- console + indicatif (terminal UX)
- regex + once_cell (response cleaning)
- serde + serde_json + serde_yaml (hook config)

### Not Used (Intentional)

- async-trait (enum dispatch instead)
- mockall (manual mocks instead)
- serde for core types (FromStr instead)

## CLI Interface

```
commitment [OPTIONS] [COMMAND]

Commands:
  init    Initialize git hooks

Options:
  --agent <NAME>     AI agent [default: claude] [values: claude, codex, gemini]
  --dry-run          Generate without committing
  --message-only     Output raw message only
  --quiet, -q        Suppress progress output
  --verbose, -v      Show debug output
  --cwd <PATH>       Working directory [default: .]

Init Options:
  --hook-manager <MANAGER>  Override auto-detection
  --agent <NAME>            Default agent for hooks [default: claude]
```

## Success Criteria

1. **Functional parity** with TypeScript version (minus eval)
2. **All tests pass** with `cargo test`
3. **No clippy warnings** with `cargo clippy -- -D warnings`
4. **Self-dogfooding** - use commitment-rs for its own commits
5. **Single binary** distribution via `cargo build --release`

## Implementation Order

Reference only - detailed tasks in `/spectacular:plan`:

1. Scaffolding (Cargo.toml, module files)
2. Types (enums, newtypes, errors)
3. Git module (trait, provider, diff)
4. Prompt (simple template)
5. Agent (enum, execute, clean)
6. Core (generate_commit_message)
7. CLI (clap, handlers)
8. Polish (spinner, colors)
9. Hooks (detect, install)
10. Extend (Codex, Gemini agents)
