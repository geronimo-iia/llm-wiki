#!/usr/bin/env bash
section "4. Content (MCP)"

run_mcp      "content_read returns page body"        "Mixture of Experts" \
             wiki_content_read '{"uri":"concepts/mixture-of-experts"}'

run_mcp      "content_read includes frontmatter"     "type:" \
             wiki_content_read '{"uri":"concepts/mixture-of-experts"}'

run_mcp      "content_read with backlinks"           "backlinks" \
             wiki_content_read '{"uri":"concepts/mixture-of-experts","backlinks":true}'

run_mcp_json "backlinks includes CommonMark link source" \
             '[.backlinks // [] | .[] | .slug] | any(. == "concepts/mixture-of-experts")' "true" \
             wiki_content_read '{"uri":"concepts/scaling-laws","backlinks":true}'

run_mcp      "content_read via wiki:// URI"          "Mixture of Experts" \
             wiki_content_read '{"uri":"wiki://research/concepts/mixture-of-experts"}'

run_mcp_json "wiki_resolve existing slug has exists:true" \
             '.exists' "true" \
             wiki_resolve '{"uri":"concepts/mixture-of-experts"}'

run_mcp_json "wiki_resolve existing slug has .md path" \
             '.path | endswith(".md")' "true" \
             wiki_resolve '{"uri":"concepts/mixture-of-experts"}'

run_mcp_json "wiki_resolve non-existing slug has exists:false" \
             '.exists' "false" \
             wiki_resolve '{"uri":"concepts/does-not-exist-xyz"}'

run_mcp_json "wiki_resolve non-existing slug returns would-be path" \
             '.path | endswith(".md")' "true" \
             wiki_resolve '{"uri":"concepts/does-not-exist-xyz"}'

run_mcp_json "wiki_resolve returns slug and wiki_root" \
             '.slug | length > 0' "true" \
             wiki_resolve '{"uri":"concepts/mixture-of-experts"}'

# content_write, content_new, and content_commit mutate the wiki — skip to preserve test state
skip "content_write"  "mutates wiki state"
skip "content_new"    "mutates wiki state"
skip "content_commit" "mutates wiki state"
