# Rebuild serialisation: `rebuild_lock: Mutex<()>` on `SpaceIndexManager`

## Decision

Add `rebuild_lock: Mutex<()>` to `SpaceIndexManager`. `rebuild()` acquires it
at entry and holds it for the full rebuild duration, serialising concurrent
rebuild calls on the same wiki space.

## Context

Two concurrent `rebuild_index` calls on the same space both enter the
three-rename atomic swap:

```
search-index/          -> search-index-prev/
search-index-building/ -> search-index/
```

If both callers complete `writer.commit()` before either runs the swap, the
second rename overwrites the result of the first. Both readers call
`reload_reader()` at non-deterministic points. The outcome is a torn index
state — neither rebuild's result is guaranteed to be what the reader sees.

This scenario arises when a watcher event fires while an explicit
`wiki_index_rebuild` tool call is in flight.

## Relation to `SpaceContext.rebuilding: Arc<AtomicBool>`

`rebuilding` (see `watcher-rebuild-guard-atomic-bool.md`) prevents the
**watcher** from submitting a second rebuild task when one is already in
progress. It is a best-effort skip at task submission time.

`rebuild_lock` is a hard serialisation gate at execution time. It covers cases
`AtomicBool` cannot:

- A direct `wiki_index_rebuild` MCP call concurrent with a watcher-triggered
  rebuild. The tool call bypasses the watcher's flag check entirely.
- A race between the watcher's `compare_exchange` check and the flag being set
  — the window is tiny but non-zero.

The two mechanisms are complementary: `AtomicBool` reduces unnecessary
rebuilds; `rebuild_lock` guarantees correctness when they do overlap.

## Alternatives considered

**Rely on Tantivy's internal `IndexWriter` lock** — `IndexWriter` serialises
writes within a single `Index` instance. But `rebuild()` opens a fresh
`Index::open_or_create()` in `search-index-building/`, not the live index.
Two concurrent rebuilds use two independent writer instances — Tantivy's lock
does not protect the rename sequence.

**Hold `state: RwLock` write lock for the full rebuild** — would block all read
operations (search, list, graph) for the entire rebuild duration. Rejected:
rebuilds can take seconds on large wikis; blocking reads for that window is
unacceptable.

**Per-space background task with a channel** — a single long-lived task per
wiki draining rebuild requests from a channel; concurrent requests queue.
Rejected: adds per-wiki background tasks and shutdown coordination. The
`Mutex<()>` achieves the same serialisation with no new tasks and no new
primitives beyond `std`.

**`tokio::sync::Mutex`** — `rebuild()` is a blocking function called via
`spawn_blocking`; it runs on a thread-pool thread, not a tokio task. A
`std::sync::Mutex` is correct and avoids the `async` propagation that
`tokio::sync::Mutex` would require.

## Why `Mutex<()>` on `SpaceIndexManager`

- `SpaceIndexManager` is the natural owner: it owns the index directory and all
  writers. The lock belongs with the resource it protects.
- `Mutex<()>` is zero-size in the guard state — no payload to initialise or
  manage.
- `std::sync::Mutex` is correct for the blocking context (`spawn_blocking`).
- Second caller blocks and then runs a full rebuild — it does not skip. If the
  first rebuild was triggered by a schema change and the second by a watcher
  event, the second rebuild is still needed to pick up any changes committed
  during the first. Skipping would leave the index stale.

## Consequences

- `SpaceIndexManager` gains `rebuild_lock: Mutex<()>`, initialised in `new()`.
- `rebuild()` acquires the guard at entry; the guard is held for the entire
  three-rename swap and `reload_reader()` call.
- No change to `search()`, `list()`, or `status()` — they acquire
  `inner: RwLock<IndexInner>` at read level as before.
- Regression test: `concurrent_rebuilds_do_not_corrupt_index` confirms two
  simultaneous `rebuild()` calls both complete without corrupting the index.
