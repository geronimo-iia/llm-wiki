#!/usr/bin/env bash
# setup-test-env.sh — create a persistent test environment for llm-wiki validation
#
# Source this script to also export the env vars needed by validate-engine.sh:
#
#   source ./docs/testing/scripts/setup-test-env.sh [--dir <path>]
#
# Or run directly (env vars won't be set in parent shell):
#
#   ./docs/testing/scripts/setup-test-env.sh [--dir <path>]
#
# Creates a stable testing layout at $HOME/llm-wiki-testing (or --dir path).
# Run from the repo root. Requires: llm-wiki binary on PATH (or LLM_WIKI_BIN), git.

# Detect if sourced or executed (works in bash and zsh)
_SOURCED=0
if [ -n "${ZSH_VERSION:-}" ]; then
    case "$ZSH_EVAL_CONTEXT" in *:file*) _SOURCED=1 ;; esac
elif [ -n "${BASH_VERSION:-}" ]; then
    [[ "${BASH_SOURCE[0]}" != "$0" ]] && _SOURCED=1
fi

# ── Resolve script location regardless of sourcing ───────────────────────────

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"

BINARY="${LLM_WIKI_BIN:-llm-wiki}"
FIXTURES="$(cd "$_SCRIPT_DIR/../../.." && pwd)/tests/fixtures"
TEST_DIR="${LLM_WIKI_TEST_DIR:-$HOME/llm-wiki-testing}"

# ── Parse args ────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) TEST_DIR="$2"; shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── Helpers ───────────────────────────────────────────────────────────────────

green()  { printf '\033[32m%s\033[0m\n' "$*"; }
red()    { printf '\033[31m%s\033[0m\n' "$*"; }
yellow() { printf '\033[33m%s\033[0m\n' "$*"; }

if ! command -v "$BINARY" &>/dev/null && [ ! -x "$BINARY" ]; then
    red "ERROR: '$BINARY' not found."
    echo "Build with 'cargo build --release' and set LLM_WIKI_BIN or add to PATH."
    return 1 2>/dev/null || exit 1
fi

if ! command -v mcp &>/dev/null; then
    yellow "WARNING: 'mcp' (mcptools) not found — MCP server validation will not work."
    echo "Install with:"
    echo "  brew tap f/mcptools && brew install mcp"
    echo "  or download from https://github.com/f/mcptools/releases"
fi

# ── Main ─────────────────────────────────────────────────────────────────────

echo "llm-wiki setup-test-env.sh"
echo "binary:   $("$BINARY" --version 2>/dev/null || echo unknown)"
echo "test dir: $TEST_DIR"
echo "fixtures: $FIXTURES"
echo

CONFIG_FILE="$TEST_DIR/config.toml"
RESEARCH_ROOT="$TEST_DIR/wikis/research"
NOTES_ROOT="$TEST_DIR/wikis/notes"

mkdir -p "$TEST_DIR/wikis"

# research wiki
if [ -d "$RESEARCH_ROOT" ]; then
    echo "  research wiki already exists — skipping copy"
else
    cp -r "$FIXTURES/wikis/research" "$RESEARCH_ROOT"
    mkdir -p "$RESEARCH_ROOT/wiki/inbox"
    git -C "$RESEARCH_ROOT" init -q
    git -C "$RESEARCH_ROOT" add .
    git -C "$RESEARCH_ROOT" -c user.name=test -c user.email=test@test.com \
        commit -q -m "init"
    green "  ✓ research wiki created"
fi

# notes wiki
if [ -d "$NOTES_ROOT" ]; then
    echo "  notes wiki already exists — skipping copy"
else
    cp -r "$FIXTURES/wikis/notes" "$NOTES_ROOT"
    git -C "$NOTES_ROOT" init -q
    git -C "$NOTES_ROOT" add .
    git -C "$NOTES_ROOT" -c user.name=test -c user.email=test@test.com \
        commit -q -m "init"
    green "  ✓ notes wiki created"
fi

# inbox documents
echo "  copying inbox fixtures → $RESEARCH_ROOT/wiki/inbox/"
\cp -f "$FIXTURES"/inbox/* "$RESEARCH_ROOT/wiki/inbox/"
green "  ✓ inbox documents copied"

# register wikis
"$BINARY" --config "$CONFIG_FILE" spaces create "$RESEARCH_ROOT" --name research 2>/dev/null || true
"$BINARY" --config "$CONFIG_FILE" spaces create "$NOTES_ROOT"    --name notes    2>/dev/null || true
"$BINARY" --config "$CONFIG_FILE" spaces set-default research                    2>/dev/null || true
green "  ✓ wikis registered in $CONFIG_FILE"

# Always patch log_path to keep logs inside the test dir
sed -i.bak "s|log_path = \".*\"|log_path = \"$TEST_DIR/logs\"|" "$CONFIG_FILE" && rm -f "$CONFIG_FILE.bak"
green "  ✓ log_path set to $TEST_DIR/logs"

# ACP test config: low session cap so validate-acp.sh section 08 can test the limit
"$BINARY" --config "$CONFIG_FILE" config set serve.acp_max_sessions 3 --global 2>/dev/null || true
green "  ✓ serve.acp_max_sessions set to 3 (for ACP cap test)"


# ── Export env vars ───────────────────────────────────────────────────────────

export LLM_WIKI_TEST_DIR="$TEST_DIR"
export LLM_WIKI_CONFIG="$CONFIG_FILE"

echo
green "Setup complete."
if [ "$_SOURCED" = "1" ]; then
    green "Env vars exported to current shell:"
    echo "  LLM_WIKI_TEST_DIR=$LLM_WIKI_TEST_DIR"
    echo "  LLM_WIKI_CONFIG=$LLM_WIKI_CONFIG"
else
    echo
    echo "NOTE: env vars were NOT exported (script was executed, not sourced)."
    echo "To export them, run:"
    echo "  source ./docs/testing/scripts/setup-test-env.sh"
    echo
    echo "Or set them manually:"
    echo "  export LLM_WIKI_TEST_DIR=$TEST_DIR"
    echo "  export LLM_WIKI_CONFIG=$CONFIG_FILE"
fi
echo
echo "Run validation:"
echo "  LLM_WIKI_BIN=./target/release/llm-wiki \\"
echo "  ./docs/testing/scripts/validate-engine.sh"
echo
echo "  LLM_WIKI_BIN=./target/release/llm-wiki \\"
echo "  ./docs/testing/scripts/validate-acp.sh"
echo
echo "Clean up:"
echo "  source ./docs/testing/scripts/clean-test-env.sh"
