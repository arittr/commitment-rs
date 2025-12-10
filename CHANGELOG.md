# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - Parity Enhancements

### Added

#### Diff Processing Enhancements (FR1-FR4)
- **FR1: Diff Truncation** - Automatic truncation of diffs exceeding 8000 characters to prevent token limit issues
  - Respects UTF-8 character boundaries to avoid corruption
  - Appends clear truncation indicator message
  - Implemented in `src/prompt.rs::truncate_diff()`

- **FR2: Optimized Git Diff Flags** - Enhanced git diff output for AI analysis
  - `--unified=3`: Compact context (3 lines instead of default) for reduced token count
  - `--ignore-space-change`: Filters whitespace noise for cleaner, more semantic diffs
  - Configured in `src/git.rs::get_staged_diff()`

- **FR3: Staged Files Display** - Improved visibility of changed files
  - File statistics section (`--stat`) showing file count and line changes
  - File status section (`--name-status`) showing A/M/D status with paths
  - Structured prompt sections in `src/prompt.rs::build_prompt()`

- **FR4: Change Summary in Prompts** - Concise overview of changes for AI context
  - Extracts file count from name-status output
  - Parses lines added/removed from stat output using regex
  - Displays summary before detailed diff sections
  - Implemented in `src/prompt.rs::parse_change_summary()`

#### Hook Integration Safety (FR5)
- **FR5: Lefthook Safety Check** - Prevents signature bypasses in lefthook configurations
  - Validates `skip_output` is not set to `["meta"]` (unsafe pattern)
  - Warns users about signature bypass risks during installation
  - Guards against accidental signature stripping
  - Implemented in `src/hooks/managers.rs::validate_lefthook_config()`

### Code Quality Improvements

#### DRY Compliance (~260 lines removed)
- **Shared Agent Infrastructure** - Eliminated duplication across Claude, Codex, and Gemini agents
  - Extracted common command execution logic to `src/agents/mod.rs`
  - Shared utilities: `check_command_exists()`, `run_command()`, `format_timeout_error()`
  - Reduced agent implementation from ~100 lines to ~30 lines each
  - Single source of truth for timeout handling and error formatting

#### Rust Idioms
- **#[must_use] Annotations** - Added to `ConventionalCommit::validate()` to prevent ignoring validation results
- **AsRef<str> Trait** - Implemented on `ConventionalCommit` for ergonomic string conversion
- **Deref Trait** - Implemented on `ConventionalCommit` for transparent string access
- **Default Derive** - Added to agent structs (ClaudeAgent, CodexAgent, GeminiAgent) for consistent initialization
- **Parse, Don't Validate** - Enhanced newtype pattern for `ConventionalCommit` to make invalid states unrepresentable

### Testing

#### Integration Tests
- Comprehensive integration test suite in `tests/integration_tests.rs`
- Full generation flow testing with MockGitProvider
- Diff truncation verification (>8000 chars)
- Change summary prompt integration tests
- Staged files display formatting tests
- Git diff flags output verification
- Response cleaning and validation tests
- UTF-8 handling and boundary respect tests

## [0.1.0] - Initial Release

### Added
- Core commit message generation using local AI CLI tools (Claude, Codex, Gemini)
- Conventional commit format validation with newtype pattern
- Git provider abstraction with trait-based dependency injection
- Agent enum dispatch (no trait objects, no async_trait)
- Sync git operations, async agent execution
- Error handling with thiserror (domain errors) and anyhow (CLI errors)
- Terminal UX with console and indicatif
- Git worktree support with path resolution
- Hook manager detection and installation framework
- Basic lefthook, husky, simple-git-hooks, and plain git hooks support
