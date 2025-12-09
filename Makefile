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
	@bash -lc 'source $(CI_SCRIPT) && ensure_build_metadata && cargo fmt --all && cargo build'

build-release: prepare
	@bash -lc 'source $(CI_SCRIPT) && ensure_build_metadata && cargo fmt --all && cargo build --release'
	@python3 ci/update_readme.py

.PHONY: update-readme
update-readme:
	@python3 ci/update_readme.py

scan: prepare
	@bash -lc 'source $(CI_SCRIPT) && ensure_build_metadata && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings'
	@command -v cargo-audit >/dev/null 2>&1 && cargo audit || echo "cargo-audit not installed; skipping security scan"

coverage: build
	@bash -lc '\
		source $(CI_SCRIPT) && \
		ensure_build_metadata && \
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
	@py=$$(command -v python3 || command -v python || true); \
	if [ -z "$$py" ]; then \
		echo "python3 (or python) is required for release bumping." >&2; \
		exit 1; \
	fi; \
	cur=$$($$py - <<'PY' \
import re, sys, pathlib \
toml = pathlib.Path("Cargo.toml").read_text() \
m = re.search(r'^version\\s*=\\s*"([0-9]+\\.[0-9]+\\.[0-9]+)"', toml, re.M) \
print(m.group(1) if m else "") \
PY \
	); \
	if [ -z "$$cur" ]; then echo "Could not read version from Cargo.toml" >&2; exit 1; fi; \
	next=$$($$py - <<'PY' "$$cur" \
import sys \
major, minor, patch = map(int, sys.argv[1].split(".")) \
patch += 1 \
print(f"{major}.{minor}.{patch}") \
PY \
	); \
	if ! git diff --quiet || ! git diff --cached --quiet; then \
		echo "Working tree is not clean; commit or stash changes before tagging." >&2; \
		exit 1; \
	fi; \
	if git rev-parse "v$$next" >/dev/null 2>&1; then \
		echo "Tag v$$next already exists; aborting." >&2; \
		exit 1; \
	fi; \
	$$py - <<'PY' "$$cur" "$$next" \
import pathlib, re, sys \
cur, new = sys.argv[1], sys.argv[2] \
path = pathlib.Path("Cargo.toml") \
text = path.read_text() \
text = re.sub(r'^version\\s*=\\s*"' + re.escape(cur) + r'"', f'version = "{new}"', text, count=1, flags=re.M) \
path.write_text(text) \
PY \
	; \
	echo "Bumped version: $$cur -> $$next"; \
	git add Cargo.toml; \
	git commit -m "chore(release): v$$next"; \
	git tag -a "v$$next" -m "Release v$$next"; \
	echo "Created tag v$$next"

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
	@echo "  release            → bump patch version, commit, and tag v<version>"
	@echo "  clean              → cargo clean"
	@echo "  help               → this help text"
