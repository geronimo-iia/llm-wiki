# Study: wiki_stats — wiki health dashboard

A dedicated tool for wiki health metrics. One call, one response,
health only.

## Problem

Understanding wiki health requires multiple tool calls: `wiki_list`
for facets, `wiki_graph` for connectivity, `wiki_index_status` for
index health. No single view shows the overall state.

## Decisions

- **Fixed staleness buckets** — `fresh` (<7d), `stale_7d` (7-30d),
  `stale_30d` (>30d). No config key — these answer "how healthy?"
  not "how precise?".
- **No tag distribution** — stats is about health, not content.
  Tags are a content question handled by facets in search/list.
- **No `--verbose` flag** — one response, all metrics. JSON fields
  are cheap to add later.
- **Composed from existing primitives** — no new index fields.
  Orchestrates `wiki_list`, `wiki_graph`, `wiki_index_status`, and
  a tantivy query on `last_updated`.
- **18th MCP tool** — `wiki_stats`.

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

### Text output

```
research — 42 pages, 3 sections
types:     concept(20) paper(15) article(5) section(3)
status:    active(38) draft(4)
orphans:   3
graph:     2.4 avg connections, 0.12 density
staleness: fresh(30) 7d(8) 30d(4)
index:     ok, built 2025-07-21T14:32:01Z
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

## Interaction with existing features

- **Bootstrap** — `wiki_stats` replaces the multi-call orientation
  pattern. One call gives the full picture.
- **Lint** — orphan count and staleness overlap with lint checks.
  Stats gives the numbers, lint offers fixes.
- **Facets** — stats reuses the facet collection code for type/status.

## Tasks

### 1. Update specifications

- [ ] Create `docs/specifications/tools/stats.md` — CLI, MCP,
  response format, metrics table
- [ ] Update `docs/specifications/tools/overview.md` — add
  `wiki_stats` (18 tools)

### 2. Graph metrics

- [ ] `src/graph.rs` — add `GraphMetrics { nodes, edges, orphans,
  avg_connections, density }` computed from petgraph
- [ ] Expose via a `compute_metrics` function that takes the built
  graph

### 3. Staleness query

- [ ] `src/ops/stats.rs` — query `last_updated` field from tantivy,
  bucket into fresh/stale_7d/stale_30d based on current date

### 4. Stats composition

- [ ] `src/ops/stats.rs` — `WikiStats` struct with all metrics
- [ ] `src/ops/stats.rs` — `stats()` function that composes from
  list facets, graph metrics, staleness query, index status
- [ ] `src/ops/mod.rs` — export stats

### 5. MCP

- [ ] `src/mcp/tools.rs` — add `wiki_stats` tool schema (wiki)
- [ ] `src/mcp/handlers.rs` — `handle_stats` handler

### 6. CLI

- [ ] `src/cli.rs` — add `Stats` command with `--wiki`, `--format`
- [ ] `src/main.rs` — render stats in text and JSON

### 7. Tests

- [ ] Stats returns expected metrics for a wiki with pages
- [ ] Stats orphan count matches graph
- [ ] Stats staleness buckets are correct
- [ ] Stats on empty wiki returns zeros
- [ ] Existing test suite passes unchanged

### 8. Decision record

- [ ] `docs/decisions/wiki-stats.md`

### 9. Update skills

- [ ] `llm-wiki-skills/skills/bootstrap/SKILL.md` — use `wiki_stats`
  instead of multi-call orientation
- [ ] `llm-wiki-skills/skills/lint/SKILL.md` — reference stats for
  orphan count and staleness
- [ ] `llm-wiki-skills/skills/content/SKILL.md` — mention stats for
  wiki overview

### 10. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets -- -D warnings`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md`
- [ ] Remove this prompt

## Success criteria

- `wiki_stats("research")` returns all metrics in one call
- Orphan count matches `wiki_graph` analysis
- Staleness buckets are correct relative to current date
- Text output is human-readable, JSON is machine-parseable
- No new index fields needed
- 18 MCP tools total
