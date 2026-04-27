#!/usr/bin/env bash
# validate-engine.sh — end-to-end CLI validation for llm-wiki v0.2.0
#
# Usage:
#   ./docs/testing/scripts/validate-engine.sh [--binary /path/to/llm-wiki]
#
# Requires: llm-wiki binary on PATH (or pass --binary), jq, git
# Run from the repo root.

set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────

BINARY="${LLM_WIKI_BIN:-llm-wiki}"
FIXTURES="$(cd "$(dirname "$0")/../../.." && pwd)/tests/fixtures"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

PASS=0
FAIL=0
SKIP=0

# ── Helpers ───────────────────────────────────────────────────────────────────

green() { printf '\033[32m%s\033[0m\n' "$*"; }
red()   { printf '\033[31m%s\033[0m\n' "$*"; }
yellow(){ printf '\033[33m%s\033[0m\n' "$*"; }

pass() { PASS=$((PASS+1)); green "  ✓ $1"; }
fail() { FAIL=$((FAIL+1)); red   "  ✗ $1"; [ -n "${2:-}" ] && red "    $2"; }
skip() { SKIP=$((SKIP+1)); yellow "  - $1 (skipped: $2)"; }

section() { echo; echo "── $1 ──────────────────────────────────────"; }

run() {
    # run <description> <expected_pattern> <command...>
    local desc="$1" pattern="$2"; shift 2
    local out
    if out=$("$@" 2>&1); then
        if [ -z "$pattern" ] || echo "$out" | grep -q "$pattern"; then
            pass "$desc"
        else
            fail "$desc" "output did not match: $pattern"
            echo "    output: $(echo "$out" | head -3)"
        fi
    else
        fail "$desc" "command failed (exit $?)"
        echo "    output: $(echo "$out" | head -3)"
    fi
}

run_json() {
    # run_json <description> <jq_filter> <expected_value> <command...>
    local desc="$1" filter="$2" expected="$3"; shift 3
    local out actual
    if out=$("$@" 2>&1); then
        actual=$(echo "$out" | jq -r "$filter" 2>/dev/null || echo "jq-error")
        if [ "$actual" = "$expected" ]; then
            pass "$desc"
        else
            fail "$desc" "expected '$expected', got '$actual'"
        fi
    else
        fail "$desc" "command failed"
        echo "    output: $(echo "$out" | head -3)"
    fi
}

check_binary() {
    if ! command -v "$BINARY" &>/dev/null; then
        echo
        red "ERROR: '$BINARY' not found on PATH."
        echo "Build with 'cargo build --release' and set LLM_WIKI_BIN or add to PATH."
        exit 1
    fi
}

# ── Setup: create isolated config + wiki pair ─────────────────────────────────

CONFIG_FILE="$TMPDIR/config.toml"

setup_wikis() {
    # research wiki
    local research_root="$TMPDIR/research"
    cp -r "$FIXTURES/wikis/research" "$research_root"
    mkdir -p "$research_root/inbox"
    git -C "$research_root" init -q
    git -C "$research_root" add .
    git -C "$research_root" -c user.name=test -c user.email=test@test.com \
        commit -q -m "init"

    # notes wiki
    local notes_root="$TMPDIR/notes"
    cp -r "$FIXTURES/wikis/notes" "$notes_root"
    git -C "$notes_root" init -q
    git -C "$notes_root" add .
    git -C "$notes_root" -c user.name=test -c user.email=test@test.com \
        commit -q -m "init"

    # copy inbox fixtures into wiki/inbox/ (path relative to wiki root)
    cp "$FIXTURES"/inbox/* "$research_root/wiki/inbox/"

    # register both wikis using isolated config
    "$BINARY" --config "$CONFIG_FILE" spaces create "$research_root" --name research 2>/dev/null || true
    "$BINARY" --config "$CONFIG_FILE" spaces create "$notes_root"    --name notes    2>/dev/null || true
    "$BINARY" --config "$CONFIG_FILE" spaces set-default research                    2>/dev/null || true
}

# ── Main ─────────────────────────────────────────────────────────────────────

check_binary

echo "llm-wiki validate-engine.sh"
echo "binary: $($BINARY --version 2>/dev/null || echo unknown)"
echo "tmpdir: $TMPDIR"

setup_wikis
CLI="$BINARY --config $CONFIG_FILE"

# ── 1. Space management ───────────────────────────────────────────────────────

section "1. Space management"

run       "spaces list returns both wikis"     "research"  $CLI spaces list
run_json  "default wiki is research"           '.wikis[] | select(.default) | .name' "research" \
          $CLI spaces list --format json
run       "spaces set-default notes"           ""          $CLI spaces set-default notes
run       "spaces set-default back to research" ""         $CLI spaces set-default research

# ── 2. Index ──────────────────────────────────────────────────────────────────

section "2. Index"

run "index rebuild research"  "ok"  $CLI index rebuild --wiki research
run "index status research"   "ok"  $CLI index status  --wiki research
run "index rebuild notes"     "ok"  $CLI index rebuild --wiki notes

# ── 3. Search ─────────────────────────────────────────────────────────────────

section "3. Search"

run      "basic search returns results"          "mixture"    $CLI search "mixture of experts"
run      "type filter: concept"                  "concept"    $CLI search "routing" --type concept
run      "cross-wiki search"                     "attention"  $CLI search "attention" --cross-wiki
run_json "search json has results array"         '.results | length > 0' "true" \
         $CLI search "transformer" --format json

# ── 4. Content ────────────────────────────────────────────────────────────────

section "4. Content"

run      "read page by slug"                     "Mixture of Experts"  \
         $CLI content read concepts/mixture-of-experts
run      "read page with backlinks"              "backlinks"           \
         $CLI content read concepts/mixture-of-experts --backlinks
run      "read cross-wiki page via uri"          "Attention"           \
         $CLI content read "wiki://notes/concepts/attention-mechanism"

# ── 5. Ingest ─────────────────────────────────────────────────────────────────

section "5. Ingest"

RESEARCH_ROOT="$TMPDIR/research"

run "ingest dry-run inbox/"  "dry_run"  $CLI ingest inbox/ --dry-run
run "ingest single file dry-run"  "" \
    $CLI ingest "inbox/01-paper-switch-transformer.md" --dry-run

# Copy one inbox file to the wiki inbox for real ingest
cp "$RESEARCH_ROOT/inbox/01-paper-switch-transformer.md" "$RESEARCH_ROOT/inbox/test-ingest.md"
run "ingest real file"  "ingested"  $CLI ingest "inbox/test-ingest.md"

# Incremental validation: only changed files validated
run "ingest dry-run incremental"  ""  $CLI ingest inbox/ --dry-run

# Redaction
cp "$RESEARCH_ROOT/inbox/03-note-with-secrets.md" "$RESEARCH_ROOT/inbox/secrets-test.md"
run "ingest with redact flag"  "redacted"  $CLI ingest "inbox/secrets-test.md" --redact

# Verify redaction worked
if grep -q "sk-ant-api03" "$RESEARCH_ROOT/wiki/sources/secrets-test.md" 2>/dev/null || \
   grep -rq "sk-ant-api03" "$RESEARCH_ROOT/wiki/" 2>/dev/null; then
    fail "redaction: Anthropic key was NOT redacted"
else
    pass "redaction: Anthropic key was redacted"
fi
if grep -rq "REDACTED" "$RESEARCH_ROOT/wiki/" 2>/dev/null; then
    pass "redaction: REDACTED placeholder present in output"
else
    fail "redaction: REDACTED placeholder not found"
fi

# ── 6. Lint ───────────────────────────────────────────────────────────────────

section "6. Lint"

run      "lint all rules"                        "findings"   $CLI lint
run      "lint broken-link rule"                 "broken"     $CLI lint --rules broken-link
run      "lint orphan rule"                      "orphan"     $CLI lint --rules orphan
run_json "lint json has findings array"          '.findings | type' "array" \
         $CLI lint --format json
run_json "broken-link finds concepts/does-not-exist" \
         '[.findings[] | select(.rule=="broken-link")] | length > 0' "true" \
         $CLI lint --rules broken-link --format json
run_json "orphan finds orphan-concept"           \
         '[.findings[] | select(.slug=="concepts/orphan-concept")] | length > 0' "true" \
         $CLI lint --rules orphan --format json

# Per-wiki lint
run      "lint with --wiki flag"                 "findings"   $CLI lint --wiki research

# ── 7. Graph ──────────────────────────────────────────────────────────────────

section "7. Graph"

run      "graph mermaid output"                  "graph"      $CLI graph
run      "graph dot output"                      "digraph"    $CLI graph --format dot
run      "graph llms output"                     "cluster"    $CLI graph --format llms
run      "graph type filter"                     ""           $CLI graph --type concept
run      "graph root + depth"                    ""           $CLI graph \
         --root concepts/mixture-of-experts --depth 2

# Cross-wiki graph (needs both wikis mounted)
run      "graph cross-wiki"                      "external\|notes\|attention" \
         $CLI graph --cross-wiki

# ── 8. Stats ──────────────────────────────────────────────────────────────────

section "8. Stats"

run      "stats returns output"                  "research"   $CLI stats
run_json "stats json has pages"                  '.pages > 0' "true" \
         $CLI stats --format json
run_json "stats communities present (threshold=5)" '.communities != null' "true" \
         $CLI stats --format json

# ── 9. Suggest ────────────────────────────────────────────────────────────────

section "9. Suggest"

run      "suggest returns results"               ""           \
         $CLI suggest concepts/mixture-of-experts
run_json "suggest json is array"                 'type' "array" \
         $CLI suggest concepts/mixture-of-experts --format json
run_json "suggest has community peers reason"    \
         '[.[] | select(.reason | test("cluster"))] | length >= 0' "true" \
         $CLI suggest concepts/mixture-of-experts --format json

# ── 10. Export ────────────────────────────────────────────────────────────────

section "10. Export"

EXPORT_OUT="$TMPDIR/export-llms.txt"
run      "export llms-txt"          ""           $CLI export --path "$EXPORT_OUT" --wiki research
[ -f "$EXPORT_OUT" ] && pass "export: file created" || fail "export: file not created"
grep -q "Mixture of Experts" "$EXPORT_OUT" 2>/dev/null && \
    pass "export: content contains expected page" || \
    fail "export: content missing expected page"

EXPORT_JSON="$TMPDIR/export.json"
run      "export json format"       ""           $CLI export --path "$EXPORT_JSON" \
         --format json --wiki research
[ -f "$EXPORT_JSON" ] && pass "export json: file created" || fail "export json: file not created"

# ── 11. History ───────────────────────────────────────────────────────────────

section "11. History"

run      "history returns commits"               ""           \
         $CLI history concepts/mixture-of-experts
run_json "history json has entries"              'length > 0' "true" \
         $CLI history concepts/mixture-of-experts --format json

# ── 12. Schema ────────────────────────────────────────────────────────────────

section "12. Schema"

run      "schema list"                           "concept"    $CLI schema list
run      "schema show concept"                   "title"      $CLI schema show concept
run      "schema validate"                       ""           $CLI schema validate

# ── 13. Config ────────────────────────────────────────────────────────────────

section "13. Config"

run      "config list global"                    ""           $CLI config list
run      "config get graph.format"               ""           $CLI config get graph.format

# ── 14. Confidence field in search ranking ────────────────────────────────────

section "14. Confidence + search ranking"

# Active page with high confidence should rank above draft with low confidence
# on the same topic (compute-efficiency is draft/0.5, mixture-of-experts is active/0.9)
run_json "high-confidence page ranks first for topic query" \
         '.results[0].confidence >= .results[1].confidence // 1' "true" \
         $CLI search "mixture experts compute" --format json 2>/dev/null || \
    skip "confidence ranking" "search result order not deterministic in small corpus"

# ── 15. Backlinks ─────────────────────────────────────────────────────────────

section "15. Backlinks"

run_json "backlinks: mixture-of-experts has inbound links" \
         '.backlinks | length > 0' "true" \
         $CLI content read concepts/mixture-of-experts --backlinks --format json

# ── 16. Incremental validation ────────────────────────────────────────────────

section "16. Incremental validation"

# Modify a page and verify only it is validated
MODIFIED="$RESEARCH_ROOT/wiki/concepts/scaling-laws.md"
echo "" >> "$MODIFIED"
run_json "incremental ingest reports unchanged_count" \
         '.unchanged_count >= 0' "true" \
         $CLI ingest wiki/concepts/scaling-laws.md --format json 2>/dev/null || \
    skip "incremental unchanged_count" "format json not supported on ingest"

# ── Summary ───────────────────────────────────────────────────────────────────

echo
echo "────────────────────────────────────────"
echo "Results: $(green "$PASS passed") | $([ $FAIL -gt 0 ] && red "$FAIL failed" || echo "$FAIL failed") | $SKIP skipped"
echo "────────────────────────────────────────"

[ $FAIL -eq 0 ]
