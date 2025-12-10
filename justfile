# https://just.systems

# list available commands
default:
    @just --list

# build debug binary
build:
    cargo build

# build release binary
release:
    cargo build --release

# install locally
install:
    cargo install --path .

# format code
fmt:
    cargo fmt

# check formatting without modifying
fmt-check:
    cargo fmt -- --check

# run clippy lints
clippy:
    cargo clippy -- -D warnings

# format and lint
lint: fmt clippy

# run all unit and integration tests
test:
    cargo test

# run only unit tests (no integration)
test-unit:
    cargo test --lib

# run integration tests
test-integration:
    cargo test --test integration_tests

# run real agent tests (requires CLI auth)
test-claude:
    cargo test --test real_agent_tests claude -- --ignored --nocapture

test-codex:
    cargo test --test real_agent_tests codex -- --ignored --nocapture

test-gemini:
    cargo test --test real_agent_tests gemini -- --ignored --nocapture

test-agents:
    cargo test --test real_agent_tests -- --ignored --nocapture

# full CI check: format, lint, test
check: lint test

# quick check without formatting
ci: fmt-check clippy test

# watch for changes and run tests
watch:
    cargo watch -x test

# clean build artifacts
clean:
    cargo clean

# show test coverage (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# run with dry-run flag
run-dry:
    cargo run -- --dry-run

# run with verbose output
run-verbose:
    cargo run -- --dry-run --verbose

# show help
run-help:
    cargo run -- --help
