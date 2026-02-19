.PHONY: build run clean release install help all

# Default target - show help
all help:
	@echo "Pomodoro Timer - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  make         - Show this help message"
	@echo "  make build   - Build the project (debug)"
	@echo "  make run     - Build and run the project (debug)"
	@echo "  make release - Build the project (release)"
	@echo "  make run-release - Build and run the project (release)"
	@echo "  make clean   - Clean build artifacts"
	@echo "  make deps    - Install dependencies"
	@echo "  make test    - Run tests"
	@echo "  make lint    - Run clippy linter"
	@echo "  make fmt     - Format code"
	@echo "  make check-fmt - Check code formatting"

# Build the project (debug)
build:
	cargo build

# Run the app
run: build
	cargo run

# Build for release
release:
	cargo build --release

# Run release build
run-release: release
	cargo run --release

# Clean build artifacts
clean:
	cargo clean

# Install cargo dependencies
deps:
	cargo fetch

# Run tests
test:
	cargo test

# Run clippy
lint:
	cargo clippy -- -D warnings

# Format code
fmt:
	cargo fmt

# Check formatting
check-fmt:
	cargo fmt --check