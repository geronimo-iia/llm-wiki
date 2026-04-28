#!/usr/bin/env bash
section "6. Lint (MCP)"

run_mcp      "lint returns findings"                 "error\|warning" \
             wiki_lint

run_mcp      "lint broken-link rule"                 "broken-link" \
             wiki_lint '{"rules":["broken-link"]}'

run_mcp      "lint orphan rule"                      "orphan" \
             wiki_lint '{"rules":["orphan"]}'

run_mcp_json "lint json findings array"              \
             '.findings | type' "array" \
             wiki_lint '{"format":"json"}'

run_mcp_json "lint broken-link finds does-not-exist" \
             '[.findings[] | select(.rule=="broken-link")] | length > 0' "true" \
             wiki_lint '{"rules":["broken-link"],"format":"json"}'

run_mcp_json "lint orphan finds orphan-concept"      \
             '[.findings[] | select(.slug=="concepts/orphan-concept")] | length > 0' "true" \
             wiki_lint '{"rules":["orphan"],"format":"json"}'

run_mcp      "lint with wiki param"                  "error\|warning" \
             wiki_lint '{"wiki":"research"}'

run_mcp_json "lint findings have non-empty path"     \
             '[.findings[] | select(.path == "" or .path == null)] | length == 0' "true" \
             wiki_lint '{"rules":["broken-link"],"format":"json"}'

run_mcp_json "lint finding path ends with .md"       \
             '[.findings[] | select(.path | endswith(".md") | not)] | length == 0' "true" \
             wiki_lint '{"rules":["broken-link"],"format":"json"}'
