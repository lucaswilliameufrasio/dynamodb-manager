FLUTTER ?= fvm flutter
DART    ?= fvm dart
CARGO   ?= cargo

.PHONY: help setup get pub-get format format-check fmt analyze test test-ci coverage \
		lint validate check sync coverage-tools build build-rust build-macos build-linux build-windows \
		run run-macos run-linux run-windows run-release devices doctor info upgrade clean bootstrap aws-check

help:
	@echo "DynamoDB Manager commands:"
	@echo "  make setup         - Install Flutter and Rust dependencies"
	@echo "  make bootstrap     - Install pinned tools and project dependencies"
	@echo "  make get           - Alias for pub-get"
	@echo "  make pub-get       - Install Flutter dependencies"
	@echo "  make format        - Format Dart and Rust code"
	@echo "  make format-check  - Check Dart and Rust formatting without changes"
	@echo "  make analyze       - Run Flutter static analysis"
	@echo "  make test          - Run Flutter and Rust tests"
	@echo "  make test-ci       - Run all tests and collect coverage"
	@echo "  make coverage      - Run Flutter tests and Rust coverage"
	@echo "  make coverage-tools - Install cargo-llvm-cov if needed"
	@echo "  make lint          - Run Flutter analysis and Rust Clippy"
	@echo "  make validate      - Run format-check, lint, tests, and Rust build"
	@echo "  make sync          - Regenerate flutter_rust_bridge bindings"
	@echo "  make build         - Alias for build-macos"
	@echo "  make build-macos   - Build the macOS desktop app"
	@echo "  make build-linux   - Build the Linux desktop app"
	@echo "  make build-windows - Build the Windows desktop app"
	@echo "  make build-rust    - Compile the Rust library"
	@echo "  make run           - Alias for run-macos"
	@echo "  make run-macos     - Run on macOS desktop"
	@echo "  make run-linux     - Run on Linux desktop"
	@echo "  make run-windows   - Run on Windows desktop"
	@echo "  make run-release   - Run the macOS app in release mode"
	@echo "  make devices       - List available Flutter devices"
	@echo "  make doctor        - Run Flutter and Rust toolchain diagnostics"
	@echo "  make info          - Show project and toolchain versions"
	@echo "  make aws-check     - Check AWS CLI and configured profiles"
	@echo "  make upgrade       - Upgrade Flutter and Rust dependencies"
	@echo "  make clean         - Clean Flutter and Rust build artifacts"
	@echo "  make help          - Show this help"

setup:
	$(FLUTTER) pub get
	cd rust_builder && cargo fetch

get: pub-get

pub-get:
	$(FLUTTER) pub get

format: fmt

fmt:
	$(DART) format lib test
	$(CARGO) fmt --manifest-path rust/Cargo.toml

format-check:
	$(DART) format --output=none --set-exit-if-changed lib test
	$(CARGO) fmt --manifest-path rust/Cargo.toml -- --check

analyze:
	$(FLUTTER) analyze

test:
	$(FLUTTER) test
	$(CARGO) test --manifest-path rust/Cargo.toml

test-ci: coverage

coverage-tools:
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "Installing cargo-llvm-cov..."; \
		$(CARGO) install cargo-llvm-cov; \
	else \
		echo "cargo-llvm-cov is already installed."; \
	fi

coverage: coverage-tools
	$(FLUTTER) test --coverage
	$(CARGO) llvm-cov --manifest-path rust/Cargo.toml --summary-only

lint: analyze
	$(CARGO) clippy --manifest-path rust/Cargo.toml -- -D warnings

validate: format-check lint test build-rust

check: validate

sync:
	flutter_rust_bridge_codegen generate

build: build-macos

build-macos:
	$(FLUTTER) build macos

build-linux:
	$(FLUTTER) build linux

build-windows:
	$(FLUTTER) build windows

build-rust:
	$(CARGO) build --manifest-path rust/Cargo.toml

run: run-macos

run-macos:
	$(FLUTTER) run -d macos

run-linux:
	$(FLUTTER) run -d linux

run-windows:
	$(FLUTTER) run -d windows

run-release:
	$(FLUTTER) run -d macos --release

devices:
	$(FLUTTER) devices

doctor:
	$(FLUTTER) doctor -v
	$(CARGO) --version
	$(CARGO) clippy --version

info:
	@echo "=== Project ==="
	@echo "  root:    $(CURDIR)"
	@$(FLUTTER) --version
	@$(DART) --version
	@$(CARGO) --version
	@if command -v aws >/dev/null 2>&1; then aws --version; else echo "AWS CLI is not installed or not on PATH."; fi

aws-check:
	@if command -v aws >/dev/null 2>&1; then \
		aws --version; \
		echo "Configured AWS profiles:"; \
		aws configure list-profiles; \
	else \
		echo "AWS CLI is not installed or not on PATH."; \
	fi

upgrade:
	$(FLUTTER) pub upgrade --major-versions
	$(CARGO) update --manifest-path rust/Cargo.toml

clean:
	$(FLUTTER) clean
	$(CARGO) clean --manifest-path rust/Cargo.toml

bootstrap:
	mise install
	fvm install
	$(MAKE) setup
