---
title: "Schema Change Detection"
summary: "How the engine detects type registry changes and triggers index rebuilds."
status: ready
last_updated: "2025-07-18"
---

# Schema Change Detection

For the spec, see
[index-management.md](../specifications/engine/index-management.md).

## Per-Wiki Type Registry

Each wiki has its own `schemas/` directory and `wiki.toml`. The engine
builds a `SpaceTypeRegistry` and `IndexSchema` per wiki via
`build_space()` in `space_builder.rs`.

```
Engine {
    spaces: { name → SpaceState {
        type_registry: SpaceTypeRegistry,
        index_schema: IndexSchema,
    }}
}
```

## Shared Builder

`build_space(repo_root, tokenizer)` reads each schema file once and
produces both `SpaceTypeRegistry` and `IndexSchema`. No raw JSON is
kept after construction. See `space_builder.rs`.

## state.toml

```toml
schema_hash = "a1b2c3d4..."
commit      = "a3f9c12..."
pages       = 142
sections    = 8
built       = "2025-07-17T14:32:01Z"

[types]
concept  = "e5f6a7b8..."
paper    = "c9d0e1f2..."
skill    = "3a4b5c6d..."
```

`schema_hash` is a content-based hash from
`SpaceTypeRegistry::schema_hash()`. Per-type hashes are stored for
future partial rebuild support.

## Staleness

```
stale = (state.commit != HEAD)
     || (state.schema_hash != registry.schema_hash())
```

Missing or malformed `state.toml` is treated as "never built".

## Startup Sequence Per Wiki

```
1. build_space(repo_root, tokenizer) → (type_registry, index_schema)
2. Read state.toml → stored schema_hash
3. Compare with type_registry.schema_hash():
   - Missing → full rebuild
   - Mismatch → full rebuild
   - commit != HEAD → incremental update (if auto_rebuild)
   - All match → current
4. Store SpaceState { type_registry, index_schema, ... }
```

## What Triggers a Rebuild

| Change | Detected by | Action |
|--------|------------|--------|
| Schema file added/removed/modified | `schema_hash` mismatch | Full rebuild |
| `wiki.toml` `[types.*]` changed | `schema_hash` mismatch | Full rebuild |
| New commit (content change) | `commit` mismatch | Incremental update |
| `state.toml` missing/malformed | Parse failure | Full rebuild |

## Partial Rebuild (future)

Per-type hashes are stored in `state.toml` but not compared yet. Any
`schema_hash` mismatch triggers a full rebuild.
