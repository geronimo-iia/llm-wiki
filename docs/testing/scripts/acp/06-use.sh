#!/usr/bin/env bash
section "6. Use Workflow (ACP)"

# Find a valid slug from the research wiki for testing
_ACP_TEST_SLUG=""
if command -v "$CLI" &>/dev/null && [ -f "$CONFIG" ]; then
    _ACP_TEST_SLUG=$("$CLI" --config "$CONFIG" list --wiki research --format json 2>/dev/null | \
        jq -r '.pages[0].slug // empty' 2>/dev/null | head -1)
fi

if [ -n "$_ACP_TEST_SLUG" ]; then
    run_acp "use existing slug streams page content" \
            "." \
            "llm-wiki:use $_ACP_TEST_SLUG" \
            "research"
else
    skip "use existing slug streams page content" "could not resolve a slug"
fi

run_acp "use without slug returns usage message" \
        "Usage" \
        "llm-wiki:use" \
        "research"

run_acp "use missing slug returns error" \
        "not found\|No page\|error" \
        "llm-wiki:use zzz-missing-slug-xyz" \
        "research"
