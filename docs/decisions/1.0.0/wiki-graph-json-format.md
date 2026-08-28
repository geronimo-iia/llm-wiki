# wiki_graph — JSON output format

Date: 2026-08-17
Status: Implemented (1.0.0)

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


## When to implement

When `GraphReport` field names are considered stable, or when a concrete consumer
(agent skill, external tool) requires structured graph output.

## Implementation note

Implemented in 1.0.0. `GraphReport` fields (`nodes`, `edges`, `output`) were
already stable. The JSON output uses a dedicated `WikiGraphJson` type rather
than `GraphReport` to carry the full node list, edge list, metrics, and
community assignments — matching the original intent of the decision.
