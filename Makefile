.PHONY: help build test fmt lint clean docs

help:
	@echo "Nomnom - Data Transformation Framework"
	@echo ""
	@echo "Available targets:"
	@echo "  build      - Build the library and CLI tool"
	@echo "  test       - Run all tests"
	@echo "  fmt        - Format Rust code"
	@echo "  lint       - Run clippy linter"
	@echo "  clean      - Remove build artifacts"
	@echo "  docs       - Generate documentation"

build:
	@echo "Building nomnom..."
	cargo build --release

test:
	@echo "Running tests..."
	cargo test

fmt:
	@echo "Formatting code..."
	cargo fmt

lint:
	@echo "Running clippy..."
	cargo clippy -- -D warnings

clean:
	@echo "Cleaning build artifacts..."
	cargo clean

docs:
	@echo "Generating documentation..."
	cargo doc --no-deps --open
