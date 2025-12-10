# https://just.systems
build:
    cargo build

release:
    cargo build --release

# format and lint
lint:
    cargo fmt
    cargo clippy -- -D warnings

test:
    cargo test

integration-test:
    cargo test --test integration_tests

check: lint test
