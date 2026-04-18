---
title: "Type Registry Implementation"
summary: "How types are loaded, compiled, cached, and invalidated at runtime."
status: draft
last_updated: "2025-07-17"
---

# Type Registry Implementation

Implementation reference for the type registry. Not a specification —
see [type-system.md](../specifications/model/type-system.md) for the
design.

## Overview

The type registry is an in-memory cache of compiled validators and
metadata for all registered types. Built once at startup, used on every
ingest, invalidated when the type configuration changes.

## Core Structs

```rust
/// Per-wiki type registry
struct SpaceTypeRegistry {
    /// type name → compiled type
    types: HashMap<String, RegisteredType>,
    /// SHA-256 hash of all type inputs (for change detection)
    schema_hash: String,
    /// per-type hashes (for partial rebuild)
    type_hashes: HashMap<String, String>,
}

struct RegisteredType {
    /// compiled JSON Schema validator — no re-parsing on each ingest
    validator: jsonschema::Validator,
    /// x-index-aliases: source field → canonical field
    aliases: HashMap<String, String>,
    /// x-graph-edges: field → (relation, direction, target_types)
    edges: Vec<EdgeDecl>,
}

struct EdgeDecl {
    field: String,
    relation: String,
    direction: String,
    target_types: Option<Vec<String>>,
}

/// All wikis combined - used by cross-wiki operations
struct GlobalTypeRegistry {
    /// wiki name -> per-wiki registry
    spaces: HashMap<String, SpaceTypeRegistry>,
}
```

`GlobalTypeRegistry` is built at startup by loading each wiki's
`SpaceTypeRegistry`. Cross-wiki search and graph operations iterate
over all spaces.

## SpaceTypeRegistryManager

Manages the lifecycle of a `SpaceTypeRegistry` — builds, detects
changes, and rebuilds only what's needed.

```rust
struct SpaceTypeRegistryManager {
    wiki_root: PathBuf,
    registry: SpaceTypeRegistry,
}

impl SpaceTypeRegistryManager {
    /// Build from wiki.toml + schemas/
    fn build(wiki_root: &Path) -> Result<Self>;

    /// Check if type registry has changed on disk
    fn has_changed(&self) -> Result<bool>;

    /// Rebuild entirely
    fn rebuild(&mut self) -> Result<RebuildReport>;

    /// Detect which types changed and rebuild only those
    fn refresh(&mut self) -> Result<RefreshReport>;

    /// Get the current registry (read-only)
    fn registry(&self) -> &SpaceTypeRegistry;
}

struct RefreshReport {
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
    needs_full_index_rebuild: bool,
    needs_partial_index_rebuild: Vec<String>,
}
```

### Change detection

`has_changed()` recomputes `schema_hash` from current `wiki.toml` +
`schemas/` and compares with the stored hash.

### Refresh logic

`refresh()` compares per-type hashes to determine what changed:

1. Types in old but not new -> removed
2. Types in new but not old -> added
3. Types in both but hash differs -> changed
4. If added or removed -> `needs_full_index_rebuild = true`
5. If all types changed -> `needs_full_index_rebuild = true`
6. If some types changed -> `needs_partial_index_rebuild` = changed list

Only the affected `RegisteredType` entries are rebuilt (re-read schema,
re-compile validator, re-extract aliases/edges). Unchanged types keep
their compiled validators.

### Called by EngineManager

`EngineManager.on_type_change()` calls `SpaceTypeRegistryManager.refresh()`,
reads the `RefreshReport`, and decides whether to do a full or partial
index rebuild. See [engine.md](engine.md).

## Build Sequence

1. Read `wiki.toml` → collect all `[types.*]` entries (name, schema path,
   description)
2. For each type:
   a. Load the JSON Schema file from `schemas/`
   b. Compile the validator via `jsonschema::validator_for()`
   c. Extract `x-index-aliases` from the schema
   d. Extract `x-graph-edges` from the schema
   e. Compute per-type hash from aliases + edges
3. Compute global `schema_hash` from all per-type hashes
4. Store in `SpaceTypeRegistry`

## Usage

### On ingest

```
1. Read page's `type` field (default: "page")
2. Look up type in registry → get RegisteredType
3. Fall back to "default" if type not found
4. validator.validate(frontmatter) → accept or reject
5. aliases → resolve field names for indexing
6. edges → resolve graph edges for indexing
```

No file I/O, no schema parsing — everything is pre-compiled.

### On search / list / graph

The registry is not needed — these operate on the tantivy index. The
graph builder reads edge declarations from the registry only when
building petgraph from the index.

## Lifecycle

### llm-wiki serve

Built once at startup. Lives for the process lifetime. If `wiki.toml`
or `schemas/` change on disk, the server doesn't detect it
automatically — run `llm-wiki index rebuild` or restart.

### CLI commands

Each invocation:

1. Read `state.toml` → get stored `schema_hash`
2. Recompute hash from current `wiki.toml` + `schemas/`
3. Match → load cached `schema.json` → build registry from cache
4. Mismatch → rebuild registry from schema files → update cache

## Relationship to Index Schema

The tantivy `IndexSchema` is built from the `SpaceTypeRegistry`. The registry
knows all fields across all types (after alias resolution), their JSON
Schema types, and their edge declarations. The index schema is the
tantivy translation of that.

```
wiki.toml + schemas/
    → SpaceTypeRegistry (validators, aliases, edges)
        → IndexSchema (tantivy Schema, field handles)
            → tantivy Index
```

Both are rebuilt together when `schema_hash` changes. See
[tantivy.md](tantivy.md) for the `IndexSchema` struct and how fields
are classified.

## Cache File

The compiled registry metadata (not the validators — those can't be
serialized) is cached as `schema.json` at
`~/.llm-wiki/indexes/<name>/schema.json`.

Contains:
- Field set (name, tantivy type, options)
- Per-type aliases
- Per-type edge declarations

The `jsonschema::Validator` is rebuilt from the schema files when the
registry is loaded — compilation is fast (microseconds per schema).

## Invalidation

The registry is invalidated when `schema_hash` changes. See
[index-management.md](../specifications/engine/index-management.md)
for the change detection logic.

What triggers invalidation:
- Type added or removed in `wiki.toml`
- Type pointing to a different schema file
- `x-index-aliases` changed
- `x-graph-edges` changed

What does not:
- Page content changes
- Config changes outside `[types.*]`
- Schema changes that don't affect aliases or edges

## Crate

Use the `jsonschema` crate for validation:

```toml
jsonschema = "0.28"
```

Reference: https://docs.rs/jsonschema/latest/jsonschema/
