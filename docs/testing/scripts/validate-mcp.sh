#!/usr/bin/env bash
# validate-mcp.sh — end-to-end MCP validation for llm-wiki via mcptools stdio
#
# Usage:
#   LLM_WIKI_BIN=./target/release/llm-wiki \
#   ./docs/testing/scripts/validate-mcp.sh [--section NN]
#
# Requires:
#   mcp (mcptools)     — brew tap f/mcptools && brew install mcp
#   LLM_WIKI_TEST_DIR  — path to test dir (default: ~/llm-wiki-testing)
#   LLM_WIKI_CONFIG    — path to config.toml (default: $LLM_WIKI_TEST_DIR/config.toml)
#   LLM_WIKI_BIN       — path to llm-wiki binary (default: llm-wiki)
#
# Setup first:
#   source ./docs/testing/scripts/setup-test-env.sh

set -uo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_FIXTURES="$(cd "$_SCRIPT_DIR/../../.." && pwd)/tests/fixtures"

# ── Load helpers ──────────────────────────────────────────────────────────────

source "$_SCRIPT_DIR/lib/helpers.sh"
source "$_SCRIPT_DIR/lib/mcp-helpers.sh"

# ── Config ────────────────────────────────────────────────────────────────────

CLI="${LLM_WIKI_BIN:-llm-wiki}"
TEST_DIR="${LLM_WIKI_TEST_DIR:-$HOME/llm-wiki-testing}"
CONFIG="${LLM_WIKI_CONFIG:-$TEST_DIR/config.toml}"
RESEARCH_ROOT="$TEST_DIR/wikis/research"
SECTION_FILTER=""

# MCP_SERVER is passed to every `mcp call` invocation as the stdio server command
MCP_SERVER=("$CLI" --config "$CONFIG" serve)
export MCP_SERVER

# ── Args ──────────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --section) SECTION_FILTER="$2"; shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── Preflight ─────────────────────────────────────────────────────────────────

echo "llm-wiki validate-mcp.sh"
echo "binary:   $("$CLI" --version 2>/dev/null || echo unknown)"
echo "test dir: $TEST_DIR"
echo

if ! command -v mcp &>/dev/null; then
    red "ERROR: 'mcp' (mcptools) not found — cannot run MCP validation."
    echo "Install with:"
    echo "  brew tap f/mcptools && brew install mcp"
    echo "  or download from https://github.com/f/mcptools/releases"
    exit 1
fi

if [ ! -f "$CONFIG" ]; then
    red "ERROR: config not found at $CONFIG"
    echo "Run: source ./docs/testing/scripts/setup-test-env.sh"
    exit 1
fi

# Reset inbox fixtures and clear logs for a clean idempotent run
\cp -f "$_FIXTURES"/inbox/* "$RESEARCH_ROOT/wiki/inbox/" 2>/dev/null || true
rm -f "$TEST_DIR/logs"/*.log 2>/dev/null || true

# Rebuild index so all sections start from a known clean state
"$CLI" --config "$CONFIG" index rebuild --wiki research > /dev/null 2>&1
"$CLI" --config "$CONFIG" index rebuild --wiki notes    > /dev/null 2>&1

# ── Counters ──────────────────────────────────────────────────────────────────

PASS=0; FAIL=0; SKIP=0
export PASS FAIL SKIP

# ── Run sections ──────────────────────────────────────────────────────────────

run_section() {
    local num="$1" file="$2"
    if [ -n "$SECTION_FILTER" ] && [ "$SECTION_FILTER" != "$num" ]; then
        return
    fi
    # shellcheck disable=SC1090
    source "$file"
}

run_section "01" "$_SCRIPT_DIR/mcp/01-spaces.sh"
run_section "02" "$_SCRIPT_DIR/mcp/02-index.sh"
run_section "03" "$_SCRIPT_DIR/mcp/03-search.sh"
run_section "04" "$_SCRIPT_DIR/mcp/04-content.sh"
run_section "05" "$_SCRIPT_DIR/mcp/05-ingest.sh"
run_section "06" "$_SCRIPT_DIR/mcp/06-lint.sh"
run_section "07" "$_SCRIPT_DIR/mcp/07-graph.sh"
run_section "08" "$_SCRIPT_DIR/mcp/08-stats.sh"
run_section "09" "$_SCRIPT_DIR/mcp/09-suggest.sh"
run_section "10" "$_SCRIPT_DIR/mcp/10-export.sh"
run_section "11" "$_SCRIPT_DIR/mcp/11-schema.sh"
run_section "12" "$_SCRIPT_DIR/mcp/12-structural.sh"

# ── Summary ───────────────────────────────────────────────────────────────────

echo
echo "────────────────────────────────────────"
printf "Results: "
printf '\033[32m%s\033[0m' "$PASS passed"
printf " | "
[ "$FAIL" -gt 0 ] && printf '\033[31m%s\033[0m' "$FAIL failed" || printf "%s failed" "$FAIL"
printf " | "
printf '\033[33m%s\033[0m' "$SKIP skipped"
echo
echo "────────────────────────────────────────"

[ "$FAIL" -eq 0 ]
