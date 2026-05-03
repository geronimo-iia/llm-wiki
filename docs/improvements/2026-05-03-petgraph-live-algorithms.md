---
title: "petgraph-live Phase 3 — Algorithm suite"
summary: "Expose petgraph-live structural algorithms (articulation points, bridges, diameter, radius, center, periphery) via wiki_stats and a new wiki_health MCP tool."
status: proposed
target_version: "0.4.0"
branch: feat/petgraph-live-algorithms
pr_target: dev/v0.4.0
depends_on:
  - 2026-05-03-petgraph-live-cache.md
last_updated: "2026-05-03"
read_when:
  - Implementing Phase 3 of petgraph-live integration
  - Working on ops/stats.rs, ops/health.rs, mcp/tools.rs, mcp/handlers.rs
  - Designing the wiki_health MCP tool
---

# petgraph-live Phase 3 — Algorithm suite

## Problem

`wiki_stats` reports basic metrics: orphan count, density, average connections, community count. Structural health questions — "which pages are critical connectors?", "which links are load-bearing?", "how far apart are the most isolated concepts?" — are unanswerable today.

`petgraph-live` default features include `connect` and `metrics` modules that answer exactly these questions on any `DiGraph`.

## Solution

Add `ops/health.rs` with a `wiki_health` function. Register a `wiki_health` MCP tool. Extend `WikiStats` with optional structural fields populated from the same algorithms.

Phase 3 is **independent of Phase 2** (snapshot). It requires only Phase 1 (`GenerationCache<WikiGraph>` in `SpaceContext`).

## Algorithms

All operate on `Arc<WikiGraph>` from `space.graph_cache.get_or_build(...)`.

| Field | Module | Call | Meaning |
|-------|--------|------|---------|
| `articulation_points` | `connect` | `articulation_points(&g)` | Slugs whose removal disconnects the graph |
| `bridges` | `connect` | `find_bridges(&g)` | Edges whose removal disconnects the graph |
| `diameter` | `metrics` | `diameter(&g)` | Maximum shortest-path length (O(n²)) |
| `radius` | `metrics` | `radius(&g)` | Minimum eccentricity |
| `center` | `metrics` | `center(&g)` | Nodes with eccentricity = radius (hub pages) |
| `periphery` | `metrics` | `periphery(&g)` | Nodes with eccentricity = diameter (isolated pages) |

`articulation_points` and `find_bridges` are O(n+e) — always computed.

`diameter`, `radius`, `center`, `periphery` are O(n²) — skipped for large graphs. Threshold configurable via `graph.max_nodes_for_diameter` (default 2000). When skipped, fields return `null` with a `note` field explaining why.

## `wiki_health` MCP tool

New tool: `wiki_health`. Input: `wiki` (optional). Output:

```json
{
  "wiki": "my-wiki",
  "node_count": 312,
  "articulation_points": ["slug-a", "slug-b"],
  "bridges": [
    { "from": "slug-a", "to": "slug-b" },
    { "from": "slug-c", "to": "slug-d" }
  ],
  "diameter": 12,
  "radius": 4,
  "center": ["slug-hub"],
  "periphery": ["slug-isolated"],
  "note": null
}
```

When `diameter` skipped:

```json
{
  "diameter": null,
  "radius": null,
  "center": null,
  "periphery": null,
  "note": "graph too large for diameter computation (312 nodes > max_nodes_for_diameter=2000 threshold — set graph.max_nodes_for_diameter to override)"
}
```

## `wiki_stats` additions

`WikiStats` gains optional structural fields — populated only when `include_health: bool` is passed (default `false`, avoids O(n²) cost on every `wiki_stats` call):

```rust
pub struct WikiStats {
    // ... existing fields ...
    pub articulation_points: Option<Vec<String>>,
    pub bridges:             Option<Vec<(String, String)>>,
    pub diameter:            Option<u32>,
    pub radius:              Option<u32>,
    pub center:              Option<Vec<String>>,
    pub periphery:           Option<Vec<String>>,
    pub health_note:         Option<String>,
}
```

MCP tool `wiki_stats` gains optional `include_health` boolean parameter.

## Config addition

New field in `GraphConfig`:

```toml
[graph]
max_nodes_for_diameter = 2000   # skip O(n²) algorithms above this threshold
```

New arm in `set_global_config_value` / `get_config_value`: `graph.max_nodes_for_diameter`.

## `ops/health.rs`

New module. Single public function:

```rust
pub fn wiki_health(
    engine:    &EngineState,
    wiki_name: &str,
) -> Result<WikiHealthReport>
```

Acquires `Arc<WikiGraph>` from `space.graph_cache`, runs algorithms, applies size threshold, returns `WikiHealthReport`.

`WikiHealthReport` is the typed backing for the JSON above.

## Affected files

| File | Change |
|------|--------|
| `src/ops/health.rs` | New — `wiki_health` function + `WikiHealthReport` struct |
| `src/ops/mod.rs` | Add `pub mod health` |
| `src/ops/stats.rs` | Add optional structural fields to `WikiStats`; call `ops/health` when `include_health` |
| `src/mcp/tools.rs` | Register `wiki_health` tool; add `include_health` param to `wiki_stats` |
| `src/mcp/handlers.rs` | Add `wiki_health` handler; update `wiki_stats` handler |
| `src/config.rs` | Add `graph.max_nodes_for_diameter` to `GraphConfig`; add match arms |
| `CHANGELOG.md` | Add `[Unreleased]` entry |

## Breaking changes

None. `wiki_stats` JSON gains new nullable fields — additive only. `wiki_health` is a new tool.

## Out of scope

| Item | Reason |
|------|--------|
| `wiki_suggest` articulation reranking | Too speculative; boost-on-articulation-point is a Phase 4+ idea |
| Community detection via petgraph-live | Louvain is intentionally out of scope for petgraph-live; keep existing implementation |
| Async algorithm execution | Not needed; algorithms run in <1s for wikis below threshold |

## Constraints

- `gen` is reserved in Rust 2024 — never use as variable name
- `anyhow::Result` throughout
- `graph.max_nodes_for_diameter` check must apply to **local** node count (non-external), same as `min_nodes_for_communities`
- `wiki_health` must return a well-formed JSON response even when all O(n²) fields are null

## Validation

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --doc
# Manual: wiki_health on a real wiki returns non-empty articulation_points
# Manual: wiki_stats with include_health=true returns same data
```

## Definition of done

- [ ] `ops/health.rs` with `wiki_health` function
- [ ] `wiki_health` MCP tool registered and returns correct JSON
- [ ] `wiki_stats` accepts `include_health` and populates structural fields when true
- [ ] `graph.max_nodes_for_diameter` config key wired up
- [ ] `note` field populated correctly when O(n²) skipped
- [ ] All tests pass, clippy clean, fmt clean, doc tests pass
- [ ] `CHANGELOG.md` `[Unreleased]` updated

## See also

- [Phase 1 — GenerationCache](2026-05-03-petgraph-live-cache.md) (required prerequisite)
- [Phase 2 — Snapshot warm-start](2026-05-03-petgraph-live-snapshot.md) (independent)
