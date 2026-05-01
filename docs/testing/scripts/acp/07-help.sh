#!/usr/bin/env bash
section "7. Help & Unknown Workflows (ACP)"

run_acp "llm-wiki:help returns workflow listing" \
        "research\|lint\|graph\|ingest\|use" \
        "llm-wiki:help" \
        "research"

run_acp "unknown workflow returns error + listing" \
        "Unknown workflow\|Available" \
        "llm-wiki:bogus-command" \
        "research"

run_acp "unknown workflow message contains workflow list" \
        "research" \
        "llm-wiki:bogus-command" \
        "research"
