# wiki_stats — remove isolated slug list, add detail parameter

**Date:** 2026-08-20
**Status:** Proposed

## Context

`wiki_stats` returns a single `WikiStats` struct covering page counts, graph
metrics, staleness buckets, index health, community detection results, and
structural topology. The tool has one parameter: `wiki`.

At 1,315 pages the response is 275KB. The bloat comes from two fields:

- `communities.isolated: Vec<String>` — the full slug list of pages in
  communities of size ≤ 2. At 1,241 entries this alone accounts for the
  majority of the response.
- `center: Vec<String>` — hub page slugs. At scale this can be dozens of
  entries.

An agent that needs `avg_connections`, `orphans`, and `isolated count` must
receive and discard ~274KB of slug lists to extract three numbers. The only
working flow observed in production was: call `wiki_stats`, save the 275KB
response to disk, parse with Python.

## Problem

Two distinct issues:

**1. `communities.isolated` is redundant with `wiki_lint`.**

`wiki_lint` already returns per-page findings with slug, path, and message for
the same structural concerns:

| `communities.isolated` source | `wiki_lint` equivalent |
|---|---|
| Pages in communities of size ≤ 2 (Louvain) | `rules: "periphery"` — structurally peripheral pages |
| Pages with no incoming links | `rules: "orphan"` |
| Structural connectors | `rules: "articulation-point"` |

Keeping `isolated: Vec<String>` in `wiki_stats` creates two sources of truth
that can diverge: lint uses graph traversal and index queries; community
detection uses Louvain clustering. They do not produce identical sets. An agent
that wants isolated page slugs should call `wiki_lint` — it gets richer output
(slug + path + message + severity) and a single authoritative source.

**2. `WikiStats` conflates triage and investigation.**

Triage — "is this wiki healthy?" — needs numbers, not lists. Investigation —
"which pages are structural hubs?" — needs slug lists. There is no way to
request triage output today.

## Decision

### Remove `isolated: Vec<String>` from `CommunityStats` permanently

Drop the field entirely — not gated on a `detail` parameter. `CommunityStats`
becomes:

```json
"communities": {
  "count": 74,
  "largest": 42,
  "smallest": 1
}
```

Callers that need isolated page slugs use `wiki_lint rules: "periphery,orphan"`.
This is a breaking change to `CommunityStats` but the field was already
unusable at scale and redundant with lint.

### Add `detail` parameter for `center`

`center: Vec<String>` (hub page slugs) has no lint equivalent — it is unique
to `wiki_stats`. Gate it on `detail: "summary"` (default) vs `detail: "full"`:

- `detail: "summary"` (new default): `center` is replaced by `center_count: usize`.
- `detail: "full"`: `center` returns the full slug list. Identical to current
  behaviour for this field.

Response size for a 1,315-page wiki with `detail: "summary"`: under 2KB.

## `structural_note` silent null fix

Separate but related: when `structural_algorithms: false` in config, `diameter`,
`radius`, and `center` are all `null` with no explanation. `structural_note` is
only set when `local_count > max_nodes_for_diameter`, not when the algorithms
are disabled by config. Fix: set `structural_note` in the
`!resolved.graph.structural_algorithms` branch:

```rust
(None, None, vec![], Some("structural algorithms disabled in config".to_string()))
```

## Implementation notes

- Remove `isolated: Vec<String>` from `CommunityStats` in `src/graph.rs`.
  Update the single construction site in `build_community_data`.
- `handle_stats` in `src/mcp/handlers.rs` extracts `detail` arg.
- `WikiStats` uses `#[serde(skip_serializing_if = "Option::is_none")]` on
  `center: Option<Vec<String>>` (present only when `detail: "full"`) and adds
  `center_count: usize` (always present).
- Tool schema adds `"detail": opt_str("Response detail: summary (default) | full. Use full to retrieve center hub slug list.")`.
- `structural_note` fix is a one-line change — ship in the same commit.

## Alternatives considered

**Gate `isolated` on `detail: "full"` instead of removing it**: rejected —
it is redundant with lint and creates a divergent second source of truth.
Removing it is cleaner than hiding it.

**Separate `wiki_stats_summary` tool**: rejected — tool count is already at 24;
a parameter is the right primitive for a detail level toggle.

**Pagination over isolated nodes**: rejected — an agent that needs isolated
page slugs should use `wiki_lint`, which already paginates via `page_size`
and `cursor`.
