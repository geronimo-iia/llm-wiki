---
title: "petgraph-live Phase 2 — Snapshot warm-start"
summary: "Replace GenerationCache<WikiGraph> with GraphState<WikiGraph> so the graph survives process restarts. Cold builds only on first launch or after wiki_index_rebuild."
status: proposed
target_version: "0.4.0"
branch: feat/petgraph-live-snapshot
pr_target: dev/v0.4.0
depends_on:
  - 2026-05-03-petgraph-live-cache.md
last_updated: "2026-05-03"
read_when:
  - Implementing Phase 2 of petgraph-live integration
  - Working on engine.rs, space_builder.rs, mcp/handlers.rs, config.rs
  - Reviewing graph snapshot lifecycle
---

# petgraph-live Phase 2 — Snapshot warm-start

## Problem

Phase 1 replaces `RwLock<Option<CachedGraph>>` with `GenerationCache<WikiGraph>`. The cache is in-memory only — every process restart rebuilds the graph from the Tantivy index. For large wikis this is a measurable cold-start penalty on every `llm-wiki serve` restart.

## Solution

Replace `GenerationCache<WikiGraph>` with `petgraph_live::live::GraphState<WikiGraph>`.

`GraphState` composes generation-keyed cache + snapshot lifecycle:
- On startup: attempt to load snapshot for the current generation key
- Hit: return cached graph (warm start — no Tantivy traversal)
- Miss (stale key, missing file, corrupt): cold build → save snapshot → return graph
- On `wiki_index_rebuild`: explicit invalidation writes a new snapshot for the new generation

`community_cache: GenerationCache<CommunityData>` stays unchanged — community data is fast to rebuild from the graph, not worth snapshotting.

## Cargo.toml change

```toml
petgraph-live = { version = "0.3", features = ["snapshot-lz4"] }
```

`snapshot-lz4` implies `snapshot`. LZ4 chosen over zstd: faster decompression, adequate compression ratio for graph data (struct-heavy, repetitive strings).

## SpaceContext change

```rust
// Phase 1
pub graph_cache: GenerationCache<WikiGraph>,

// Phase 2
pub graph_state: GraphState<WikiGraph>,
```

`community_cache: GenerationCache<CommunityData>` unchanged.

`GraphState` is `Send + Sync`. No `RwLock` wrapper needed.

## Snapshot configuration

`GraphState` requires a `SnapshotConfig` at construction time:

```rust
SnapshotConfig {
    dir:         state_dir.join("snapshots").join(&space.name),
    name:        "wiki-graph".into(),
    format:      SnapshotFormat::Bincode,
    compression: Compression::Lz4,
    keep:        config.graph.snapshot_keep,
}
```

Generation key passed per call: `graph_state.get_or_build(current_gen, builder)`.

Key mismatch on load → stale snapshot → cold build → new snapshot saved automatically.

`state_dir` must come from `EngineState::state_dir` — never hardcode.

Snapshot directory created lazily by petgraph-live `save()` — no `mkdir` needed.

## Config additions

New fields in `GraphConfig` (`src/config.rs`):

```toml
[graph]
snapshot         = true         # default: true; false disables snapshot (CI, tests)
snapshot_keep    = 3            # snapshot rotation count (keep N most recent)
snapshot_format  = "bincode+lz4"  # or "bincode" / "bincode+zstd"
```

`snapshot = false` preserves Phase 1 behaviour — `GraphState` falls back to in-memory only. Useful in CI and integration tests where snapshot files are unwanted.

New arms in `set_global_config_value` / `get_config_value` / `set_wiki_config_value`: `graph.snapshot`, `graph.snapshot_keep`, `graph.snapshot_format`.

## `wiki_index_rebuild` invalidation

After rebuild, the old snapshot is stale. Explicit invalidation required:

```rust
space.graph_state.invalidate();
// Optionally pre-warm synchronously:
space.graph_state.get_or_build(new_gen, builder)?;
```

Without explicit invalidation, the next call detects generation mismatch and cold-builds automatically — but leaving a stale snapshot on disk is wasteful. `invalidate()` deletes it.

## `LabeledEdge` serde requirement

`GraphState` snapshot requires `WikiGraph` nodes and edges to implement `Serialize + Deserialize`. Phase 1 adds these derives to `LabeledEdge`. Phase 2 depends on that being in place.

`PageNode` already derives serde. No change needed.

## Affected files

| File | Change |
|------|--------|
| `Cargo.toml` | Change `petgraph-live = "0.3"` → `petgraph-live = { version = "0.3", features = ["snapshot-lz4"] }` |
| `src/engine.rs` | Replace `GenerationCache<WikiGraph>` with `GraphState<WikiGraph>`; add `SnapshotConfig` construction |
| `src/space_builder.rs` | Pass `state_dir` and `SnapshotConfig` when constructing `SpaceContext` |
| `src/ops/graph.rs` | Update graph access from `graph_cache.get_or_build` → `graph_state.get_or_build` |
| `src/ops/stats.rs` | Same — `graph_state.get_or_build` |
| `src/mcp/handlers.rs` | `wiki_index_rebuild` handler calls `space.graph_state.invalidate()` after rebuild |
| `src/config.rs` | Add `graph.snapshot`, `graph.snapshot_keep`, `graph.snapshot_format` to `GraphConfig`; add match arms |
| `docs/implementation/graph-cache.md` | Document snapshot lifecycle |
| `CHANGELOG.md` | Add `[Unreleased]` entry |

## Breaking changes

None public. Snapshot files appear under `state_dir/snapshots/<wiki-name>/`.

## Constraints

- `gen` is reserved in Rust 2024 — never use as variable name
- `state_dir` never hardcoded — always from `EngineState::state_dir`
- `snapshot = false` must fully bypass `GraphState` snapshot logic (pass via config at construction)
- Community cache stays `GenerationCache<CommunityData>` — not snapshotted
- Integration tests must set `graph.snapshot = false` to avoid snapshot files in `tempfile::TempDir`

## Validation

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --doc
# Manual: start server, kill, restart — first tool call must not log "building graph from index"
```

## Definition of done

- [ ] `GenerationCache<WikiGraph>` replaced with `GraphState<WikiGraph>` in `SpaceContext`
- [ ] `SnapshotConfig` constructed from `state_dir` + `GraphConfig`
- [ ] `wiki_index_rebuild` calls `graph_state.invalidate()` after rebuild
- [ ] `graph.snapshot = false` disables snapshot in tests and CI
- [ ] Warm-start verified: process restart skips cold build log
- [ ] All tests pass, clippy clean, fmt clean, doc tests pass
- [ ] `CHANGELOG.md` `[Unreleased]` updated

## See also

- [Phase 1 — GenerationCache](2026-05-03-petgraph-live-cache.md) (required prerequisite)
- [Phase 3 — Algorithm suite](2026-05-03-petgraph-live-algorithms.md) (independent)
