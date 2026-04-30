---
title: "Graph Cache Implementation"
summary: "In-memory graph cache keyed on index generation — eliminates redundant build_graph and Louvain calls in serve mode."
status: ready
last_updated: "2026-04-30"
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

pub struct CachedGraph {
    pub graph:            Arc<WikiGraph>,
    pub community_map:    Option<Arc<HashMap<String, usize>>>,
    pub community_stats:  Option<CommunityStats>,
    pub index_gen:        u64,
}
```

`community_map` — slug→community_id, used by `wiki_suggest` strategy 4.
`community_stats` — aggregated stats (`count`, `isolated` list), used by `wiki_stats`.
`index_gen` — generation value at cache time; compared against current generation to detect staleness.

`CachedGraph` lives in `SpaceContext`:

```rust
// src/engine.rs
pub graph_cache: RwLock<Option<CachedGraph>>,
```

Multiple readers can use the cached graph simultaneously. A single writer
rebuilds and stores on miss.

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
    graph_cache:   &RwLock<Option<CachedGraph>>,
    searcher:      &Searcher,
    filter:        &GraphFilter,
) -> Result<Arc<WikiGraph>>
```

- Filtered requests (`!filter.is_default()`) bypass cache, build and return fresh.
- Cache hit: `cached.index_gen == current_gen` → return `Arc::clone`.
- Cache miss: build graph, compute `community_map` + `community_stats`, store, return.

### `get_cached_community_map`

```rust
pub fn get_cached_community_map(
    ...,
    min_nodes: usize,
) -> Result<Option<Arc<HashMap<String, usize>>>>
```

Returns cached `community_map`. If `min_nodes > 30` (the cache threshold),
recomputes without overwriting cache. Triggers graph build as side effect on
miss.

### `get_cached_community_stats`

```rust
pub fn get_cached_community_stats(
    ...,
    min_nodes: usize,
) -> Result<Option<CommunityStats>>
```

Same pattern as `get_cached_community_map` but returns `CommunityStats`. Used
by `ops/stats.rs` to skip Louvain on cache hit.

## Cache population

On cache miss, calls `build_community_data(graph, 30)` — a private helper that
runs Louvain exactly once and returns both `CommunityStats` and
`HashMap<String, usize>` (community map). Both fields are stored in
`CachedGraph` atomically.

`compute_communities` and `node_community_map` are thin wrappers over
`build_community_data` — they extract `.0` and `.1` respectively. This ensures
all three entry points share identical Louvain logic with no duplication.

The community threshold used is `30` (the default config value). Callers
requesting a higher threshold get a fresh recompute without overwriting the
cache. This keeps the cache a single stable entry per space.

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

- Cache lives only for process lifetime. Cold start always rebuilds.
  See [imp-graph-snapshot.md](../improvements/imp-graph-snapshot.md) for
  the planned persistent snapshot feature.
- Community data is cached at `min_nodes = 30`. Wikis configured with a
  different threshold recompute communities on every call (but reuse the
  cached graph structure).
- Only unfiltered full graphs are cached. Each distinct filter combination
  builds fresh.
