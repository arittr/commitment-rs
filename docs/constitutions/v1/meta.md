# Constitution Metadata

**Version:** 1
**Created:** 2025-12-09
**Previous:** None (initial version)

## Summary

Initial constitution for commitment-rs, the Rust port of commitment.

## Core Principles

1. **Idiomatic Rust** - Follow Rust conventions, not TypeScript patterns
2. **Functions over structs** - Use structs only when state is meaningful
3. **Parse, don't validate** - Invalid states should be unrepresentable
4. **Let AI analyze** - No complex diff analysis in code; AI sees the diff

## Changelog

### v1 (2025-12-09)

Initial constitution establishing:

- **Architecture:** Layered design (CLI → Core → Git/Agents)
- **Patterns:** Enum dispatch, newtypes, thiserror + anyhow
- **Tech stack:** Tokio, clap, console/indicatif
- **Testing:** Co-located tests, trait-based mocking
- **Anti-patterns:** No diff analysis, no trait objects for agents, no Generator struct

**Rationale:** Port from TypeScript while embracing idiomatic Rust. Key simplifications from TS version:
- Enum dispatch instead of trait objects (known agent set)
- Functions instead of Generator class (minimal state)
- No analyzeCodeChanges() (AI analyzes diffs better)

## Evolution

To change the constitution:

1. **Clarifications:** Edit in place (non-breaking)
2. **New patterns:** Create new version
3. **Breaking changes:** Create new version

When in doubt: Follow the constitution, or propose an amendment.
