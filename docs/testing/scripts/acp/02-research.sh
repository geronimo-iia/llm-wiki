#!/usr/bin/env bash
section "2. Research Workflow (ACP)"

run_acp "bare prompt triggers research workflow" \
        "research" \
        "what is mixture of experts?" \
        "research"

run_acp "llm-wiki:research explicit prefix" \
        "research" \
        "llm-wiki:research scaling laws" \
        "research"

run_acp "research no match returns no results message" \
        "No results" \
        "llm-wiki:research zzz-no-match-guaranteed-xyz" \
        "research"

run_acp_json "research prompt response has stopReason=end_turn" \
             '.result.stopReason // empty' "end_turn" \
             "llm-wiki:research mixture of experts" \
             "research"
