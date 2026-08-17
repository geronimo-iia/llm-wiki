# Watcher rebuild guard: `Arc<AtomicBool>` per `SpaceContext`

## Decision

Add `rebuilding: Arc<AtomicBool>` to `SpaceContext`. The watcher checks this
flag before dispatching a `schema_rebuild` task and skips if one is already
in progress. The flag is owned by `SpaceContext` and reset to `false` on
re-mount.

## Context

The filesystem watcher debounces events but does not deduplicate rebuild
triggers across debounce windows. Two rapid schema changes (e.g. saving a
`.json` file twice in quick succession) produce two `WatchAction::RebuildIndex`
events. With `schema_rebuild` moved to `spawn_blocking` (P2.1), both tasks
are submitted to the blocking thread pool concurrently. The second task blocks
on Tantivy's internal write lock, then runs a full redundant rebuild after the
first completes — wasting CPU and I/O for no benefit (P2.4).

## Alternatives considered

**Track the `JoinHandle` and abort or await it** — would allow cancelling an
in-progress rebuild when a newer event arrives. Rejected: Tantivy's index
writer does not support cancellation mid-rebuild. Aborting the task would
leave the index in an inconsistent state. The correct behaviour is to let the
first rebuild complete and skip the second, not cancel the first.

**`tokio::sync::watch` channel per wiki** — a sender/receiver pair where the
watcher sends a "rebuild needed" signal and a dedicated per-wiki task drains
it. Cleaner architecture but introduces a per-wiki background task, complicates
shutdown coordination, and is disproportionate to the problem. The `AtomicBool`
achieves the same skip-if-busy semantics with no new tasks and no new
synchronization primitives beyond what `std` provides.

**Debounce window extension** — increase `debounce_ms` to reduce the chance of
two events landing in separate windows. Rejected: does not eliminate the race,
only reduces its frequency. Also degrades responsiveness for legitimate
sequential changes.

## Why `Arc<AtomicBool>` on `SpaceContext`

- No new dependencies. `AtomicBool` and `Ordering` are in `std`.
- The flag is co-located with the space it guards. A re-mount via
  `mount_wiki` constructs a new `SpaceContext` with `rebuilding = false`,
  automatically resetting any stuck flag without special-case logic.
- `Arc` allows `watch.rs` to clone the flag out from under the read lock
  before releasing it, so the lock is not held during the `spawn_blocking`
  call or the `.await`.
- `compare_exchange(false, true, AcqRel, Acquire)` is the correct atomic
  test-and-set: only one concurrent caller wins; all others skip.

## Known edge case

If the Tokio future running `run_watcher` is dropped between setting the flag
(`flag.swap(true, ...)`) and the `spawn_blocking` `.await` completing — for
example, if the `CancellationToken` fires mid-iteration — the flag stays
`true` and that wiki will not rebuild again in the current process lifetime.

This is benign for two reasons:

1. The cancellation token fires only on server shutdown. `SpaceContext` is
   dropped shortly after, taking the flag with it.
2. If the wiki is re-mounted (e.g. after a config reload), `mount_wiki`
   constructs a fresh `SpaceContext` with `rebuilding = false`.

There is no scenario where a stuck flag causes a permanently broken wiki in a
running server under normal operation.

## Consequences

- `SpaceContext` gains `pub rebuilding: Arc<AtomicBool>`, initialised to
  `false` in `mount_space`.
- `src/engine.rs`: `use std::sync::atomic::{AtomicBool, Ordering}` added.
- `src/watch.rs`: `RebuildIndex` arm clones the flag under a brief read lock,
  checks it with `swap`, and resets it in all three outcome branches
  (`Ok(Ok)`, `Ok(Err)`, `Err` from panic).
- `schema_rebuild` itself has no knowledge of the flag — the guard lives
  entirely in the watcher, keeping `engine.rs` free of watcher-specific logic.
