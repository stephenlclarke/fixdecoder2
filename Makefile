# Makefile — invokes helper functions from ./ci_helper.sh for common tasks

SHELL := /bin/bash
CI_SCRIPT := ./ci_helper.sh

.PHONY: setup-environment prepare build build-release scan coverage sonar clean help

setup-environment:
	@bash -lc 'source $(CI_SCRIPT) && cmd_setup_environment'

prepare:
	@bash -lc 'source $(CI_SCRIPT) && cmd_setup_environment && ensure_build_metadata && download_fix_specs'
	@cargo run --quiet --bin generate_sensitive_tags >/dev/null

build: prepare
	@bash -lc 'source $(CI_SCRIPT) && cargo fmt --all && cargo build'

build-release: prepare
	@bash -lc 'source $(CI_SCRIPT) && cargo fmt --all && cargo build --release'

scan: prepare
	@bash -lc 'source $(CI_SCRIPT) && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings'
	@command -v cargo-audit >/dev/null 2>&1 && cargo audit || echo "cargo-audit not installed; skipping security scan"

coverage: build
	@bash -lc '\
		source $(CI_SCRIPT) && \
		mkdir -p target/coverage && \
		cargo llvm-cov clean --workspace >/dev/null 2>&1 || true; \
		cargo llvm-cov --workspace --cobertura \
		  --ignore-filename-regex "src/fix/sensitive.rs|src/bin/generate_sensitive_tags.rs" \
		  --output-path target/coverage/coverage.xml \
	'

sonar: coverage
	@bash -lc '\
		source $(CI_SCRIPT) && \
		ensure_sonar_scanner && \
		sonar-scanner \
	'

clean:
	@cargo clean

help:
	@echo "Available targets:"
	@echo "  setup-environment  → ensure toolchain + coverage tools"
	@echo "  prepare            → setup + build metadata + download FIX specs + regenerate generators"
	@echo "  build              → fmt + cargo build (debug)"
	@echo "  build-release      → fmt + cargo build --release"
	@echo "  scan               → fmt --check + clippy (+ cargo-audit when available)"
	@echo "  coverage           → cargo llvm-cov --cobertura"
	@echo "  sonar              → sonar-scanner (requires coverage.xml)"
	@echo "  clean              → cargo clean"
	@echo "  help               → this help text"
