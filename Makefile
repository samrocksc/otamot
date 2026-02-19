.PHONY: build run clean release install help

# Default target
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

# Help
help:
	@echo "Pomodoro Timer - Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the project (debug)"
	@echo "  run          - Build and run the project (debug)"
	@echo "  release      - Build the project (release)"
	@echo "  run-release  - Build and run the project (release)"
	@echo "  clean        - Clean build artifacts"
	@echo "  deps         - Install dependencies"
	@echo "  test         - Run tests"
	@echo "  lint         - Run clippy linter"
	@echo "  fmt          - Format code"
	@echo "  check-fmt    - Check code formatting"
	@echo "  help         - Show this help message"