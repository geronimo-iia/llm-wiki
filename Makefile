# llm-wiki development Makefile
#
# Integration test targets require: llm-wiki binary on PATH or LLM_WIKI_BIN set.
# MCP tests require: mcptools (`mcp` command) — brew install mcp.

TEST_DIR  ?= $(HOME)/llm-wiki-testing
TEST_CFG  := $(TEST_DIR)/config.toml
DEBUG_BIN := ./target/debug/llm-wiki
REL_BIN   := ./target/release/llm-wiki
SCRIPTS   := docs/testing/scripts

.PHONY: build build-release test-clean test-setup validate validate-mcp validate-acp validate-engine pre-release

# ── Build ──────────────────────────────────────────────────────────────────────

build:
	cargo build

build-release:
	cargo build --release --locked

# ── Integration test environment ──────────────────────────────────────────────

test-clean:
	bash $(SCRIPTS)/clean-test-env.sh --dir $(TEST_DIR) --yes

test-setup: build
	LLM_WIKI_BIN=$(DEBUG_BIN) \
	LLM_WIKI_TEST_DIR=$(TEST_DIR) \
	bash $(SCRIPTS)/setup-test-env.sh --dir $(TEST_DIR)

# ── Validation scripts ────────────────────────────────────────────────────────

validate-engine: build
	LLM_WIKI_BIN=$(DEBUG_BIN) \
	LLM_WIKI_TEST_DIR=$(TEST_DIR) \
	LLM_WIKI_CONFIG=$(TEST_CFG) \
	bash $(SCRIPTS)/validate-engine.sh

validate-mcp: build
	LLM_WIKI_BIN=$(DEBUG_BIN) \
	LLM_WIKI_TEST_DIR=$(TEST_DIR) \
	LLM_WIKI_CONFIG=$(TEST_CFG) \
	bash $(SCRIPTS)/validate-mcp.sh

validate-acp: build
	LLM_WIKI_BIN=$(DEBUG_BIN) \
	LLM_WIKI_TEST_DIR=$(TEST_DIR) \
	LLM_WIKI_CONFIG=$(TEST_CFG) \
	bash $(SCRIPTS)/validate-acp.sh

validate: test-setup validate-engine validate-mcp validate-acp

# ── Pre-release checklist ─────────────────────────────────────────────────────
# Mirrors docs/guides/release.md pre-release checklist.
# Run before opening a release PR — all commands must pass.

pre-release:
	cargo test
	cargo test --doc
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo build --release --locked
