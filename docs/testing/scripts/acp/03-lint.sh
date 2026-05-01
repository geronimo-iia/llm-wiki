#!/usr/bin/env bash
section "3. Lint Workflow (ACP)"

run_acp "lint all rules runs and returns summary" \
        "lint" \
        "llm-wiki:lint" \
        "research"

run_acp "lint specific rule: orphan" \
        "orphan" \
        "llm-wiki:lint orphan" \
        "research"

run_acp "lint comma-separated rules" \
        "stale\|broken" \
        "llm-wiki:lint stale,broken-link" \
        "research"

run_acp_json "lint prompt response has stopReason=end_turn" \
             '.result.stopReason // empty' "end_turn" \
             "llm-wiki:lint" \
             "research"
