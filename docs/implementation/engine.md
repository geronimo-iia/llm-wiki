---
title: "Engine Implementation"
summary: "Top-level engine structs, space mounting, and how registries and indexes compose at runtime."
status: ready
last_updated: "2026-04-28"
---

# Engine Implementation

Implementation reference for the engine runtime. Not a specification —
see [specifications/](../specifications/README.md) for the design.

## Core Structs

```rust
/// Holds the current engine state — config, mounted spaces.
pub struct EngineState {
    pub config: GlobalConfig,
    pub config_path: PathBuf,
    pub state_dir: PathBuf,
    pub spaces: HashMap<String, SpaceContext>,
}

/// Top-level coordinator. Wraps EngineState in Arc<RwLock>.
pub struct WikiEngine {
    pub state: Arc<RwLock<EngineState>>,
}
```

`EngineState` holds the current state. `WikiEngine` sits above it and
provides `build`, `refresh_index`, and `rebuild_index`. Tools read
from `EngineState` via the shared reference.

### SpaceContext

One per mounted wiki. Holds everything needed to serve a wiki:

```rust
pub struct SpaceContext {
    pub name: String,
    pub wiki_root: PathBuf,
    pub repo_root: PathBuf,
    pub type_registry: SpaceTypeRegistry,
    pub index_schema: IndexSchema,
    pub index_manager: SpaceIndexManager,
}
```

## Startup

`WikiEngine::build` loads config, then calls `mount_wiki` per entry:

```
1. Load GlobalConfig from ~/.llm-wiki/config.toml
2. For each registered wiki → mount_wiki():
   a. Build SpaceTypeRegistry from schemas/ + wiki.toml overrides
   b. Build IndexSchema from the type registry
   c. Create SpaceIndexManager
   d. Check staleness (StalenessKind enum):
      - Current → skip
      - CommitChanged → incremental update
      - TypesChanged → partial rebuild (affected types only)
      - FullRebuildNeeded → full rebuild
   e. Open tantivy index (with auto-recovery on corruption)
   f. Return SpaceContext
3. Per-wiki errors: warn and skip (don't fail the engine)
4. Assemble EngineState, wrap in Arc<RwLock>
```

## Tool Dispatch

Tools receive a read reference to `EngineState` and a wiki name (from
`--wiki` flag or default). Index mutations go through `WikiEngine`.

```rust
// Read path (search, list, graph, read)
let engine = wiki_engine.state.read();
let space = engine.space(wiki_name)?;
let searcher = space.index_manager.searcher()?;

// Write path (ingest)
wiki_engine.refresh_index(wiki_name)?;
```

## WikiEngine Interface

```rust
impl WikiEngine {
    /// Build from config file. Mounts all registered wikis.
    pub fn build(config_path: &Path) -> Result<Self>;

    /// Incremental index update after ingest.
    pub fn refresh_index(&self, wiki_name: &str) -> Result<UpdateReport>;

    /// Full index rebuild.
    pub fn rebuild_index(&self, wiki_name: &str) -> Result<IndexReport>;
}
```

Hot-reload (add/remove wikis, config changes without restart) is not
yet implemented. Currently requires a server restart.

## Lifecycle

### llm-wiki serve

`WikiEngine` built once at startup. `Arc<RwLock<EngineState>>` shared
across all transports (stdio, SSE, ACP). Read-heavy workload — most
tool calls only read.

### CLI commands

`WikiEngine` built per invocation. Schema hash check determines
whether to use cached index or rebuild. For single-shot commands
(search, list, read), the engine is read-only.
