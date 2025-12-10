# commitment-rs

> AI-powered commit message generator using your local CLI tools - no API keys required

<img width="500" height="304" alt="rust-bart" src="https://github.com/user-attachments/assets/78132637-ffb6-42e4-ab8d-4017be943722" />

A Rust port of [commitment](https://github.com/arittr/commitment) - generates conventional commit messages from git diffs using Claude, Codex, or Gemini CLI tools.

## Status

Currently under development. See CLAUDE.md for architecture and development guidelines.

## Features (Planned)

- Generate conventional commit messages from staged changes
- Support for Claude, Codex, and Gemini AI agents
- Git hook integration (lefthook, husky, simple-git-hooks, plain git hooks)
- Git worktree support
- Fast startup, single binary distribution

## Build

```bash
cargo build
cargo run
```

## Development

See CLAUDE.md for detailed development guidelines, architecture, and patterns.
