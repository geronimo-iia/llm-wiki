---
title: "Engine Implementation"
summary: "Top-level engine struct, change manager, and how registries and indexes compose at runtime."
status: draft
last_updated: "2025-07-17"
---

# Engine Implementation

Implementation reference for the engine runtime. Not a specification —
see [specifications/](../specifications/README.md) for the design.

## Core Structs

```rust
/// Immutable snapshot of the engine state
struct Engine {
    config: GlobalConfig,
    types: GlobalTypeRegistry,
    indexes: IndexRegistry,
}

/// Manages the engine lifecycle and change propagation
struct EngineManager {
    engine: Arc<RwLock<Engine>>,
}
```

`Engine` holds the current state. `EngineManager` sits above it and
orchestrates reactions when something changes. Tools read from
`Engine` via the shared reference. Mutations go through
`EngineManager`.

## Registries

### GlobalConfig

Loaded from `~/.llm-wiki/config.toml`. Holds the space registry
(which wikis exist and where) and global defaults.

See [global-config.md](../specifications/model/global-config.md).

### GlobalTypeRegistry

Per-wiki type registries combined. Each `SpaceTypeRegistry` holds
compiled JSON Schema validators, alias maps, and edge declarations.

See [type-registry.md](type-registry.md).

### IndexRegistry

Per-wiki tantivy indexes combined.

```rust
struct IndexRegistry {
    spaces: HashMap<String, SpaceIndex>,
}

struct SpaceIndex {
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    schema: IndexSchema,
}
```

`IndexSchema` is built from the `SpaceTypeRegistry` — it holds the
tantivy `Schema` and field handles. See [tantivy.md](tantivy.md).

## Startup

```
1. Load GlobalConfig from ~/.llm-wiki/config.toml
2. For each registered wiki:
   a. Load SpaceTypeRegistry from wiki.toml + schemas/
   b. Check schema_hash against state.toml
   c. If mismatch -> rebuild index
   d. Open tantivy index -> build SpaceIndex
3. Assemble GlobalTypeRegistry from all SpaceTypeRegistries
4. Assemble IndexRegistry from all SpaceIndexes
5. Build Engine
6. Wrap in EngineManager
```

## Tool Dispatch

Tools receive a read reference to `Engine` and a wiki name (from
`--wiki` flag or default). Mutations go through `EngineManager`.

```rust
// Read path (search, list, graph, read)
let engine = manager.engine.read();
let space_index = engine.indexes.get(wiki_name)?;

// Write path (ingest, commit, config set)
manager.on_ingest(wiki_name, paths)?;
```

## Change Manager

`EngineManager` handles all state mutations. Each change may cascade
through the dependency chain:

```
wiki.toml / schemas/ changed
    -> rebuild SpaceTypeRegistry
        -> rebuild IndexSchema
            -> partial or full index rebuild

wiki added (spaces create)
    -> load new SpaceTypeRegistry + SpaceIndex
        -> register in both global registries

wiki removed (spaces remove)
    -> drop SpaceTypeRegistry + SpaceIndex

config changed (config set)
    -> reload affected config values

pages ingested
    -> incremental index update
```

### Interface

```rust
impl EngineManager {
    /// Pages written and need indexing
    fn on_ingest(&self, wiki: &str, paths: &[PathBuf]) -> Result<IngestReport>;

    /// Type schema changed (wiki.toml or schemas/ modified)
    fn on_type_change(&self, wiki: &str) -> Result<()>;

    /// New wiki registered
    fn on_wiki_added(&self, name: &str, path: &Path) -> Result<()>;

    /// Wiki removed from registry
    fn on_wiki_removed(&self, name: &str) -> Result<()>;

    /// Config key changed
    fn on_config_change(&self, key: &str, value: &str) -> Result<()>;
}
```

Each method knows the dependency chain and rebuilds only what's needed.

### Initial scope

Only `on_ingest` is implemented — incremental index update via git
diff. The other methods exist as stubs that return "restart required."

### Future

- `on_type_change` — rebuild affected `SpaceTypeRegistry` + `IndexSchema`,
  partial or full index rebuild based on per-type hashes
- `on_wiki_added` / `on_wiki_removed` — hot add/remove wikis without
  restart
- `on_config_change` — reload config values that affect runtime behavior
- File watcher integration — detect `wiki.toml` and `schemas/` changes
  automatically during `llm-wiki serve`

## Lifecycle

### llm-wiki serve

`EngineManager` built once at startup. `Arc<RwLock<Engine>>` shared
across all transports (stdio, SSE, ACP). Read-heavy workload — most
tool calls only read. Writes (ingest, commit) acquire the write lock
briefly.

### CLI commands

`EngineManager` built per invocation. Schema hash check determines
whether to use cached `schema.json` or rebuild. For single-shot
commands (search, list, read), the manager is read-only.
