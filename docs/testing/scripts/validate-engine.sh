#!/usr/bin/env bash
# validate-engine.sh — end-to-end CLI validation for llm-wiki v0.2.0
#
# Usage:
#   source ./docs/testing/scripts/setup-test-env.sh
#   LLM_WIKI_BIN=./target/release/llm-wiki ./docs/testing/scripts/validate-engine.sh
#
#   # Run a single section:
#   LLM_WIKI_BIN=./target/release/llm-wiki \
#   ./docs/testing/scripts/validate-engine.sh --section 05
#
# Requires: setup-test-env.sh sourced first, jq, git
# Run from the repo root.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURES="$(cd "$SCRIPT_DIR/../../.." && pwd)/tests/fixtures"

BINARY="${LLM_WIKI_BIN:-llm-wiki}"
SECTION_FILTER="${1:-}"
if [ "$SECTION_FILTER" = "--section" ]; then
    SECTION_FILTER="$2"
    shift 2
fi

# ── Require persistent test environment ──────────────────────────────────────

if [ -z "${LLM_WIKI_TEST_DIR:-}" ] || [ -z "${LLM_WIKI_CONFIG:-}" ]; then
    echo "ERROR: LLM_WIKI_TEST_DIR and LLM_WIKI_CONFIG are not set."
    echo "Run: source ./docs/testing/scripts/setup-test-env.sh"
    exit 1
fi

TEST_DIR="$LLM_WIKI_TEST_DIR"
CONFIG_FILE="$LLM_WIKI_CONFIG"
RESEARCH_ROOT="$TEST_DIR/wikis/research"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "ERROR: $CONFIG_FILE not found — run setup-test-env.sh first."
    exit 1
fi

# ── Counters ──────────────────────────────────────────────────────────────────

PASS=0
FAIL=0
SKIP=0

# ── Source helpers ────────────────────────────────────────────────────────────

# shellcheck source=lib/helpers.sh
source "$SCRIPT_DIR/lib/helpers.sh"

# ── Binary check ──────────────────────────────────────────────────────────────

if ! command -v "$BINARY" &>/dev/null && [ ! -x "$BINARY" ]; then
    red "ERROR: '$BINARY' not found."
    echo "Build with 'cargo build --release' and set LLM_WIKI_BIN or add to PATH."
    exit 1
fi

echo "llm-wiki validate-engine.sh"
echo "binary:   $($BINARY --version 2>/dev/null || echo unknown)"
echo "test dir: $TEST_DIR"

# Re-copy inbox fixtures so each run starts with a clean inbox
cp "$FIXTURES"/inbox/* "$RESEARCH_ROOT/wiki/inbox/"

CLI="$BINARY --config $CONFIG_FILE"

# ── Run sections ──────────────────────────────────────────────────────────────

for section_script in "$SCRIPT_DIR"/sections/[0-9][0-9]-*.sh; do
    num="$(basename "$section_script" | cut -c1-2)"
    if [ -n "$SECTION_FILTER" ] && [ "$num" != "$SECTION_FILTER" ]; then
        continue
    fi
    # shellcheck source=/dev/null
    source "$section_script"
done

# ── Summary ───────────────────────────────────────────────────────────────────

echo
echo "────────────────────────────────────────"
echo "Results: $(green "$PASS passed") | $([ $FAIL -gt 0 ] && red "$FAIL failed" || echo "$FAIL failed") | $SKIP skipped"
echo "────────────────────────────────────────"

[ $FAIL -eq 0 ]
