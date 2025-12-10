# commitment-rs

> AI-powered commit message generator using your local CLI tools - no API keys required

<img width="500" height="378" alt="commitment-rs" src="https://github.com/user-attachments/assets/92761655-310f-44fb-85ef-aecf67910dd3" />

[![license: ISC](https://img.shields.io/badge/license-ISC-blue.svg)](./LICENSE)

A Rust port of [commitment](https://github.com/arittr/commitment) - generates conventional commit messages from git diffs using Claude, Codex, or Gemini CLI tools.

We all know we should write better commit messages. But we don't.

**commitment-rs** uses your **local AI CLI** (Claude Code, Codex, or Gemini) to analyze git diffs and generate professional, conventional commit messages automatically. **No API keys or additional services required** - it works with the AI tools you already have installed.

## Features

- **No API keys required** - Uses Claude Code, Codex, or Gemini CLI tools you already have installed
- **Context-aware** - Agentic coding CLIs understand your codebase context beyond just the diff
- **Conventional Commits** - Every commit follows [Conventional Commits](https://www.conventionalcommits.org/) format
- **Frictionless setup** - One command (`commitment init`) and stop committing `wip2` and `formatting`
- **Hook integration** - Works with lefthook, husky, simple-git-hooks, or plain git hooks
- **Fast startup** - Native Rust binary, instant startup
- **Smart diff handling** - Automatic truncation, change summaries, optimized for AI analysis

## Quick Start

```bash
# 1. Install (cargo install coming soon)
cargo install --path .

# 2. Set up git hooks (automatic)
commitment init

# 3. Make changes and commit
git add .
git commit  # Message generated automatically!
```

That's it! Every commit now gets an AI-generated, pretty good commit message.

## Installation

### From Source

```bash
git clone https://github.com/arittr/commitment-rs
cd commitment-rs
cargo build --release

# Binary is at target/release/commitment-rs
```

### Cargo Install (Coming Soon)

```bash
cargo install commitment-rs
```

## Requirements

- **Git repository**
- **AI CLI** (one of):
  - [Claude CLI](https://docs.anthropic.com/en/docs/claude-cli) (recommended) - Install with `npm install -g @anthropic-ai/claude-code`
  - [Codex CLI](https://developers.openai.com/codex/cli) - Install with `npm install -g @openai/codex`
  - [Gemini CLI](https://geminicli.com/docs/) - Install with `npm install -g @google/gemini-cli`

> [!IMPORTANT]
> commitment-rs uses your **local AI CLI tools** (not the OpenAI API or other cloud services). You need one of the CLIs above installed and configured.

## Usage

### Automatic (Recommended)

After running `commitment init`, commit messages are generated automatically:

```bash
git add src/components/Button.tsx
git commit  # Opens editor with AI-generated message
```

### Manual

Generate a message and commit in one step:

```bash
git add .
commitment
```

Generate message only (preview without committing):

```bash
commitment --dry-run
```

Use a specific AI agent:

```bash
commitment --agent codex
# or
commitment --agent gemini
```

## How It Works

1. **Analyze**: Reads your staged changes with `git diff --cached`
2. **Optimize**: Truncates large diffs (8000 char limit), adds change summary
3. **Generate**: Sends diff to AI CLI with a detailed prompt
4. **Validate**: Ensures response follows Conventional Commits format
5. **Commit**: Creates commit with generated message

## Example

```bash
git add src/api/ src/types/
commitment
```

**Generated:**

```text
refactor(agents): extract shared infrastructure to parent module

- Add check_command_exists() and run_command_with_stdin() utilities
- Define AGENT_TIMEOUT constant (120s) shared across all agents
- Reduce individual agent files to ~20 lines each
- Add display_name() and install_url() methods to AgentName

🤖 Generated with Claude via commitment-rs
```

## Configuration

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--agent <name>` | AI agent to use (`claude`, `codex`, or `gemini`) | `claude` |
| `--dry-run` | Generate message without creating commit | `false` |
| `--message-only` | Output only the commit message | `false` |
| `--quiet` | Suppress progress messages | `false` |
| `--cwd <path>` | Working directory | current directory |

**Examples:**

```bash
# Use Gemini agent
commitment --agent gemini

# Preview message without committing
commitment --dry-run

# Suppress progress messages (for scripts)
commitment --quiet
```

### Hook Setup

commitment-rs supports multiple hook managers:

| Manager | Command | Best For |
|---------|---------|----------|
| **Auto-detect** | `commitment init` | Most projects |
| **Lefthook** | `commitment init --hook-manager lefthook` | Fast, parallel execution, YAML config (recommended) |
| **Husky** | `commitment init --hook-manager husky` | Teams with existing husky setup |
| **simple-git-hooks** | `commitment init --hook-manager simple-git-hooks` | Lightweight alternative |
| **Plain Git Hooks** | `commitment init --hook-manager plain` | No dependencies |

**Configure default agent:**

```bash
commitment init --agent gemini  # Use Gemini by default
commitment init --agent codex   # Use Codex by default
```

### Lefthook Safety

When using lefthook, commitment-rs detects existing hook configurations before installation to prevent accidentally overwriting custom hooks or removing AI signature requirements.

## Troubleshooting

### Hooks Not Running

**Check installation:**

```bash
# For lefthook
cat lefthook.yml

# For husky
ls -la .husky/prepare-commit-msg

# For plain git hooks
ls -la .git/hooks/prepare-commit-msg
```

**Reinstall:**

```bash
commitment init
```

**Check permissions (Unix-like systems):**

```bash
# For lefthook, run:
lefthook install

# For husky
chmod +x .husky/prepare-commit-msg

# For plain git hooks
chmod +x .git/hooks/prepare-commit-msg
```

### Hooks Override My Custom Messages

This should **not** happen. Hooks check if you've specified a message:

```bash
git commit -m "my message"  # Uses your message ✅
git commit                  # Generates message ✅
```

If hooks override your messages, please [file an issue](https://github.com/arittr/commitment-rs/issues).

## Cross-Platform Support

| Platform | CLI Usage | Hooks | AI Agents |
|----------|-----------|-------|-----------|
| macOS    | ✅ | ✅ | ✅ Claude, Codex, Gemini |
| Linux    | ✅ | ✅ | ✅ Claude, Codex, Gemini |
| Windows  | ✅ | ⚠️ Git Bash/WSL | ✅ Claude, Codex, Gemini |

> **Note**: Windows users should use Git Bash or WSL for best hook compatibility.

## Contributing

Contributions welcome!

### Development

**Requirements:**

- Rust (stable)

**Commands:**

```bash
# Build
cargo build

# Run tests
cargo test

# Run linting
cargo clippy -- -D warnings

# Format code
cargo fmt

# Run integration tests
cargo test --test integration_tests
```

**Architecture:**

This project follows a strict layered architecture. See [CLAUDE.md](./CLAUDE.md) for detailed development guidelines and [docs/constitutions/current/](./docs/constitutions/current/) for:

- Architecture guidelines
- Patterns (enum dispatch, newtypes, shared infrastructure)
- Testing patterns
- Code style requirements

## License

ISC

## Acknowledgments

- Follows [Conventional Commits](https://www.conventionalcommits.org/) specification
- Port of [commitment](https://github.com/arittr/commitment)
- Built with Rust, Tokio, and clap
- Developed using [Claude Code](https://claude.com/claude-code)
- Inspired by years of bad commit messages
