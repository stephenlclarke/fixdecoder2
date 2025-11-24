# Makefile — invokes helper functions from ./ci/ci_helper.sh for common tasks

SHELL := /bin/bash
CI_SCRIPT := ./ci/ci_helper.sh

.PHONY: setup-environment prepare build build-release scan coverage sonar release clean help

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

release:
	@ver=$$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/'); \
	if git rev-parse "v$$ver" >/dev/null 2>&1; then \
		echo "Tag v$$ver already exists; aborting."; \
		exit 1; \
	fi; \
	if ! git diff --quiet || ! git diff --cached --quiet; then \
		echo "Working tree is not clean; commit or stash changes before tagging."; \
		exit 1; \
	fi; \
	git tag -a "v$$ver" -m "Release v$$ver"; \
	echo "Created tag v$$ver"

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
