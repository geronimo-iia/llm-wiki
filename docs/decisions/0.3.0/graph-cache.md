---
title: "Graph Cache v0.3.0"
summary: "Design decisions for in-memory WikiGraph cache: keying strategy, invalidation, community map co-location, filtered bypass."
date: "2026-05-01"
---

# Graph Cache v0.3.0

## Decision

Cache the full unfiltered `WikiGraph` per wiki space in `SpaceContext.graph_cache: RwLock<Option<CachedGraph>>`. Key the cache on `SpaceIndexManager.generation()` — a monotonic counter incremented on every index write. Filtered requests (type, relation, root, depth) bypass the cache and build on demand. The community map (`HashMap<slug, community_id>`) lives inside `CachedGraph` alongside the graph, built once and reused by `wiki_suggest`.

## Context

`wiki_graph`, `wiki_stats`, and `wiki_suggest` all needed the full petgraph. Before this change, each call rebuilt the graph from the tantivy index: O(pages + edges), no reuse. On a 500-page wiki this was measurable latency on every MCP call. The ACP graph workflow made this more visible — streaming graph output to the IDE after a fresh ingest triggered two sequential full builds (graph + stats).

## Decisions and Rationale

### Generation counter as cache key

**Decision:** `CachedGraph.generation: u64` stores the `SpaceIndexManager.generation()` value at build time. On each cache read, the current generation is compared; mismatch → evict and rebuild.

**Rationale:** The generation counter is already incremented by `reload_reader` after every index write (ingest, commit). It is cheap to read (`AtomicU64::load(Relaxed)`) and precisely tracks index mutations. Alternatives considered:

- **File mtime of tantivy segment files:** Requires filesystem calls per request; unreliable across NFS/Docker volume mounts.
- **Content hash of index:** Expensive — requires reading all stored fields.
- **Timestamp:** Clock skew on rapid writes within same millisecond causes stale cache reads.

The generation counter is the right primitive: zero-cost invalidation check, exact semantics.

### Filtered requests bypass cache

**Decision:** Requests with non-default `GraphParams` (type_filter, relation, root, depth) skip the cache and call `build_graph` directly.

**Rationale:** The cache stores the full unfiltered graph. Filtered views are derived from it at render time — they are not stored separately. Caching every (filter combination × wiki) would require an unbounded map with complex eviction. Instead, filtered requests are cheap to rebuild because they call `build_graph` which reads the tantivy index sequentially — no file I/O beyond the index reader already held open.

**Trade-off:** A repeated `wiki_graph(root: concepts/moe, depth: 2)` call rebuilds every time. Acceptable for v0.3.0 — subgraph requests are interactive, not hot-path.

### Community map co-located in CachedGraph

**Decision:** `CachedGraph` holds both `graph: Arc<WikiGraph>` and `community_map: Option<Arc<HashMap<String, usize>>>`. Both are built once when the cache is populated.

**Rationale:** Community detection (`node_community_map`) runs Louvain on the full graph — it cannot run on a filtered subgraph. Since the full graph is already being built for the cache, computing the community map at the same time adds negligible overhead. Storing it in `CachedGraph` means `wiki_suggest` (strategy 4: community peers) reads from cache without a second graph build.

**Alternative rejected:** Build community map lazily on first `wiki_suggest` call. Rejected because it requires a second write lock on `graph_cache` and complicates the cache state machine (graph present but map absent vs. both present).

### `merge_cached_graphs` for cross-wiki builds

**Decision:** `merge_cached_graphs` accepts `&[(&str, Arc<WikiGraph>)]` slices of already-cached graphs instead of rebuilding from index. Used by `wiki_graph(cross_wiki: true)`.

**Rationale:** Cross-wiki graph builds previously called `build_graph_cross_wiki` which re-read all indices. With per-wiki caches available, the merged graph can be assembled from already-built `Arc<WikiGraph>` clones — no index reads. The merged result is not cached (cross-wiki requests are rare; caching would require a separate key per wiki combination).

### Arc wrapping for zero-copy sharing

**Decision:** `CachedGraph.graph: Arc<WikiGraph>` and `community_map: Option<Arc<...>>`. Callers clone the `Arc`, not the graph.

**Rationale:** A 500-page wiki graph is ~100 KB of heap. Cloning it per request would be wasteful. `Arc` gives safe shared ownership across concurrent MCP handlers without copying. The graph is immutable after construction — no interior mutability needed.
