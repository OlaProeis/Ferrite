# Ferrite Build Makefile
# Simple build automation for local development

.PHONY: build release run clean check fmt clippy test all

# Default target
all: check build

# Debug build (fast compilation)
build:
	cargo build

# Release build (optimized)
release:
	cargo build --release

# Run debug build
run:
	cargo run

# Run release build
run-release:
	cargo run --release

# Check for errors without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Run clippy lints
clippy:
	cargo clippy --all-targets -- -D warnings

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Full lint check (format + clippy)
lint: fmt clippy

# Pre-commit check (format, clippy, test, build)
precommit: fmt clippy test build
	@echo "All checks passed!"

# Show binary size after release build
size: release
ifeq ($(OS),Windows_NT)
	@dir /s target\release\ferrite.exe 2>nul || echo "Binary not found"
else
	@ls -lh target/release/ferrite 2>/dev/null || echo "Binary not found"
endif

# Help
help:
	@echo "Ferrite Build Targets:"
	@echo "  make build      - Debug build"
	@echo "  make release    - Release build (optimized)"
	@echo "  make run        - Run debug build"
	@echo "  make run-release- Run release build"
	@echo "  make check      - Check for errors"
	@echo "  make fmt        - Format code"
	@echo "  make clippy     - Run lints"
	@echo "  make test       - Run tests"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make lint       - Format + clippy"
	@echo "  make precommit  - Full pre-commit check"
	@echo "  make size       - Show release binary size"
