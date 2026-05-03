---
title: "petgraph-live Phase 1 — GenerationCache"
summary: "Replace bespoke CachedGraph + RwLock<Option<CachedGraph>> with petgraph_live::cache::GenerationCache<WikiGraph> and a separate GenerationCache<CommunityData>. Zero behaviour change."
status: in-progress
target_version: "0.4.0"
branch: feat/petgraph-live-cache
pr_target: dev/v0.4.0
depends_on: []
last_updated: "2026-05-03"
read_when:
  - Implementing Phase 1 of petgraph-live integration
  - Reviewing SpaceContext graph cache design
  - Working on graph.rs, engine.rs, ops/stats.rs, ops/suggest.rs, ops/graph.rs
---

# petgraph-live Phase 1 — GenerationCache

## Problem

`SpaceContext` carries a bespoke generation-keyed graph cache:

```rust
pub graph_cache: RwLock<Option<CachedGraph>>,

pub struct CachedGraph {
    pub graph:           Arc<WikiGraph>,
    pub community_map:   Option<Arc<HashMap<String, usize>>>,
    pub community_stats: Option<CommunityStats>,
    pub index_gen:       u64,
}
```

`get_or_build_graph()` in `src/graph.rs` manually manages read/write locks, compares `index_gen`, rebuilds on miss. Community data is co-located in `CachedGraph` — graph and community share one lock, one invalidation event.

Pain points:
- Manual lock/invalidation logic not covered beyond integration smoke tests
- Community threshold check requires re-traversing the graph on every hot call
- No warm-start (Phase 2 concern, but Phase 1 must lay the foundation)
- `CachedGraph` is a private bespoke type — `petgraph-live` solves this generically

## Solution

Replace with `petgraph_live::cache::GenerationCache<T>` — a production-quality generation-keyed cache that is `Send + Sync` with no external `RwLock` wrapper.

Two caches on `SpaceContext`:

```rust
pub graph_cache:     GenerationCache<WikiGraph>,
pub community_cache: GenerationCache<CommunityData>,
```

`CommunityData` replaces the community fields of `CachedGraph` and stores `local_count` to avoid re-traversal on the hot path:

```rust
pub struct CommunityData {
    pub local_count: usize,                        // local node count at build time
    pub map:         Arc<HashMap<String, usize>>,  // slug → community id
    pub stats:       CommunityStats,
}
```

## Hot-path design

`get_cached_community_map` and `get_cached_community_stats` use a **nested closure pattern**:

```rust
let community = community_cache.get_or_build(current_gen, || {
    let graph = graph_cache.get_or_build(current_gen, || {
        build_graph(searcher, index_schema, &GraphFilter::default(), type_registry)
    })?;
    let local_count = graph.node_indices().filter(|&i| !graph[i].external).count();
    let (stats_opt, map_opt) = build_community_data(&graph, 0);
    Ok(CommunityData { local_count, map: Arc::new(map_opt.unwrap_or_default()), stats: ... })
})?;
```

On cache hit: `community_cache.get_or_build` returns immediately after one read lock — `graph_cache` never touched. This avoids 3 atomic ops + 1 lock acquisition vs. the naive "fetch graph first, then community" approach.

`community.local_count` makes the `min_nodes` threshold check a field read — no graph traversal.

## Import path

`GenerationCache` lives at `petgraph_live::cache::GenerationCache`, not `petgraph_live::GenerationCache`.

```rust
use petgraph_live::cache::GenerationCache;
```

## `LabeledEdge` serde

Phase 2 snapshot requires `LabeledEdge: Serialize + Deserialize`. Add derives now to keep Phase 1 and Phase 2 diffs clean:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledEdge { pub relation: String }
```

## Cargo.toml

```toml
petgraph-live = "0.3"   # default features only — no snapshot feature needed for Phase 1
```

No diamond conflict: `petgraph-live` depends on `petgraph = "0.8"`, same as llm-wiki.

## Affected files

| File | Change |
|------|--------|
| `Cargo.toml` | Add `petgraph-live = "0.3"`; bump version `0.3.0` → `0.4.0` |
| `src/graph.rs` | Add `Serialize + Deserialize` to `LabeledEdge`; add `CommunityData`; rewrite `get_or_build_graph`, `get_cached_community_map`, `get_cached_community_stats`; delete `CachedGraph` |
| `src/engine.rs` | Replace `RwLock<Option<CachedGraph>>` with two `GenerationCache` fields; keep `RwLock` import (`WikiEngine.state` still uses it) |
| `src/ops/graph.rs` | Call sites compile automatically — `graph_cache` type change is transparent |
| `src/ops/stats.rs` | Add `&space.community_cache` parameter to `get_cached_community_stats` call |
| `src/ops/suggest.rs` | Add `&space.community_cache` parameter to `get_cached_community_map` call |
| `tests/graph_cache.rs` | Add `&space.community_cache` to community function calls; `get_or_build_graph` calls unchanged |
| `docs/implementation/graph-cache.md` | Update core structs and accessor signatures |
| `CHANGELOG.md` | Add `[Unreleased]` entry |

## Breaking changes

None public.

## Constraints

- `gen` is reserved in Rust 2024 — never use as variable name
- `anyhow::Result` throughout — `GenerationCache::get_or_build` builder returns `Result<T, E>` where `E: From<anyhow::Error>`; passes cleanly with `?`
- Keep `use std::sync::RwLock` in `engine.rs` — `WikiEngine.state: Arc<RwLock<EngineState>>` still needs it
- `docs/roadmap.md` already has v0.4.0 section — update in place, do not duplicate

## Validation

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --doc
```

## Definition of done

- [ ] `CachedGraph` struct deleted
- [ ] `SpaceContext` has `graph_cache: GenerationCache<WikiGraph>` and `community_cache: GenerationCache<CommunityData>`, no `RwLock` wrapper
- [ ] `get_or_build_graph` uses `GenerationCache::get_or_build`
- [ ] Community accessors use nested closure pattern
- [ ] `graph_cache_hit_returns_same_arc` test passes — `Arc::ptr_eq` on generation hit
- [ ] All tests pass, clippy clean, fmt clean, doc tests pass
- [ ] `CHANGELOG.md` `[Unreleased]` updated

## See also

- [Phase 2 — Snapshot warm-start](2026-05-03-petgraph-live-snapshot.md)
- [Phase 3 — Algorithm suite](2026-05-03-petgraph-live-algorithms.md)
- Implementation plan: `docs/brainstorm/plan/2026-05-02-petgraph-live-phase1.md`
