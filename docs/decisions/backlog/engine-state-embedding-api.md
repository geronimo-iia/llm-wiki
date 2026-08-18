---
title: "WikiEngine embedding API — hide engine.state behind with_state"
summary: "Replace direct engine.state.read() access with WikiEngine::with_state<F,T> so embedders don't couple to the internal lock shape or EngineState fields."
status: deferred
date: "2026-08-18"
---

# WikiEngine embedding API — hide `engine.state`

## Problem

`WikiEngine` exposes its internal state as a public field:

```rust
pub struct WikiEngine {
    pub state: Arc<RwLock<EngineState>>,
    pub config_write_lock: Arc<Mutex<()>>,
}
```

Every embedder who calls any operation today must write:

```rust
let state = engine.state.read().map_err(|_| anyhow::anyhow!("lock poisoned"))?;
ops::search::search(&state, &wiki_name, &params)
```

This leaks three things:

1. **Lock type.** Embedders know the interior mutability model is `RwLock`. Changing to `parking_lot::RwLock`, a sharded lock, or a `tokio::sync::RwLock` for an async variant is a breaking change for every caller.
2. **Lock error handling.** Each caller must decide what to do with a poisoned lock. The current convention (`map_err(|_| anyhow::anyhow!("lock poisoned"))`) is duplicated across the codebase.
3. **`EngineState` shape.** Embedders can access any `pub` field on `EngineState` directly — `spaces`, `config`, `config_path`, `state_dir` — even fields that are implementation details. Once embedded applications reference these, they become part of the stable API surface even though they were never intended to be.

The `examples/embed.rs` file illustrates the problem: it reaches into `engine.state` to resolve the wiki name even though that logic belongs inside the engine.

## Proposed API

Add two methods to `WikiEngine`:

```rust
impl WikiEngine {
    /// Read the engine state inside a closure.
    ///
    /// Preferred over `engine.state.read()` in embedding contexts — hides the lock
    /// type and centralises poison handling.
    pub fn with_state<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&EngineState) -> Result<T>,
    {
        let state = self.state.read().map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
        f(&state)
    }

    /// Mutate the engine state inside a closure.
    ///
    /// Intended for internal and advanced embedding use only. Prefer the higher-level
    /// `mount_wiki` / `unmount_wiki` / `set_default` methods for the common cases.
    pub fn with_state_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut EngineState) -> Result<T>,
    {
        let mut state = self.state.write().map_err(|_| anyhow::anyhow!("engine lock poisoned"))?;
        f(&mut state)
    }
}
```

The `examples/embed.rs` usage becomes:

```rust
// Before
let state = engine.state.read().map_err(|_| anyhow::anyhow!("lock poisoned"))?;
let wiki_name = wiki_override.as_deref().unwrap_or_else(|| state.default_wiki_name()).to_string();
let result = ops::search::search(&state, &wiki_name, &params)?;

// After
let wiki_name = engine.with_state(|s| {
    Ok(wiki_override.as_deref().unwrap_or_else(|| s.default_wiki_name()).to_string())
})?;
let result = engine.with_state(|s| ops::search::search(s, &wiki_name, &params))?;
```

## What does NOT change in this step

- `EngineState` stays public — its fields are still accessible inside the closure. Making fields `pub(crate)` is a separate, larger refactor (see `pub-crate-partial-migration` decision).
- Internal engine code (`src/main.rs`, `src/mcp/`, `src/acp/`) can keep accessing `engine.state.read()` directly. The new methods are for the embedding surface, not a mandate to refactor all internal sites.
- `engine.state` itself may stay `pub` for internal test access; deprecating it is optional and deferred.

## Migration

1. Add `with_state` and `with_state_mut` to `impl WikiEngine` in `src/engine.rs`.
2. Update `examples/embed.rs` to use `with_state`.
3. Add `#[deprecated]` to `pub state` if the field-level `pub` is to be hidden over time (semver-compatible deprecation).
4. Update lib.rs doc comment to advertise `with_state` as the embedding entry point.

The change is purely additive at step 1-2. Steps 3-4 are optional enhancements.

## Revisit conditions

- Any new embedding example is added — it must use `with_state`, not `engine.state.read()`.
- The lock type is reconsidered (e.g. async or parking_lot migration) — `with_state` makes that change transparent to embedders.
- Post-1.0 `pub(crate)` migration lands — coordinate to avoid a second wave of embedding breakage.
