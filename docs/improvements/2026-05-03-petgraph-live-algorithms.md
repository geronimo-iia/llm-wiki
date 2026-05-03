---
title: "petgraph-live Phase 3 — Algorithm suite"
summary: "Expose petgraph-live structural algorithms via wiki_lint (3 new rules) and wiki_stats (3 new fields). No new MCP tool."
status: proposed
target_version: "0.4.0"
branch: feat/petgraph-live-algorithms
pr_target: dev/v0.4.0
depends_on:
  - 2026-05-03-petgraph-live-cache.md
last_updated: "2026-05-03"
read_when:
  - Implementing Phase 3 of petgraph-live integration
  - Working on ops/lint.rs, ops/stats.rs, mcp/tools.rs
---

# petgraph-live Phase 3 — Algorithm suite

## Problem

`wiki_stats` reports basic metrics: orphan count, density, average connections, community count. Structural health questions — "which pages are critical connectors?", "which links are load-bearing?", "how far apart are the most isolated concepts?" — are unanswerable today.

`petgraph-live` default features include `connect` and `metrics` modules that answer exactly these questions on any `DiGraph`.

## Solution

Expose algorithms via existing tools — no new MCP tool needed:

- **`wiki_lint`**: 3 new rules (`articulation-point`, `bridge`, `periphery`) — per-page/per-edge findings with fix guidance
- **`wiki_stats`**: 3 new aggregate fields (`diameter`, `radius`, `center`) — topology summary

Phase 3 is **independent of Phase 2** (snapshot). Requires only Phase 1.

## Algorithms

| Algorithm | Module | Complexity | Exposed via |
|-----------|--------|------------|-------------|
| `articulation_points` | `connect` | O(n+e) | `wiki_lint` rule |
| `find_bridges` | `connect` | O(n+e) | `wiki_lint` rule |
| `periphery` | `metrics` | O(n²) | `wiki_lint` rule (skipped above threshold) |
| `diameter` | `metrics` | O(n²) | `wiki_stats` field (skipped above threshold) |
| `radius` | `metrics` | O(n²) | `wiki_stats` field (skipped above threshold) |
| `center` | `metrics` | O(n²) | `wiki_stats` field (skipped above threshold) |

`articulation_points` and `find_bridges` require an undirected view — `build_undirected`
symmetrizes `WikiGraph` into `UnGraph`, excluding external nodes.

`metrics` algorithms operate on directed `WikiGraph` directly.

## Config additions

```toml
[graph]
structural_algorithms  = true   # enable diameter/radius/center in wiki_stats (default: true)
max_nodes_for_diameter = 2000   # skip O(n²) algorithms above this local node count
```

`structural_algorithms` gates `wiki_stats` only. Lint rules `articulation-point`,
`bridge`, `periphery` are controlled by `--rules` — no config flag.

## Affected files

| File | Change |
|------|--------|
| `src/ops/lint.rs` | 3 new rules + `build_undirected` helper |
| `src/ops/stats.rs` | Add `diameter`, `radius`, `center`, `structural_note` fields |
| `src/config.rs` | Add `graph.structural_algorithms` and `graph.max_nodes_for_diameter` to `GraphConfig`; add match arms |
| `src/cli.rs` | Update `--rules` doc comment |
| `src/mcp/tools.rs` | Update `wiki_lint` rules description; update `wiki_stats` description |
| `CHANGELOG.md` | Add `[Unreleased]` entry |

## Breaking changes

None. `wiki_stats` JSON gains new nullable fields — additive only. `wiki_lint` rules are additive (included in default all-rules set).

## Out of scope

| Item | Reason |
|------|--------|
| `wiki_suggest` articulation reranking | Too speculative — Phase 4+ |
| Community detection via petgraph-live | Louvain intentionally out of scope for petgraph-live |
| Async algorithm execution | Not needed; all finish in <1s below threshold |

## Constraints

- `gen` reserved in Rust 2024 — never use as variable name
- `anyhow::Result` throughout
- `max_nodes_for_diameter` check uses **local** node count (non-external), same as `min_nodes_for_communities`

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
- Phase 2 — Snapshot warm-start ✓ implemented (v0.4.0)
