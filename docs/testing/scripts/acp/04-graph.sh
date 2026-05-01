#!/usr/bin/env bash
section "4. Graph Workflow (ACP)"

run_acp "graph default renders node/edge count" \
        "nodes\|edges" \
        "llm-wiki:graph" \
        "research"

run_acp "graph missing slug returns error in tool call" \
        "not found\|no page\|error" \
        "llm-wiki:graph zzz-missing-root-slug" \
        "research"

run_acp_json "graph prompt response has stopReason=end_turn" \
             '.result.stopReason // empty' "end_turn" \
             "llm-wiki:graph" \
             "research"
