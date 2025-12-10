# Architecture

## Layered Design

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
│   trait     │  │ • Shared infrastructure │
│ • StagedDiff│  │ • clean_ai_response()   │
└─────────────┘  └─────────────────────────┘
```

## Dependency Rule

**Mandatory:** Dependencies flow DOWNWARD only.

- ✅ CLI → Core (allowed)
- ✅ Core → Git, Agents (allowed)
- ❌ Core → CLI (FORBIDDEN)
- ❌ Git → Core (FORBIDDEN)
- ❌ Agents → Core (FORBIDDEN)

**Violation breaks architecture:** Lower layers must not know about upper layers.

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
- Shared infrastructure (command checking, stdin handling)

**Forbidden:**
- Git operations
- Prompt construction
- Business logic

### Git (`git.rs`)

**Allowed:**
- Run git commands
- Parse git output into structs
- Abstract via `GitProvider` trait
- Share `resolve_git_dir()` for hook detection

**Forbidden:**
- AI operations
- Prompt construction

### Hooks (`hooks/`)

**Allowed:**
- Detect hook managers
- Install/configure hooks
- File system operations for hook setup
- Detect existing hooks before overwriting

**Forbidden:**
- Git operations beyond hook installation
- AI operations

## File Organization

### Module Structure

```
src/
├── main.rs          # Entry point, #[tokio::main]
├── lib.rs           # Public API: generate_commit_message()
├── cli.rs           # clap args + command handlers
├── agents/
│   ├── mod.rs       # Agent enum, shared infra, clean_ai_response()
│   ├── claude.rs    # ClaudeAgent (~20 lines)
│   ├── codex.rs     # CodexAgent (~20 lines)
│   └── gemini.rs    # GeminiAgent (~20 lines)
├── git.rs           # GitProvider trait, RealGitProvider, StagedDiff
├── prompt.rs        # build_prompt(), truncate_diff(), parse_change_summary()
├── types.rs         # AgentName enum, ConventionalCommit newtype
├── error.rs         # AgentError, GitError, GeneratorError
└── hooks/
    ├── mod.rs       # HookManager enum + detect/install
    └── managers.rs  # Per-manager install logic
tests/
└── integration_tests.rs  # Cross-module integration tests
```

### Naming

- `snake_case.rs` for files
- `snake_case` for modules
- `PascalCase` for types
- `SCREAMING_SNAKE_CASE` for constants

### Visibility

- `pub` only for public API
- `pub(crate)` for crate-internal items
- Private by default
