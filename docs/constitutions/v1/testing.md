# Testing

## Test Organization

### Co-located Unit Tests

**Mandatory:** Tests live with their source using `#[cfg(test)]` modules.

```rust
// src/types.rs
pub enum AgentName { ... }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_claude() {
        assert!("claude".parse::<AgentName>().is_ok());
    }

    #[test]
    fn rejects_invalid() {
        assert!("invalid".parse::<AgentName>().is_err());
    }
}
```

**Why:** Tests close to code they test. Discoverable. Run with `cargo test`.

### Integration Tests

**Location:** `tests/` directory for cross-module tests.

```
tests/
├── cli_integration.rs
└── generation_flow.rs
```

**When to use:** Tests spanning multiple modules or requiring full setup.

## Mocking Patterns

### Trait-based Mocking

**Mandatory:** Use traits for dependencies, implement mocks manually.

```rust
// Production trait
pub trait GitProvider {
    fn get_staged_diff(&self) -> Result<StagedDiff, GitError>;
    fn has_staged_changes(&self) -> Result<bool, GitError>;
    fn commit(&self, message: &str) -> Result<(), GitError>;
}

// Test mock
#[cfg(test)]
mod tests {
    use super::*;

    struct MockGitProvider {
        staged_diff: StagedDiff,
        has_changes: bool,
    }

    impl GitProvider for MockGitProvider {
        fn get_staged_diff(&self) -> Result<StagedDiff, GitError> {
            Ok(self.staged_diff.clone())
        }

        fn has_staged_changes(&self) -> Result<bool, GitError> {
            Ok(self.has_changes)
        }

        fn commit(&self, _message: &str) -> Result<(), GitError> {
            Ok(())
        }
    }
}
```

**Why:** No mock library dependency. Tests are explicit about behavior.

### Mock Builders

**For complex mocks, use builder pattern:**

```rust
#[cfg(test)]
impl MockGitProvider {
    fn new() -> Self {
        Self {
            staged_diff: StagedDiff::default(),
            has_changes: true,
        }
    }

    fn with_diff(mut self, diff: StagedDiff) -> Self {
        self.staged_diff = diff;
        self
    }

    fn with_no_changes(mut self) -> Self {
        self.has_changes = false;
        self
    }
}
```

## Async Testing

### `#[tokio::test]`

**Mandatory:** Use `#[tokio::test]` for async tests.

```rust
#[tokio::test]
async fn generates_commit_message() {
    let git = MockGitProvider::new().with_diff(sample_diff());
    let agent = Agent::Claude(ClaudeAgent);

    let result = generate_commit_message(&git, &agent, None).await;

    assert!(result.is_ok());
}
```

### Timeouts

**Use timeout for tests that might hang:**

```rust
#[tokio::test]
#[timeout(5000)]  // 5 seconds
async fn agent_responds_quickly() {
    // ...
}
```

## Test Patterns

### Success Cases

Test the happy path:

```rust
#[test]
fn validates_conventional_commit() {
    let result = ConventionalCommit::validate("feat: add feature");
    assert!(result.is_ok());
}
```

### Error Cases

Test error conditions explicitly:

```rust
#[test]
fn rejects_invalid_commit() {
    let result = ConventionalCommit::validate("not a commit message");
    assert!(matches!(
        result,
        Err(CommitValidationError::InvalidFormat(_))
    ));
}
```

### Edge Cases

Test boundary conditions:

```rust
#[test]
fn handles_empty_diff() {
    let diff = StagedDiff {
        stat: String::new(),
        name_status: String::new(),
        diff: String::new(),
    };
    let prompt = build_prompt(&diff);
    assert!(!prompt.is_empty()); // Still produces valid prompt
}
```

## What to Test

### Must Test

- Public API functions
- Type validation (newtypes, FromStr impls)
- Error conditions
- Edge cases

### Don't Test

- Private implementation details
- External CLI behavior (agent CLIs)
- Trivial getters/setters

## Test Naming

**Pattern:** `test_<what>_<condition>` or descriptive phrases

```rust
#[test]
fn parses_claude_agent_name() { ... }

#[test]
fn rejects_unknown_agent_name() { ... }

#[test]
fn validates_feat_commit() { ... }

#[test]
fn rejects_commit_without_type() { ... }
```

## Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test parses_claude

# Show output
cargo test -- --nocapture

# Single-threaded (if tests conflict)
cargo test -- --test-threads=1
```
