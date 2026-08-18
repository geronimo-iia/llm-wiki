# wiki_graph — JSON output format

**Date:** 2026-08-17
**Status:** Proposed

## Context

`wiki_graph` currently supports three output formats: `mermaid`, `dot`, and `llms`.
`GraphReport` is already `#[derive(Serialize, Deserialize)]`, so a `json` format
requires no structural changes — only a new dispatch branch in the handler.

## Problem

LLM clients and downstream tooling that need to traverse the graph programmatically
have no machine-readable option today. `mermaid` and `dot` require parsing a
domain-specific language. `llms` is human-readable prose, not structured data.

A JSON format would expose the full `GraphReport` — node list, edge list, community
assignments, and graph metrics — as a stable, queryable structure.

## Proposed change

Add `json` as a fourth value for the `format` parameter of `wiki_graph`.
The handler serializes `GraphReport` directly via `serde_json::to_string_pretty`.
No new types needed. The MCP tool description updates to:
`"Output format: mermaid | dot | llms | json (default: mermaid)"`.

## Why Post-1.0

- The API surface for `GraphReport` fields is not yet frozen. Adding `json` before
  stabilizing the struct risks a semver-breaking field rename after 1.0.0.
- No current consumer (MCP client or CLI user) has requested it.
- The `llms` format already covers the primary LLM use case.

## When to implement

When `GraphReport` field names are considered stable, or when a concrete consumer
(agent skill, external tool) requires structured graph output.
