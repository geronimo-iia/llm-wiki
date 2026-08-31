# Close mmap handles before rename in `rebuild()`

## Decision

In `SpaceIndexManager::rebuild()`, drop `inner.tantivy_index` and
`inner.index_reader` (clearing the `RwLock<IndexInner>`) before the
live→backup directory rename. After the rename sequence completes, open a
fresh `Index` and `IndexReader` from the new live directory instead of calling
`reload_reader()` on the cleared handles.

## Context

`rebuild()` performs a three-phase atomic directory swap:

```
search-index-building/   (newly written)
search-index/            (current live, open for reading)
search-index-prev/       (backup, deleted at end)

Phase 1: search-index/          → search-index-prev/
Phase 2: search-index-building/ → search-index/
Phase 3: delete search-index-prev/
```

On Windows, `fs::rename` on a directory fails with **os error 5 (Access is
denied)** if any file handle — including a memory-mapped file — is open inside
that directory. `tantivy::Index` uses `MmapDirectory`, which memory-maps index
segment files. The `Index` and `IndexReader` held in `inner` keep those maps
alive for the duration of the `rebuild()` call, blocking Phase 1.

A second Windows constraint: after the swap, if the rebuilt index is opened and
its segment files are memory-mapped, any attempt to overwrite those files (e.g.
in tests that simulate corruption) fails with **os error 1224
(ERROR_USER_MAPPED_FILE)**. Clearing the handles in tests before writing corrupt
data requires an explicit escape hatch.

Neither constraint exists on Linux/macOS, where the kernel allows rename over
open file descriptors and mmap'd files can be overwritten while mapped.

## Why close before rename and reopen after

**Close before rename** — the only portable way to guarantee no file handles
are open inside `search-index/` before Phase 1. The `RwLock<IndexInner>` write
guard sets both fields to `None`, releasing all `Arc`-counted references.
`tantivy` drops mmap handles when the last `Index` clone is dropped.

**Reopen fresh after rename, not `reload_reader()`** — `reload_reader()` calls
`reader.reload()` on the existing `IndexReader`. After the handles are cleared,
`inner.index_reader` is `None`; there is nothing to reload. The reopen path
also integrates the `fail_next_reload` test-injection flag, which previously
lived in `reload_reader()` and would not have been triggered by the new code
path.

## `close()` test escape hatch

Tests that verify recovery from a corrupt index must overwrite files in the
live `search-index/` directory after a `rebuild()` has opened it. On Windows
those files are mmap'd and cannot be overwritten while the mapping is live.
`SpaceIndexManager::close()` (marked `#[doc(hidden)]`) clears both
`inner.tantivy_index` and `inner.index_reader`, releasing the mmap handles
without tearing down the manager. Tests call it immediately before writing
corrupt data. Production code never calls it.

## Alternatives considered

**Hold the inner write lock for the full rename sequence** — prevents any
concurrent reader from acquiring `inner` between the close and the reopen.
Rejected: rebuild already holds `rebuild_lock`; no concurrent rebuild can
interleave. A search request racing with the rename window would block on
`inner.read()` for a few milliseconds and then succeed on the freshly opened
index — acceptable.

**Use a temporary copy instead of rename** — copy `search-index-building/` to
a new `search-index/` while the old one remains open, then swap atomically.
Rejected: defeats the purpose of the building→live swap and adds a full index
copy on every rebuild.

**Open `search-index-building/` for reading before the rename** — so the new
`Index` handle points at the path that will become live. Rejected: path
identity changes after the rename; tantivy caches the directory path internally;
behaviour after rename is undefined.

## Consequences

- `rebuild()` acquires `inner.write()` before Phase 1, sets both fields to
  `None`, and releases the lock. The rename sequence then runs with no open
  handles.
- After Phase 2, `rebuild()` opens a fresh `Index` from the new
  `search-index/live` directory and creates a new `IndexReader` with
  `ReloadPolicy::Manual`. If `fail_next_reload` is set (tests only), this open
  returns an injected error and the rollback path is exercised.
- `SpaceIndexManager` gains a `#[doc(hidden)]` `close()` method used only in
  unit tests (`tests/index_manager.rs`).
- The existing `rebuild_lock: Mutex<()>` (see
  `rebuild-lock-mutex-space-index-manager.md`) is unaffected — it still
  serialises concurrent `rebuild()` calls.
- Regression tests: `graph_not_empty_after_index_rebuild_cli` (CLI test, caught
  the original failure) and `open_recovers_from_corruption` /
  `open_fails_without_recovery_on_corruption` (unit tests, caught the mmap
  overwrite failure).
