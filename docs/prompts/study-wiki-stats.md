# Study: wiki_stats — wiki health dashboard

A dedicated tool for wiki health metrics. Bootstrap uses facets for
page counts, but a comprehensive health view needs more: orphan count,
connectivity, staleness distribution, graph density.

## Problem

Understanding wiki health requires multiple tool calls: `wiki_list`
for facets, `wiki_graph` for connectivity, `wiki_index_status` for
index health. No single view shows the overall state.

## Proposed behavior

### CLI

```
llm-wiki stats [--wiki <name>] [--format <fmt>]
```

### MCP

```json
{
  "wiki": "research"
}
```

### Response

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
  }
}
```

## Metrics

| Metric | Source | Description |
|--------|--------|-------------|
| `pages` | tantivy count | Total indexed pages |
| `sections` | tantivy count (type=section) | Section count |
| `types` | facets | Page count per type |
| `status` | facets | Page count per status |
| `orphans` | graph | Pages with zero inbound edges |
| `avg_connections` | graph | Mean edges per node |
| `graph_density` | graph | edges / (nodes * (nodes-1)) |
| `staleness` | `last_updated` field | Pages by age bucket |
| `index` | index status | Index health |

## Implementation

Compose from existing primitives:
- `wiki_list` with facets → types, status, page count
- `wiki_graph` → orphans, connectivity, density
- `wiki_index_status` → index health
- Tantivy query on `last_updated` field → staleness buckets

No new index fields needed. The tool orchestrates existing queries.

## Interaction with existing features

- Bootstrap: `wiki_stats` replaces the multi-call orientation pattern
- Lint: orphan count and staleness overlap with lint checks
- Facets: stats reuses the facet collection code

## Open questions

- Should staleness buckets be configurable or fixed (7d, 30d)?
- Should `wiki_stats` include tag distribution (top N tags)?
- Should there be a `--verbose` flag for additional metrics?

## Tasks

- [ ] Spec: `docs/specifications/tools/stats.md`
- [ ] `src/ops/stats.rs` — compose metrics from existing queries
- [ ] `src/mcp/tools.rs` — add `wiki_stats` tool
- [ ] `src/mcp/handlers.rs` — handler
- [ ] `src/cli.rs` — `Stats` command
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
