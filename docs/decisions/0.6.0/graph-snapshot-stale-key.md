---
title: "Fix stale graph snapshot key (issue #112)"
summary: "key_fn used generation() — an in-memory counter that resets to 0 on every process start — causing snapshot always treated as fresh on CLI invocations. Fixed by switching to last_commit()."
status: accepted
date: "2026-08-04"
---

# Fix stale graph snapshot key (issue #112)

## Decision

Replace `generation().to_string()` with `last_commit().unwrap_or("no-commit")` as the
`key_fn` inside `WikiGraphCache::WithSnapshot`. Consolidate all post-rebuild side effects
(graph cache refresh) in `ops::index_rebuild`.

## Context

`WikiGraphCache::WithSnapshot` wraps `petgraph-live`'s `GraphState`, which persists a
graph snapshot to disk keyed on whatever `key_fn` returns. On startup, `GraphState::init`
compares the current key against the snapshot filename — match means load from disk, miss
means cold build.

`generation()` is an `AtomicU64` starting at `0` on every process start. Every
`reload_reader()` call increments it. In a long-lived MCP server this works: the counter
accumulates and any index write changes the key. But the CLI is a fresh process per
invocation — `generation()` is always `0`, so the snapshot key never changes across
restarts, and a stale snapshot is served permanently even after `index rebuild`.

## Root cause

```rust
// BEFORE — resets to 0 on every process start
move || Ok(im_key.generation().to_string()),
```

On first CLI run after `index rebuild`, `generation()` is `0`. The snapshot written at
key `"0"` contains the pre-rebuild graph. On the next CLI run, `generation()` is `0`
again — key matches — stale snapshot loaded. The graph appears empty or outdated.

## Fix

```rust
// AFTER — reads state.toml; stable across restarts; changes on index rebuild
move || {
    Ok(im_key
        .last_commit()
        .unwrap_or_else(|| "no-commit".to_string()))
},
```

`last_commit()` reads the git HEAD SHA recorded in `state.toml` by `index rebuild`. It is:
- stable across process restarts for the same index content
- changed exactly when `index rebuild` runs and writes a new commit SHA
- available even when no commit exists (returns `None` → key `"no-commit"`)

Old snapshots keyed `"0"` (written by earlier versions using `generation()`) become
orphaned and are pruned by `petgraph-live`'s `keep_n` rotation.

## Side effect consolidation

Before this fix, `handlers.rs` called `graph_cache.rebuild()` after index operations.
With the key fix, `ops::index_rebuild` became the single owner of post-rebuild side
effects — it calls `graph_cache.rebuild()` internally. The duplicate block in
`handlers.rs` was removed.

This matters because CLI (`main.rs`) and MCP (`handlers.rs`) both call
`ops::index_rebuild`. If the graph cache refresh lived only in `handlers.rs`, CLI
invocations would skip it.

## Alternatives considered

**Keep `generation()`, persist across restarts** — would require writing the counter to
`state.toml` on every increment and reading it on startup. Adds disk I/O on every index
write. `last_commit()` is already persisted by the ingest path with no extra cost.

**Use a content hash of the index** — correct but expensive: requires reading all tantivy
segments on startup just to compute the key. `last_commit()` is an O(1) read from a small
TOML file.

**Use a timestamp** — not reproducible; two processes starting at slightly different times
after the same `index rebuild` would disagree on the key.

## Test coverage

- `tests/graph_snapshot.rs` — `graph_not_empty_after_index_rebuild_simulating_fresh_process`:
  three separate `WikiEngine::build` calls simulate three independent process lifetimes.
  Asserts graph is non-empty after rebuild even when each engine starts cold.
- `tests/graph_cache.rs` — `graph_cache_invalidated_after_rebuild`: updated to call
  `ops::index_rebuild` (not `index_manager.rebuild()` directly) so graph cache refresh
  is included in the assertion path.
- `tests/ops/index.rs` — `index_rebuild_populates_graph_cache`: asserts that
  `ops::index_rebuild` leaves a warm, non-empty graph cache entry.
