# Constitution Metadata

**Version:** 2
**Created:** 2025-12-10
**Previous:** v1 (2025-12-09)

## Summary

Second constitution version reflecting the parity-enhancements work that established shared agent infrastructure, improved prompts, and comprehensive testing.

## Core Principles

1. **Idiomatic Rust** - Follow Rust conventions, not TypeScript patterns
2. **Functions over structs** - Use structs only when state is meaningful
3. **Parse, don't validate** - Invalid states should be unrepresentable
4. **Let AI analyze** - No complex diff analysis in code; AI sees the diff
5. **Shared infrastructure** - Common utilities in parent modules, not duplicated

## Changelog

### v2 (2025-12-10)

**Shared Agent Infrastructure:**
- Added `check_command_exists()` and `run_command_with_stdin()` in `agents/mod.rs`
- Added `AGENT_TIMEOUT` constant (120s) shared across all agents
- Individual agent files now ~20 lines each

**Type Enhancements:**
- `AgentName::display_name()` for user-facing output
- `AgentName::install_url()` for error message help
- `ConventionalCommit` now implements `Deref<Target=str>` and `AsRef<str>`

**Prompt Improvements:**
- Diff truncation at 8000 characters to prevent token limit issues
- Change summary section (file count, lines added/removed)
- Better formatting with section headers

**Hook Safety:**
- Lefthook detects existing hooks before installation
- Warns user instead of silently overwriting

**Testing:**
- Integration tests in `tests/integration_tests.rs`
- Comprehensive coverage of all modules

### v1 (2025-12-09)

Initial constitution establishing:

- **Architecture:** Layered design (CLI → Core → Git/Agents)
- **Patterns:** Enum dispatch, newtypes, thiserror + anyhow
- **Tech stack:** Tokio, clap, console/indicatif
- **Testing:** Co-located tests, trait-based mocking
- **Anti-patterns:** No diff analysis, no trait objects for agents, no Generator struct

## Evolution

To change the constitution:

1. **Clarifications:** Edit in place (non-breaking)
2. **New patterns:** Create new version
3. **Breaking changes:** Create new version

When in doubt: Follow the constitution, or propose an amendment.
