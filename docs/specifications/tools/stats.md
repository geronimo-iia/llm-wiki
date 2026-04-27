---
title: "Stats"
summary: "Wiki health dashboard — page counts, orphans, connectivity, staleness."
read_when:
  - Assessing wiki health
  - Getting a quick overview of wiki state
status: ready
last_updated: "2025-07-23"
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

JSON (`--format json`):

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
    "smallest": 3,
    "isolated": ["tangent-thought-xyz", "draft-stub-abc"]
  }
}
```

`communities` is `null` when the wiki has fewer pages than `graph.min_nodes_for_communities` (default 30):

```json
{
  "communities": null
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

### communities

Louvain community detection run on the undirected wiki graph. Present only when
`page_count >= graph.min_nodes_for_communities`.

| Field | Description |
|-------|-------------|
| `count` | Number of distinct knowledge clusters found |
| `largest` | Size of the biggest cluster (node count) |
| `smallest` | Size of the smallest cluster |
| `isolated` | Slugs in communities of size ≤ 2 — weakly connected pages; sorted alphabetically |

`isolated` pages are the highest-priority candidates for new links. Run
`wiki_suggest(slug: "<isolated-slug>")` to find the best connections.

## MCP Tool Definition

```json
{
  "name": "wiki_stats",
  "description": "Wiki health dashboard",
  "parameters": {
    "wiki": "target wiki name (default: default wiki)"
  }
}
```
