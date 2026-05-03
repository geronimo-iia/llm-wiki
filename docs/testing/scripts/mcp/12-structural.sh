#!/usr/bin/env bash
section "12. Structural lint rules (MCP)"

run_mcp_json "lint articulation-point returns valid JSON with findings array" \
             '.findings | type' "array" \
             wiki_lint '{"rules":"articulation-point"}'

run_mcp_json "lint bridge returns valid JSON with findings array" \
             '.findings | type' "array" \
             wiki_lint '{"rules":"bridge"}'

run_mcp_json "lint periphery returns valid JSON with findings array" \
             '.findings | type' "array" \
             wiki_lint '{"rules":"periphery"}'

run_mcp_json "lint all rules includes structural"        \
             '.findings | map(.rule) | any(. == "articulation-point" or . == "bridge" or . == "periphery")' "true" \
             wiki_lint '{}'

run_mcp_json "lint articulation-point finding has slug"  \
             'if .findings | length > 0 then .findings[0].slug | length > 0 else true end' "true" \
             wiki_lint '{"rules":"articulation-point"}'
