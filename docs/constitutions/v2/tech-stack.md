# Tech Stack

## Language

**Rust 2024 Edition**

- Use stable Rust features
- Edition 2024 for latest language improvements

## Runtime

### Tokio

**Purpose:** Async runtime for agent execution

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process"] }
```

**Features used:**
- `rt-multi-thread` - Multi-threaded runtime
- `macros` - `#[tokio::main]`, `#[tokio::test]`
- `process` - `tokio::process::Command` for async subprocess

**Why Tokio:** Industry standard, learning goal for this project.

## CLI

### clap

**Purpose:** Command-line argument parsing

```toml
clap = { version = "4", features = ["derive"] }
```

**Pattern:** Derive macros for declarative CLI definition

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "claude")]
    agent: AgentName,
}
```

**Why clap:** Most popular, derive pattern, excellent error messages.

## Error Handling

### thiserror

**Purpose:** Domain error types

```toml
thiserror = "2"
```

**Pattern:** Derive `Error` for structured errors

```rust
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent not found")]
    NotFound { agent: AgentName },
}
```

### anyhow

**Purpose:** CLI boundary error handling

```toml
anyhow = "1"
```

**Pattern:** `anyhow::Result` at CLI layer, `.context()` for messages

```rust
pub async fn run() -> anyhow::Result<()> {
    something().context("failed to do something")?;
}
```

## Terminal UX

### console

**Purpose:** Terminal colors and styling

```toml
console = "0.15"
```

**Pattern:** `style()` for colored output

```rust
eprintln!("{}: message", style("error").red());
```

### indicatif

**Purpose:** Progress indicators

```toml
indicatif = "0.17"
```

**Pattern:** Spinner while waiting for AI response

```rust
let spinner = ProgressBar::new_spinner();
spinner.set_message("Generating...");
```

## Regex

### regex + once_cell

**Purpose:** Response cleaning, commit validation, diff parsing

```toml
regex = "1"
once_cell = "1"
```

**Pattern:** Lazy static regex for performance

```rust
static PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(feat|fix|...)").unwrap()
});
```

## Serialization

### serde + serde_json + serde_yaml

**Purpose:** Hook manager config parsing

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
```

**Used for:**
- `package.json` (simple-git-hooks detection)
- `lefthook.yml` (lefthook configuration)

## Development Dependencies

```toml
[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "test-util"] }
```

**Pattern:** Use `#[tokio::test]` for async tests

## Not Used (Intentionally)

### async-trait

**Not used because:** We use enum dispatch for agents, not trait objects.
No need for `#[async_trait]` or boxing futures.

### serde for core types

**Not used because:** Core types like `AgentName` use `FromStr` for parsing.
Serde only used for external config files (package.json, lefthook.yml).

### mockall or similar

**Not used because:** We use manual trait implementations for mocking.
Keeps dependencies minimal, tests explicit.
