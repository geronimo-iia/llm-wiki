---
title: "petgraph-live Integration Guide"
summary: "How petgraph-live is used in llm-wiki — GenerationCache, GraphState, SnapshotConfig, and known pitfalls."
status: ready
last_updated: "2026-05-03"
read_when:
  - Working on graph caching or warm-start (engine.rs, graph.rs)
  - Debugging snapshot init failures
  - Extending WikiGraphCache with new variants
  - Evaluating Phase 3 (algorithm suite) integration
---

# petgraph-live Integration Guide

## What petgraph-live provides

Two independent primitives:

| Type | Purpose |
|------|---------|
| `GenerationCache<T>` | In-memory cache keyed on a `u64` generation counter |
| `GraphState<T>` | Generation cache + snapshot persistence (load from disk on key match) |

Both return `Arc<T>`. Neither requires a lock wrapper — they handle internal
synchronisation themselves.

## `WikiGraphCache` — the wrapper enum

```rust
// src/graph.rs
pub enum WikiGraphCache {
    NoSnapshot(GenerationCache<WikiGraph>),
    WithSnapshot(GraphState<WikiGraph>),
}
```

`NoSnapshot` — Phase 1 behaviour. Cache lives only for process lifetime.
Constructed when `graph.snapshot = false` (CI, tests).

`WithSnapshot` — Phase 2 behaviour. On process restart, attempts to load
from disk snapshot. Cold build only on key mismatch or missing/corrupt file.
Constructed when `graph.snapshot = true` (default).

Construction happens in `build_wiki_graph_cache` inside `engine.rs::mount_space`.

## `GenerationCache<T>` usage

```rust
let cache: GenerationCache<WikiGraph> = GenerationCache::new();

// On every request:
let current_gen = index_manager.generation();
let graph: Arc<WikiGraph> = cache.get_or_build(current_gen, || {
    build_graph(&searcher, &schema, &filter, &registry)
})?;

// Force rebuild (e.g. after explicit invalidation):
cache.invalidate();
let graph = cache.get_or_build(current_gen, builder)?;
```

`get_or_build` is synchronous. The closure is called only on miss.
The generation counter (`u64`) comes from `SpaceIndexManager::generation()` —
incremented on every successful `reload_reader()` call (after any index write).

## `GraphState<T>` — construction

```rust
use petgraph_live::live::{GraphState, GraphStateConfig};
use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat};

let snap_cfg = SnapshotConfig {
    dir:         state_dir.join("snapshots").join(wiki_name),
    name:        "wiki-graph".into(),
    key:         None,              // MUST be None — GraphState manages keys internally
    format:      SnapshotFormat::Bincode,
    compression: Compression::Lz4,
    keep:        3,                 // rotate: keep N most recent snapshots
};

let state: GraphState<WikiGraph> = GraphState::builder(GraphStateConfig::new(snap_cfg))
    .key_fn(move || Ok(index_manager.generation().to_string()))
    .build_fn(move || build_graph_from_disk(...))
    .init()
    .map_err(|e| anyhow::anyhow!("graph snapshot init failed: {e}"))?;
```

### Critical: `SnapshotConfig.key` must be `None`

`GraphState` manages the snapshot key internally via `key_fn`. If you set
`key` to `Some(...)`, it conflicts with the key returned by `key_fn` and
produces incorrect behaviour. Always `None`.

### Snapshot directory

`GraphState::init()` creates the snapshot directory automatically (petgraph-live ≥ 0.3.1).

## `GraphState<T>` — usage

```rust
// Hot path: returns cached graph if key matches, cold-builds otherwise
let graph: Arc<WikiGraph> = state.get_fresh()
    .map_err(|e| anyhow::anyhow!("{e}"))?;

// Force rebuild regardless of key (e.g. after wiki_index_rebuild):
let graph: Arc<WikiGraph> = state.rebuild()
    .map_err(|e| anyhow::anyhow!("{e}"))?;
```

`get_fresh()` takes no arguments — `key_fn` is called internally on each
invocation to check staleness. If the key matches the loaded snapshot, returns
the cached `Arc<WikiGraph>` immediately (no disk I/O on the hot path).

`rebuild()` always calls `build_fn`, saves a new snapshot, and returns the
fresh graph.

## `'static` closure constraints

Both `key_fn` and `build_fn` must be `Fn() + Send + Sync + 'static`.
This means:

- No captures of `RwLock` guards (they hold borrows — E0521)
- No captures of `&SpaceTypeRegistry` (not Clone — cannot be moved)
- Use `Arc<T>` for shared ownership: `Arc<SpaceIndexManager>` and `Arc<SpaceTypeRegistry>` clone cheaply
- `IndexSchema` derives `Clone` — captured by value at construction time

Pattern for `build_fn` in llm-wiki (current — no disk re-derivation):

```rust
let im = Arc::clone(&index_manager);  // Arc<SpaceIndexManager>
let is = index_schema.clone();         // IndexSchema: Clone
let tr = Arc::clone(&type_registry);   // Arc<SpaceTypeRegistry>: Clone

let build_fn = move || {
    let searcher = im.searcher()
        .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?;
    build_graph(&searcher, &is, &GraphFilter::default(), &*tr)
        .map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))
};
```

`IndexSchema` derives `Clone` — captured at `GraphState` construction.
`SpaceTypeRegistry` is `Arc`-wrapped in `SpaceContext` — `Arc::clone` is free.
No disk I/O on cold build beyond the graph construction itself.

## Error type mapping

petgraph-live closures return `Result<T, SnapshotError>`. llm-wiki uses
`anyhow::Result` everywhere. Map at the boundary:

```rust
// In build_fn / key_fn closures (must return SnapshotError):
some_op().map_err(|e| SnapshotError::Io(std::io::Error::other(e.to_string())))?

// After init() / get_fresh() / rebuild() (convert to anyhow):
state.get_fresh().map_err(|e| anyhow::anyhow!("{e}"))
```

## Feature flags

```toml
# Cargo.toml
petgraph-live = { version = "0.3", features = ["snapshot-lz4", "snapshot-zstd"] }
```

`snapshot-lz4` enables `Compression::Lz4`. `snapshot-zstd` enables `Compression::Zstd { level }`.
Both features are enabled.

Available `Compression` variants with current feature set:
- `Compression::None`
- `Compression::Lz4`
- `Compression::Zstd { level }`

## `WikiGraph` serde requirement

`GraphState<WikiGraph>` requires `WikiGraph: Serialize + DeserializeOwned`.
`WikiGraph` is `DiGraph<PageNode, LabeledEdge>`.

- `PageNode` — derives `Serialize, Deserialize` (Phase 1)
- `LabeledEdge` — derives `Serialize, Deserialize` (Phase 1)
- `petgraph::DiGraph` — implements serde when node/edge types do

Do not remove these derives from `PageNode` or `LabeledEdge`.

## Config

Three `GraphConfig` fields control snapshot behaviour:

| Field | Default | Effect |
|-------|---------|--------|
| `snapshot` | `true` | `false` → `NoSnapshot` variant (in-memory only) |
| `snapshot_keep` | `3` | Passed to `SnapshotConfig::keep` |
| `snapshot_format` | `"bincode+lz4"` | Maps: `"bincode+lz4"` → `Compression::Lz4`; `"bincode+zstd"` → `Compression::Zstd { level: 3 }`; any other → `Compression::None` |

In integration tests, set `graph.snapshot = false` to prevent snapshot files
from appearing in `tempfile::TempDir` directories.

## Snapshot file location

```
~/.llm-wiki/snapshots/<wiki-name>/wiki-graph-<generation-key>.bin.lz4
```

`state_dir` is `EngineState.state_dir` — typically `~/.llm-wiki`. Never
hardcode this path.

Old snapshots are rotated by petgraph-live when `keep` is exceeded. No manual
cleanup needed.
