# ACP Sessions: `parking_lot::Mutex` over `std::sync::Mutex` or `tokio::sync::Mutex`

## Decision

Replace `std::sync::Mutex` with `parking_lot::Mutex` for the `Sessions` type
in `src/acp/mod.rs`. Do not use `tokio::sync::Mutex`.

## Context

`Sessions = Arc<Mutex<HashMap<String, AcpSession>>>` is shared across all ACP
request handlers in `src/acp/server.rs` and four helper functions in
`src/acp/helpers.rs` (`resolve_wiki_name`, `get_cancelled`, `clear_active_run`)
and the workflow entry points (`run_graph`, `run_research`, `run_lint`,
`run_ingest`).

The problem with `std::sync::Mutex` (P2.3 in the roadmap) is twofold:

1. A panic while the lock is held poisons it permanently. Every subsequent
   `.lock().unwrap()` panics, crashing the ACP server task with no recovery path.
2. `std::sync::Mutex` guards must not be held across `.await` points. The
   current code acquires and releases before any await, so it does not deadlock
   today — but the type gives no compile-time protection against a future
   refactor introducing a held-across-await bug.

## Alternatives considered

**`tokio::sync::Mutex`** — the natural async-first choice. Rejected because
`helpers.rs` functions (`resolve_wiki_name`, `get_cancelled`, `clear_active_run`)
are called from sync workflow entry points (`run_graph`, `run_research`, etc.).
Making them async would cascade through all four workflow modules, touching
dozens of call sites for a change whose only benefit is mutex type consistency.
The critical sections are all brief (HashMap lookup or insert) — there is no
benefit to an async mutex here.

**Keep `std::sync::Mutex` and remove `.unwrap()`** — `.lock()` returns
`LockResult<MutexGuard>` which is `Err` only when poisoned. Replacing
`.unwrap()` with `.unwrap_or_else(|e| e.into_inner())` recovers the guard
after a panic but leaves the map in an unknown state. This trades a crash for
silent data corruption. Rejected.

**`std::sync::Mutex` with explicit poison recovery** — same as above. The
session map after a panic is untrustworthy regardless of recovery strategy.

## Why `parking_lot::Mutex`

- `.lock()` returns `MutexGuard` directly — no `Result`, no `.unwrap()`,
  no poison concept. A panic in a critical section does not affect subsequent
  lock acquisitions.
- Sync-friendly: works in both sync and async contexts without `.await`.
  Helper functions remain sync; no cascade refactor required.
- `parking_lot 0.12` is already in the transitive dependency tree (via
  `jsonschema` and `tokio`). Adding it as a direct dep fetches zero new
  packages.
- Guards must still not be held across `.await` points — this is enforced by
  keeping all critical sections in explicit blocks that drop the guard before
  any subsequent async work.

## Consequences

- `src/acp/mod.rs`: `use parking_lot::Mutex` replaces `use std::sync::Mutex`.
  `Sessions` type alias text is unchanged.
- `src/acp/server.rs`: all `.lock().unwrap()` and `if let Ok(s) = sessions.lock()`
  patterns replaced with direct `.lock()` calls inside explicit drop-scoped blocks.
- `src/acp/helpers.rs`: same pattern applied to the three helper functions.
- `Cargo.toml`: `parking_lot = "0.12"` added as a direct dependency.
- **Invariant:** `parking_lot::MutexGuard` must be dropped before any `.await`
  in async handlers. Explicit blocks enforce this at every call site.
