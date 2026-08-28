---
title: "Stats"
summary: "Wiki health dashboard — page counts, orphans, connectivity, staleness, structural topology."
read_when:
  - Assessing wiki health
  - Getting a quick overview of wiki state
status: ready
last_updated: "2026-05-04"
---

# Stats

MCP tool: `wiki_stats`

```
llm-wiki stats [--wiki <name>] [--format <fmt>]
```

Returns wiki health metrics in a single call. Composed from existing
primitives — no new index fields needed.

### Output

Text (default):

```
research — 42 pages, 3 sections
types:     concept(20) paper(15) article(5) section(3)
status:    active(38) draft(4)
orphans:   3
graph:     2.4 avg connections, 0.12 density
staleness: fresh(30) 7d(8) 30d(4)
index:     ok, built 2025-07-21T14:32:01Z
```

JSON (`--format json`, `detail: "summary"` default):

```json
{
  "wiki": "research",
  "pages": 42,
  "sections": 3,
  "types": { "concept": 20, "paper": 15, "article": 5, "section": 3 },
  "status": { "active": 38, "draft": 4 },
  "orphans": 3,
  "avg_connections": 2.4,
  "graph_density": 0.12,
  "staleness": {
    "fresh": 30,
    "stale_7d": 8,
    "stale_30d": 4
  },
  "index": {
    "stale": false,
    "built": "2025-07-21T14:32:01Z"
  },
  "communities": {
    "count": 7,
    "largest": 34,
    "smallest": 3
  },
  "diameter": 4.0,
  "radius": 2.0,
  "center_count": 1,
  "structural_note": null
}
```

With `detail: "full"`, `center_count` is replaced by `center: Vec<String>` (the full slug list).

`communities` is `null` when the wiki has fewer pages than `graph.min_nodes_for_communities` (default 30).

`diameter`, `radius`, and `center`/`center_count` are `null`/`0`/empty and `structural_note` is set when:
- `graph.structural_algorithms = false` — disabled in config
- `local_count > graph.max_nodes_for_diameter` (default 2000) — graph too large
- graph is not strongly connected — `diameter` returns `None` (no path between some node pairs)

```json
{
  "communities": null
}
```

When structural algorithms are skipped due to graph size:

```json
{
  "diameter": null,
  "radius": null,
  "center_count": 0,
  "structural_note": "graph too large for diameter computation (2500 nodes > max_nodes_for_diameter=2000)"
}
```

When graph is not strongly connected (common when isolated pages exist):

```json
{
  "diameter": null,
  "radius": null,
  "center_count": 0,
  "structural_note": "graph is not strongly connected — diameter undefined; use wiki_lint(rules: \"periphery,orphan\") to find disconnected pages"
}
```

### Metrics

| Metric | Source | Description |
|--------|--------|-------------|
| `pages` | tantivy count | Total indexed pages |
| `sections` | tantivy count | Section page count |
| `types` | facets | Page count per type |
| `status` | facets | Page count per status |
| `orphans` | graph | Pages with zero inbound edges |
| `avg_connections` | graph | Mean edges per node |
| `graph_density` | graph | edges / (nodes * (nodes-1)) |
| `staleness` | `last_updated` | Fixed buckets: fresh (<7d), stale_7d (7-30d), stale_30d (>30d) |
| `index` | index status | Stale flag and last build time |
| `communities` | Louvain (graph) | Cluster stats; `null` when pages < `min_nodes_for_communities` (default 30) |
| `diameter` | petgraph-live metrics | Longest shortest directed path; `null` when disabled or graph too large |
| `radius` | petgraph-live metrics | Minimum eccentricity; `null` under same conditions as `diameter` |
| `center` / `center_count` | petgraph-live metrics | Full slug list (`detail: "full"`) or count (`detail: "summary"`); `0`/empty when `diameter` is `null` |
| `structural_note` | computed | Explanation when `diameter` is `null` for any reason; `null` when diameter was computed |

### communities

Louvain community detection run on the undirected wiki graph. Present only when
`page_count >= graph.min_nodes_for_communities`.

| Field | Description |
|-------|-------------|
| `count` | Number of distinct knowledge clusters found |
| `largest` | Size of the biggest cluster (node count) |
| `smallest` | Size of the smallest cluster |

To find weakly connected pages, use `wiki_lint(rules: "periphery,orphan")` — it
returns slugs with richer output (path, message) and is not subject to a size cap.

### structural topology

`diameter`, `radius`, `center`/`center_count`, and `structural_note` are
computed via BFS from every local node on the directed `WikiGraph` — O(n·(n+e)).

`diameter` is `null` (with `structural_note` set) in three cases:

1. `graph.structural_algorithms = false` — disabled in config; note: `"structural algorithms disabled in config"`
2. `local_count > graph.max_nodes_for_diameter` (default 2000) — graph too large; note explains the threshold
3. Graph is not strongly connected — `petgraph` BFS returns `None` when no directed path exists between
   some node pair (common when isolated pages exist); note recommends `wiki_lint(rules: "periphery,orphan")`

When `diameter` is successfully computed, `structural_note` is `null`.

| Field | Description |
|-------|-------------|
| `diameter` | Longest shortest path between any two pages |
| `radius` | Minimum eccentricity — distance from the most central page to all others |
| `center` / `center_count` | Hub slugs (eccentricity = radius); full list in `detail: "full"`, count in `detail: "summary"` |
| `structural_note` | Non-null whenever `diameter` is `null`; explains the reason |

## MCP Tool Definition

```json
{
  "name": "wiki_stats",
  "description": "Wiki health dashboard — page counts, graph metrics, staleness, structural topology (diameter, radius, center)",
  "parameters": {
    "wiki": "target wiki name (default: default wiki)",
    "detail": "\"summary\" (default) — returns center_count instead of slug list, response under 2KB; \"full\" — returns full center slug list"
  }
}
```
