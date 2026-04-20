# Decision: Engine Redesign

## Context

Three issues identified in the design review:
1. Read-lock for mutations is fragile
2. Naming is misleading (`EngineManager`, `on_*` methods)
3. `build()` does too much (~90 lines)

→ [analysis prompt](../prompts/engine-manager-redesign.md)

## Decisions

### 1. Lock: interior mutability in SpaceIndexManager

The lock problem is not in `WikiEngine` (née `EngineManager`) — it's
in `SpaceIndexManager`. Methods like `update`, `rebuild`, `writer`
take `&self` but mutate the tantivy index. This works today because
a fresh `IndexWriter` is opened each call, but it's the wrong
abstraction boundary.

**Change:** wrap the mutable parts of `SpaceIndexManager` in interior
mutability:

```rust
pub struct SpaceIndexManager {
    wiki_name: String,
    index_path: PathBuf,
    inner: RwLock<IndexInner>,
}

struct IndexInner {
    tantivy_index: Option<Index>,
    index_reader: Option<IndexReader>,
}
```

- `searcher()` takes a read lock on `inner` (concurrent reads OK)
- `update()`, `rebuild()`, `writer()` take a write lock on `inner`
- `open()` no longer needs `&mut self`
- The outer `WikiEngine` can hold `Arc<RwLock<EngineState>>` with
  read locks everywhere — mutations are scoped inside `SpaceIndexManager`

### 2. Rename: `Engine` → `EngineState`, `EngineManager` → `WikiEngine`

| Current | New | Rationale |
|---------|-----|-----------|
| `Engine` | `EngineState` | It's a data bag (config + spaces), not an actor |
| `EngineManager` | `WikiEngine` | The public-facing coordinator |
| `EngineManager::engine` field | `WikiEngine::state` | Matches the type name |

### 3. Remove `on_*` stubs, rename `on_ingest`

| Current | Action |
|---------|--------|
| `on_ingest` | Rename to `refresh_index` |
| `on_wiki_added` | Remove (bails unconditionally) |
| `on_wiki_removed` | Remove (bails unconditionally) |
| `on_config_change` | Remove (bails unconditionally) |

Hot-reload can be added later with proper methods. Stubs that always
fail are misleading.

### 4. Extract `mount_wiki` from `build()`

```rust
fn mount_wiki(
    entry: &WikiEntry,
    state_dir: &Path,
    config: &GlobalConfig,
) -> Result<SpaceContext>
```

`build()` becomes: load config → `entries.map(mount_wiki)` → collect
into HashMap. Per-wiki errors warn and skip (don't fail the engine).

## Migration Order

1. Extract `mount_wiki` — pure refactor, no API change, safe first step
2. Rename `Engine`/`EngineManager` — mechanical, high churn but no logic change
3. Remove `on_*` stubs — trivial, update callers (if any)
4. Interior mutability in `SpaceIndexManager` — most complex, do last

## Estimated Effort

| Step | Files | Lines |
|------|-------|-------|
| `mount_wiki` extraction | 1 (`engine.rs`) | ~20 net |
| Rename | ~10 (engine, ops, mcp, acp, server, main, tests) | ~80 (mechanical) |
| Remove stubs | 1 (`engine.rs`) + callers | ~15 deleted |
| Interior mutability | 1 (`index_manager.rs`) + `engine.rs` | ~40 net |
