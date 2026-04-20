---
title: "Manager Pattern"
summary: "Change detection and propagation pattern used across the engine — detect, refresh, cascade."
status: ready
last_updated: "2025-07-17"
---

# Manager Pattern

The engine uses a consistent pattern for managing stateful components
that can change at runtime. Each component has a manager that wraps it
with change detection, partial refresh, and cascade reporting.

## The Pattern

```rust
struct FooManager {
    source: PathBuf,          // where the source of truth lives
    state: Foo,               // current in-memory state
}

impl FooManager {
    fn build(source: &Path) -> Result<Self>;
    fn has_changed(&self) -> Result<bool>;
    fn refresh(&mut self) -> Result<RefreshReport>;
    fn state(&self) -> &Foo;
}
```

Three operations:

- **build** — construct from source files (startup or full rebuild)
- **has_changed** — cheap check (hash comparison) to detect if source
  files changed since last build
- **refresh** — detect what changed, rebuild only the affected parts,
  return a report describing what happened and what downstream
  components need to do

## Refresh Report

Each manager returns a report that tells its caller what cascading
actions are needed:

```rust
struct RefreshReport {
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
    // what the caller should do
    needs_full_rebuild: bool,
    needs_partial_rebuild: Vec<String>,
}
```

The caller (typically `EngineManager`) reads the report and decides
what to do next — rebuild an index, reload config, etc.

## Where It's Used

| Manager                    | Wraps               | Source of truth          | Cascades to                    |
| -------------------------- | ------------------- | ------------------------ | ------------------------------ |
| `SpaceTypeRegistryManager` | `SpaceTypeRegistry` | `wiki.toml` + `schemas/` | `IndexSchema` -> tantivy index |
| `EngineManager`            | `Engine`            | all sources              | all components                 |

## Dependency Chain

```
EngineManager
    -> SpaceTypeRegistryManager (per wiki)
        -> SpaceTypeRegistry
            -> IndexSchema
                -> tantivy Index
    -> IndexRegistry
    -> GlobalConfig
```

`EngineManager` is the top-level orchestrator. It calls each
component's manager, reads the refresh report, and cascades to
downstream components.

## Design Principles

### Cheap detection, expensive rebuild

`has_changed()` is a hash comparison — microseconds. `refresh()` may
recompile validators and rebuild indexes — milliseconds to seconds.
Always check before rebuilding.

### Partial over full

When possible, rebuild only what changed. If 1 type out of 15 changed,
recompile 1 validator and re-index pages of that type — not everything.

### Reports over side effects

`refresh()` returns a report describing what happened. It doesn't
trigger downstream rebuilds itself. The caller decides what to do.
This keeps each manager focused on its own component.

### Immutable state, mutable manager

The state (`Foo`) is read by many consumers concurrently. The manager
(`FooManager`) is the only writer. In `llm-wiki serve`, the state is
behind `Arc<RwLock<>>` — reads don't block each other, writes are
brief and infrequent.

## Initial Scope

- `EngineManager` with `on_ingest` (incremental index update)
- `SpaceTypeRegistryManager` with `build` and `has_changed`
- `refresh` returns "restart required" for type changes

Full `refresh` with partial rebuilds and file watcher integration come
later.
