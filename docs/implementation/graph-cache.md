---
title: "Graph Cache Implementation"
summary: "In-memory graph cache keyed on index generation — eliminates redundant build_graph and Louvain calls in serve mode."
status: ready
last_updated: "2026-05-03"
depends_on:
  - engine.md
  - index-manager.md
---

# Graph Cache Implementation

Implementation reference for the in-memory graph cache introduced in v0.3.0.
Not a specification — see [graph.md](../specifications/engine/graph.md) for
the design contract.

## Problem

`build_graph` scans the entire tantivy index on every call. In serve mode
(`wiki_graph`, `wiki_stats`, `wiki_suggest`), the same full graph is rebuilt
on every request even when nothing has changed. Louvain community detection
compounds the cost — it ran on every `wiki_stats` and `wiki_suggest` call.

## Core structs

```rust
// src/graph.rs

pub struct CommunityData {
    pub local_count: usize,
    pub map:   Arc<HashMap<String, usize>>,
    pub stats: CommunityStats,
}
```

`CommunityData` replaces `CachedGraph.community_map` + `CachedGraph.community_stats`.
`local_count` stores local node count at build time — avoids re-traversal on the hot path.
Community and graph caches share the same generation key.

Both caches live in `SpaceContext`:

```rust
// src/engine.rs
pub graph_cache:     WikiGraphCache,
pub community_cache: GenerationCache<CommunityData>,
```

`GenerationCache<T>` is `Send + Sync` — no `RwLock` wrapper needed.
`GenerationCache::get_or_build(gen, builder)` returns `Arc<T>` on hit,
calls `builder()` → `Result<T>` on miss, caches and returns `Arc<T>`.

## Cache key: `AtomicU64` generation counter

`IndexInner` in `SpaceIndexManager` holds a `generation: AtomicU64` starting
at 0. Every successful `reload_reader()` call does:

```rust
inner.generation.fetch_add(1, Ordering::AcqRel);
```

`reload_reader()` is called at the end of every write path:
- `rebuild()` — full index rebuild
- `update()` — incremental update from git diff
- `delete_by_type()` — type-targeted delete
- `rebuild_types()` — partial type rebuild
- watch-mode ingest

Exposed as:

```rust
pub fn generation(&self) -> u64 {
    self.inner.read().unwrap().generation.load(Ordering::Acquire)
}
```

No explicit cache flush is ever needed. Any index write automatically
invalidates the cache on the next graph request.

## Public accessors

All live in `src/graph.rs`. Callers pass individual fields rather than
`&SpaceContext` to avoid a circular dependency between `graph.rs` and
`engine.rs`.

### `get_or_build_graph`

```rust
pub fn get_or_build_graph(
    index_schema:  &IndexSchema,
    type_registry: &SpaceTypeRegistry,
    index_manager: &SpaceIndexManager,
    graph_cache:   &WikiGraphCache,
    searcher:      &Searcher,
    filter:        &GraphFilter,
) -> Result<Arc<WikiGraph>>
```

- Filtered requests (`!filter.is_default()`) bypass cache, build and return fresh.
- Cache hit: generation matches → return `Arc::clone`.
- Cache miss: call `build_graph` inside `get_or_build`, cache result, return.

### `get_cached_community_map`

```rust
pub fn get_cached_community_map(
    ...,
    graph_cache:     &WikiGraphCache,
    community_cache: &GenerationCache<CommunityData>,
    searcher:        &Searcher,
    min_nodes:       usize,
) -> Result<Option<Arc<HashMap<String, usize>>>>
```

Uses nested closure pattern: `community_cache.get_or_build` wraps `graph_cache.get_or_build` inside.
Hot path (both warm): community_cache hits immediately — graph_cache never touched.
Returns `None` when `community.local_count < min_nodes`.

### `get_cached_community_stats`

```rust
pub fn get_cached_community_stats(
    ...,
    graph_cache:     &WikiGraphCache,
    community_cache: &GenerationCache<CommunityData>,
    searcher:        &Searcher,
    min_nodes:       usize,
) -> Result<Option<CommunityStats>>
```

Same nested closure pattern. Returns `None` when `community.local_count < min_nodes`.

## Cache population

On cache miss, calls `build_community_data(graph, 0)` — a private helper that
runs Louvain exactly once and returns both `CommunityStats` and
`HashMap<String, usize>` (community map). Both fields are stored in
`CachedGraph` atomically. Passing `0` ensures Louvain always runs regardless
of graph size; the caller-supplied `min_nodes` gate is applied at read time,
not at build time.

`compute_communities` and `node_community_map` are thin wrappers over
`build_community_data` — they extract `.0` and `.1` respectively. This ensures
all three entry points share identical Louvain logic with no duplication.

## `GraphFilter::is_default()`

```rust
pub fn is_default(&self) -> bool {
    self.root.is_none() && self.types.is_empty() && self.relation.is_none()
}
```

`depth` is intentionally excluded — a depth-limited request still uses the full
cached graph; the caller extracts a subgraph via BFS post-cache.

## Callers

| Caller | Uses cache via |
|--------|----------------|
| `ops/graph.rs` — single-wiki path | `get_or_build_graph` |
| `ops/stats.rs` | `get_or_build_graph` + `get_cached_community_stats` |
| `ops/suggest.rs` | `get_or_build_graph` + `get_cached_community_map` |
| `ops/graph.rs` — cross-wiki path | `get_or_build_graph` per space + `merge_cached_graphs` |

## Cross-wiki caching

`build_graph_cross_wiki` takes raw `(searcher, schema, registry)` tuples and
calls `build_graph` directly — it cannot use the per-space cache.

The cross-wiki path in `ops/graph.rs` works around this by pre-building each
per-space graph via `get_or_build_graph` (cache-aware), then passing the
resulting `Arc<WikiGraph>` slices to `merge_cached_graphs`:

```rust
pub fn merge_cached_graphs(
    wikis: &[(&str, Arc<WikiGraph>)],
    filter: &GraphFilter,
) -> Result<WikiGraph>
```

`merge_cached_graphs` has the same semantics as `build_graph_cross_wiki` but
accepts pre-built graphs rather than raw index handles.

## Limitations

- Only unfiltered full graphs are cached. Each distinct filter combination builds fresh.
- Community data is always computed at threshold 0. `min_nodes` is applied at read time — no recompute for different thresholds.

## Snapshot warm-start (v0.4.0 Phase 2)

`SpaceContext.graph_cache` is now a `WikiGraphCache` enum:

```rust
pub enum WikiGraphCache {
    NoSnapshot(GenerationCache<WikiGraph>),
    WithSnapshot(GraphState<WikiGraph>),
}
```

`WithSnapshot` is constructed when `graph.snapshot = true` (default). On process restart:
- `GraphState::init()` compares the current generation key (from `index_manager.generation().to_string()`) against the snapshot filename.
- Match → load from disk, skip cold build.
- Miss → cold build, save snapshot, return graph.

After `wiki_index_rebuild`, `WikiGraphCache::rebuild()` forces a new snapshot for the updated generation key.

`graph.snapshot = false` constructs `NoSnapshot` — identical to Phase 1 behaviour. Use in CI and integration tests to avoid snapshot files in tempdir.

Snapshots stored at: `state_dir/snapshots/<wiki-name>/wiki-graph-<key>.<ext>`

## Structural algorithms (v0.4.0 Phase 3)

`run_lint` (new `articulation-point`, `bridge`, `periphery` rules) and `stats`
(new `diameter`/`radius`/`center` fields) both acquire `Arc<WikiGraph>` via
`get_or_build_graph` — same cache path as `wiki_suggest`. No extra cache entry.

`connect` algorithms require an undirected view. `build_undirected` in `lint.rs`
symmetrizes `WikiGraph` into `UnGraph<NodeIndex, ()>`, excluding external nodes.

`metrics` algorithms use the directed `WikiGraph` directly.
