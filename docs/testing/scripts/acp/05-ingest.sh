#!/usr/bin/env bash
section "5. Ingest Workflow (ACP)"

run_acp "ingest default path runs and returns summary" \
        "ingest\|page\|commit\|ingested" \
        "llm-wiki:ingest" \
        "research"

run_acp "ingest nonexistent path returns error" \
        "not found\|no such\|error\|Failed" \
        "llm-wiki:ingest /nonexistent-path-xyz" \
        "research"

run_acp_json "ingest prompt response has stopReason=end_turn" \
             '.result.stopReason // empty' "end_turn" \
             "llm-wiki:ingest" \
             "research"
