# commitment-rs

> AI-powered commit message generator using your local CLI tools - no API keys required

<img width="500" height="378" alt="rust-bart" src="https://github.com/user-attachments/assets/92761655-310f-44fb-85ef-aecf67910dd3" />

A Rust port of [commitment](https://github.com/arittr/commitment) - generates conventional commit messages from git diffs using Claude, Codex, or Gemini CLI tools.

## Status

Currently under development. See CLAUDE.md for architecture and development guidelines.


## Features

- Generate conventional commit messages from staged changes
- Support for Claude, Codex, and Gemini AI agents
- Optimized git diff processing:
  - Automatic truncation of large diffs (8000 char limit) to prevent token overflow
  - Compact unified diffs (--unified=3) for efficient AI analysis
  - Whitespace-change filtering (--ignore-space-change) for cleaner diffs
  - Change summaries (file count, lines added/removed) in prompts
- Git hook integration (lefthook, husky, simple-git-hooks, plain git hooks)
  - Lefthook safety verification to prevent accidental signature bypasses
- Git worktree support
- Fast startup, single binary distribution

## Build

```bash
cargo build
cargo run
```

## Development

See CLAUDE.md for detailed development guidelines, architecture, and patterns.
