#!/usr/bin/env bash
# validate-acp.sh — end-to-end ACP validation for llm-wiki via stdio JSON-RPC
#
# Usage:
#   LLM_WIKI_BIN=./target/release/llm-wiki \
#   ./docs/testing/scripts/validate-acp.sh [--section NN]
#
# Requires:
#   jq                 — brew install jq
#   LLM_WIKI_TEST_DIR  — path to test dir (default: ~/llm-wiki-testing)
#   LLM_WIKI_CONFIG    — path to config.toml (default: $LLM_WIKI_TEST_DIR/config.toml)
#   LLM_WIKI_BIN       — path to llm-wiki binary (default: llm-wiki)
#   ACP_TIMEOUT        — seconds to wait per exchange (default: 15)
#   ACP_HTTP_PORT      — port for MCP HTTP sidecar so stdio is free for ACP (default: 18765)
#
# Setup first:
#   source ./docs/testing/scripts/setup-test-env.sh

set -uo pipefail

_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ── Load helpers ──────────────────────────────────────────────────────────────

source "$_SCRIPT_DIR/lib/helpers.sh"
source "$_SCRIPT_DIR/lib/acp-helpers.sh"

# ── Config ────────────────────────────────────────────────────────────────────

CLI="${LLM_WIKI_BIN:-llm-wiki}"
TEST_DIR="${LLM_WIKI_TEST_DIR:-$HOME/llm-wiki-testing}"
CONFIG="${LLM_WIKI_CONFIG:-$TEST_DIR/config.toml}"
ACP_TIMEOUT="${ACP_TIMEOUT:-15}"
ACP_HTTP_PORT="${ACP_HTTP_PORT:-18765}"
SECTION_FILTER=""
export ACP_TIMEOUT ACP_HTTP_PORT TEST_DIR CONFIG CLI

# ACP uses stdio; MCP must be displaced to HTTP so both don't share the pipe.
# --http :PORT forces MCP onto HTTP, leaving stdio exclusively for ACP.
ACP_SERVER=("$CLI" --config "$CONFIG" serve --acp --http ":$ACP_HTTP_PORT")
export ACP_SERVER

# ── Args ──────────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --section) SECTION_FILTER="$2"; shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── Preflight ─────────────────────────────────────────────────────────────────

echo "llm-wiki validate-acp.sh"
echo "binary:   $("$CLI" --version 2>/dev/null || echo unknown)"
echo "test dir: $TEST_DIR"
echo "timeout:  ${ACP_TIMEOUT}s per exchange"
echo

if ! command -v jq &>/dev/null; then
    red "ERROR: 'jq' not found — required for ACP validation."
    echo "Install with: brew install jq"
    exit 1
fi

if [ ! -f "$CONFIG" ]; then
    red "ERROR: config not found at $CONFIG"
    echo "Run: source ./docs/testing/scripts/setup-test-env.sh"
    exit 1
fi

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

run_section "01" "$_SCRIPT_DIR/acp/01-lifecycle.sh"
run_section "02" "$_SCRIPT_DIR/acp/02-research.sh"
run_section "03" "$_SCRIPT_DIR/acp/03-lint.sh"
run_section "04" "$_SCRIPT_DIR/acp/04-graph.sh"
run_section "05" "$_SCRIPT_DIR/acp/05-ingest.sh"
run_section "06" "$_SCRIPT_DIR/acp/06-use.sh"
run_section "07" "$_SCRIPT_DIR/acp/07-help.sh"
run_section "08" "$_SCRIPT_DIR/acp/08-session-cap.sh"

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
