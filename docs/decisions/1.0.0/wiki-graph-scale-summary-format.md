# wiki_graph — scale problem and summary mode

Date: 2026-08-20
Status: Proposed

## Context

`wiki_graph` renders the concept graph in one of four formats: `mermaid`,
`dot`, `llms`, or `json`. The output is returned as a single string in the
MCP response. Existing filters (`root`, `depth`, `type`, `relation`) allow
subgraph scoping, but all are optional — an unfiltered call renders the full
graph.

Output size grows linearly with nodes and edges:

| Format | Growth | At 1,315 nodes + ~3,700 edges |
|--------|--------|-------------------------------|
| `mermaid` | ~50 chars/node + ~60 chars/edge | ~270KB |
| `dot` | ~40 chars/node + ~50 chars/edge | ~240KB |
| `llms` | prose, bounded by top-N hubs | ~5–15KB |
| `json` | ~120 bytes/node + ~80 bytes/edge | ~450KB |

`format: "llms"` is already compact because it summarises by type group and
caps hub listings. `mermaid`, `dot`, and `json` are unbounded.

## Problem

An agent calling `wiki_graph()` with no filters on a 1,315-page wiki receives
a 270–450KB response depending on format. This exceeds practical LLM context
budgets and is rarely what the agent needs — it typically wants topology
insight, not a full node/edge dump.

The existing filters solve the problem when the agent knows what to scope to.
The gap is the first call: an agent with no prior knowledge of the wiki
structure has no basis for choosing a `root` or `type` filter. It needs a
cheap overview first.

`format: "llms"` is already close to a summary — it produces type group
counts, top hubs by degree, relation counts, and isolated node titles. But
it still lists all isolated node titles (unbounded at scale) and all titles
within each type group.

## Decision

### Add `format: "summary"`

A new format value that returns only aggregate metrics — no node or edge
enumeration:

```json
{
  "nodes": 1315,
  "edges": 3742,
  "external_refs": 48,
  "by_type": { "concept": 412, "source": 287, "doc": 198, "section": 89 },
  "top_hubs": [
    { "slug": "concepts/transformer", "degree": 24 },
    { "slug": "concepts/moe", "degree": 18 }
  ],
  "relation_counts": { "links-to": 2841, "fed-by": 612, "depends-on": 289 },
  "isolated_count": 71,
  "communities": { "count": 74, "largest": 42, "smallest": 1 }
}
```

Response size: under 2KB regardless of wiki size. `top_hubs` is capped at 10
(configurable via a `limit` parameter, default 10).

No node list, no edge list, no slug enumeration. An agent uses this to decide
which filter to apply next — `type`, `root`, or `relation`.

### Cap `isolated` titles in `format: "llms"`

`render_llms` currently lists all isolated node titles. At 1,241 isolated
nodes this alone is large. Cap at 20, append `"… and N more (use
wiki_lint(rules: \"orphan,periphery\") for the full list)"` when truncated.

### Document the intended call sequence in the tool description

> For large wikis: call with `format: "summary"` first to understand topology.
> Then scope with `type`, `root`, or `relation` filters. Use `format: "llms"`
> for interpretation of a scoped subgraph. Use `format: "mermaid"` or `"dot"`
> only for visualization of scoped subgraphs — unfiltered on large wikis these
> exceed context limits.

## Implementation notes

- Add `"summary"` arm to the `match fmt` dispatch in `src/ops/graph.rs`.
- Add `render_summary(graph: &WikiGraph, top_n: usize) -> String` in
  `src/graph.rs` — single pass over nodes and edges, same pattern as
  `render_llms`. Returns JSON via `serde_json`.
- `GraphSummary` struct: `nodes`, `edges`, `external_refs`, `by_type`,
  `top_hubs`, `relation_counts`, `isolated_count`, `communities`.
- `top_hubs` capped at `top_n` (passed from handler via a `limit` param,
  default 10).
- Cap isolated titles in `render_llms`: after collecting `isolated: Vec<String>`,
  truncate to 20 before writing, append count note if truncated.
- Tool schema: add `"summary"` to the `format` description string. Add call
  sequence guidance.
- No change to `mermaid`, `dot`, `llms`, `json` render paths.

## Alternatives considered

**Pagination over nodes/edges in `mermaid`/`dot`**: rejected — these formats
are not meaningful when split across pages; a partial Mermaid diagram is not
renderable.

**Hard limit on unfiltered graph size**: rejected — would silently truncate
the graph with no indication of what was dropped. A summary mode is more
honest and more useful.

**Rename `format: "llms"` to `format: "summary"`**: rejected — `llms` is
already documented and used in skills. Adding `summary` as a distinct,
strictly-bounded format is cleaner.
