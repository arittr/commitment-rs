# https://just.systems
build:
    cargo build

release:
    cargo build --release

lint:
    cargo fmt --check
    cargo clippy -- -D warnings

test:
    cargo test

check: lint test
