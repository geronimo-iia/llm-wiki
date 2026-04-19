---
title: "Type Registry Implementation"
summary: "How types are discovered from schemas, compiled, cached, and invalidated at runtime."
status: draft
last_updated: "2025-07-18"
---

# Type Registry Implementation

Implementation reference for the type registry. Not a specification —
see [type-system.md](../specifications/model/type-system.md) for the
design.

## Overview

The type registry is an in-memory cache of compiled validators and
metadata for all registered types. Built once at startup, used on every
ingest, invalidated when schema files change.

Types are discovered from `schemas/*.json` via `x-wiki-types`, with
optional `[types.*]` entries in `wiki.toml` as overrides. See
[schema-driven-types](../decisions/schema-driven-types.md) for the
rationale.

## Core Structs

```rust
/// Per-wiki type registry
pub struct SpaceTypeRegistry {
    /// type name → compiled type
    types: HashMap<String, RegisteredType>,
    /// SHA-256 hash of all type inputs (for change detection)
    schema_hash: String,
    /// per-type hashes (for partial rebuild)
    type_hashes: HashMap<String, String>,
}

pub struct RegisteredType {
    /// Path to the schema file (relative to repo root)
    schema_path: String,
    /// Human-readable description
    description: String,
    /// compiled JSON Schema validator — no re-parsing on each ingest
    validator: jsonschema::Validator,
    /// x-index-aliases: source field → canonical field
    aliases: HashMap<String, String>,
    /// x-graph-edges: field → (relation, direction, target_types)
    edges: Vec<EdgeDecl>,
}

pub struct EdgeDecl {
    pub field: String,
    pub relation: String,
    pub direction: String,
    pub target_types: Option<Vec<String>>,
}
```

## Build Sequence

1. Scan `schemas/*.json` in the wiki repository
2. For each schema file:
   a. Parse the JSON Schema
   b. Read `x-wiki-types` → collect `(type_name, description)` pairs
   c. Extract `x-index-aliases`
   d. Extract `x-graph-edges` (Phase 3)
   e. Compile the validator via `jsonschema::Validator::new()`
   f. For each type declared in `x-wiki-types`, create a
      `RegisteredType` sharing the same compiled validator
3. Read `[types.*]` from `wiki.toml` (if any)
4. For each `wiki.toml` override:
   a. Load the referenced schema file
   b. Compile validator, extract aliases/edges
   c. Replace or add the entry in the registry
5. Compute per-type hashes and global `schema_hash`
6. Store in `SpaceTypeRegistry`

### Fallback behavior

- If `schemas/` directory is missing → use embedded default schemas
  (backward compat with Phase 1 wikis)
- The type named `default` is the fallback for pages with an
  unrecognized or missing `type` field
- If no `default` type is discovered → use embedded `base.json`

### Base schema invariant

The `default` type is critical — every unknown type falls back to it.
The engine enforces at `build()` time:

1. A `default` type must exist after discovery + overrides. If no
   schema declares it, the embedded `base.json` is injected.
2. If a custom `base.json` exists on disk, it must:
   - Declare `default` in `x-wiki-types`
   - Require at least `title` and `type` in its `required` array
3. Violation → `build()` returns an error with a clear message.

This prevents a wiki from accidentally breaking all validation by
shipping an incompatible base schema.

### Validator sharing

Multiple types can share a single compiled validator (e.g., `paper`,
`article`, `documentation` all use `paper.json`). The validator is
compiled once per schema file, then referenced by each type.

## Usage

### On ingest

```
1. Read page's `type` field (default: "page")
2. Look up type in registry → get RegisteredType
3. Fall back to "default" if type not found
4. validator.validate(frontmatter) → accept or reject
5. aliases → resolve field names for indexing
6. edges → resolve graph edges for indexing (Phase 3)
```

No file I/O, no schema parsing — everything is pre-compiled.

### On search / list / graph

The registry is not needed — these operate on the tantivy index. The
graph builder reads edge declarations from the registry only when
building petgraph from the index.

## Lifecycle

### llm-wiki serve

Built once at startup. Lives for the process lifetime. If `schemas/`
files change on disk, the server doesn't detect it automatically —
run `llm-wiki index rebuild` or restart.

### CLI commands

Each invocation:

1. Read `state.toml` → get stored `schema_hash`
2. Recompute hash from current `schemas/` + `wiki.toml` overrides
3. Match → use cached registry
4. Mismatch → rebuild registry from schema files → update cache

## Schema Hash

The `schema_hash` is a SHA-256 of all inputs that affect the type
registry:

- Contents of each `schemas/*.json` file (sorted by filename)
- Contents of `[types.*]` entries from `wiki.toml` (sorted by type name)

Per-type hashes cover the subset relevant to each type:
- The schema file content
- The `x-index-aliases` mapping
- The `x-graph-edges` declarations (Phase 3)

### What triggers invalidation

- Schema file added, removed, or modified in `schemas/`
- `[types.*]` entry added, removed, or changed in `wiki.toml`
- `x-index-aliases` changed in a schema
- `x-graph-edges` changed in a schema

### What does not trigger invalidation

- Page content changes (handled by incremental update via git diff)
- Config changes outside `[types.*]` in `wiki.toml`
- Schema changes that don't affect validation, aliases, or edges

## Relationship to Index Schema

The tantivy `IndexSchema` is built from the `SpaceTypeRegistry`:

```
schemas/*.json + wiki.toml overrides
    → SpaceTypeRegistry (validators, aliases, edges)
        → IndexSchema (tantivy Schema, field handles)
            → tantivy Index
```

Both are rebuilt together when `schema_hash` changes. See
[tantivy.md](tantivy.md) for the `IndexSchema` struct.

## Crate

```toml
jsonschema = "0.28"
```

Reference: https://docs.rs/jsonschema/latest/jsonschema/
