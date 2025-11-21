# Makefile — delegates to ./ci.sh

CI_SCRIPT := ./ci.sh

.PHONY: setup-environment build build-release scan coverage sonar clean help

build build-release scan coverage sonar clean setup-environment:
	$(CI_SCRIPT) $@

help:
	@echo "Available targets:"
	@echo "  build              → $(CI_SCRIPT) build"
	@echo "  build-release      → $(CI_SCRIPT) build-release"
	@echo "  setup-environment  → $(CI_SCRIPT) setup-environment"
	@echo "  scan               → $(CI_SCRIPT) scan"
	@echo "  coverage           → $(CI_SCRIPT) coverage"
	@echo "  sonar              → $(CI_SCRIPT) sonar"
	@echo "  clean              → $(CI_SCRIPT) clean"
	@echo "  help               → this help text"
