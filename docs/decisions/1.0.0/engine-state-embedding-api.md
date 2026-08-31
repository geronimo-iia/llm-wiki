# WikiEngine embedding API — `with_state` and `pub(crate)` fields

## Decision

Add `WikiEngine::with_state<F, T>` as the single public read accessor for
`EngineState`. Restrict `state` and `config_write_lock` to `pub(crate)`.
Provide a `#[doc(hidden)] pub fn state_for_test` accessor so integration tests
retain direct lock access without blocking the visibility change.

No `with_state_mut` is added. The existing higher-level methods (`mount_wiki`,
`unmount_wiki`, `set_default`) cover the write cases; `pub(crate)` gives
internal lib code direct write-lock access.

## Context

`WikiEngine` exposed its internal lock fields as `pub`:

```rust
pub struct WikiEngine {
    pub state: Arc<RwLock<EngineState>>,
    pub config_write_lock: Arc<Mutex<()>>,
}
```

This caused two independent problems addressed together.

**Lock-poison boilerplate** (finding #2). Every caller repeated the same
error-mapping idiom:

```rust
let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
```

There were 19 sites in `src/main.rs`, 4 in `src/engine.rs`, 2 in
`src/server.rs`, and 1 in `examples/embed.rs` — 26 sites total producing
identical error strings with no shared definition.

**Over-wide public API** (finding #7). `state` and `config_write_lock` were
never intended as stable API. Embedders writing against `engine.state.read()`
couple to the lock type, poison-handling convention, and `EngineState` field
layout. Any change to the interior mutability model is a breaking change for
every downstream consumer.

## API added

```rust
pub fn with_state<F, T>(&self, f: F) -> anyhow::Result<T>
where
    F: FnOnce(&EngineState) -> anyhow::Result<T>,
{
    let engine = self
        .state
        .read()
        .map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
    f(&engine)
}

#[doc(hidden)]
pub fn state_for_test(&self) -> &Arc<RwLock<EngineState>> {
    &self.state
}
```

`with_state` is the embedding surface. `state_for_test` is a bridge accessor
for integration tests (see Constraint below). Neither is part of the stable
embedding contract; both are in `impl WikiEngine` in `src/engine.rs`.

## Why `with_state_mut` was not added

All write-lock sites are inside the library crate (`engine.rs`, `ops/spaces.rs`).
After `pub(crate)`, they retain direct `self.state.write()` access. There are
no external write-lock callers — `mount_wiki`, `unmount_wiki`, and `set_default`
are the only paths that mutate `EngineState` and they are already wrapped
methods. Adding `with_state_mut` to the public API would widen the embedding
surface without any active caller.

## Constraint: integration test layer

`tests/*.rs` files compile as separate crates. `pub(crate)` on `state` is not
visible to them. There were ~130 direct `.state.read().unwrap()` sites across
20+ test files. The options were:

1. Keep `state` as `pub` — defers the API problem to post-1.0.
2. Add `#[cfg(test)] pub fn state_for_test` — only compiled in test builds;
   invisible to embedders at runtime but leaves the field accessible.
3. Add `#[doc(hidden)] pub fn state_for_test` — always compiled; hidden from
   rustdoc; accessible to integration tests and to any downstream consumer who
   discovers it. Semantically equivalent to option 2 for the test use case but
   does not gate on build profile.

Option 3 was chosen. The `#[doc(hidden)]` attribute signals that the method is
not part of the public contract. Downstream consumers relying on it opt in to
unstable surface. The 130 test sites were migrated mechanically from
`.state.read()` to `.state_for_test().read()`.

`config_write_lock` had zero test-layer references — it was made `pub(crate)`
unconditionally.

## Alternatives considered

**Keep `state` pub, deprecate with `#[deprecated]`.** Avoids the test migration.
Rejected — a deprecated-but-pub field is still part of the stable API until the
next major version. The goal is a clean 1.0.0 surface.

**Move integration tests into `src/` as `#[cfg(test)]` modules.** Would give
them `pub(crate)` access. Rejected — the test files are large and the migration
has no functional benefit. Noted as a post-1.0 path in
`pub-crate-partial-migration.md`.

**Add `with_state_mut` alongside `with_state`.** Widens the embedding surface
without an active caller. Deferred until a concrete external write use case
exists.

## Consequences

- `WikiEngine::with_state` is the documented entry point for embedding code
  that needs to read engine state.
- `engine.state` and `engine.config_write_lock` are `pub(crate)`. External
  crates (binary, downstream embedders, integration tests) cannot access them
  directly.
- `state_for_test()` gives integration tests `.read()/.write()` access. It is
  `#[doc(hidden)]` and carries no stability guarantee.
- The 26 production boilerplate sites are eliminated. Error message is now
  uniform: `"engine lock poisoned"`.
- The `acp/` layer's intentional poison-recovery pattern
  (`unwrap_or_else(|e| e.into_inner())`) is unaffected — `pub(crate)` grants
  it direct `state` access.
- Implementation: `.claude/plans/2026-08-28-private-state-with-state.md`
