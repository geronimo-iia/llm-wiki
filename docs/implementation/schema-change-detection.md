---
title: "Schema Change Detection"
summary: "How the engine detects type registry changes and triggers index rebuilds."
status: ready
last_updated: "2025-07-19"
---

# Schema Change Detection

For the spec, see
[index-management.md](../specifications/engine/index-management.md).

## Per-Wiki Type Registry

Each wiki has its own `schemas/` directory and `wiki.toml`. The engine
builds a `SpaceTypeRegistry` and `IndexSchema` per wiki via
`build_space()` in `space_builder.rs`.

```
EngineState {
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

## Hash Computation

Two functions compute hashes using SHA-256:

### `compute_hashes` (build time)

Called inside `build_space()` when constructing the type registry.
Each `RegisteredType` carries a `content_hash` (SHA-256 of the schema
file bytes, computed at parse time).

Per-type hash:

```
type_hash = SHA-256(schema_path + sorted_aliases + content_hash)
```

Global hash:

```
schema_hash = SHA-256(all type_hashes sorted by type name)
```

The result is stored in `SpaceTypeRegistry` and written to
`state.toml` after rebuild.

### `compute_disk_hashes` (staleness check)

A standalone function that reads schema files directly from disk
without building a full registry. Same algorithm, same output.

```rust
pub fn compute_disk_hashes(repo_root: &Path) -> Result<(String, HashMap<String, String>)>
```

Algorithm:
1. Scan `schemas/*.json` (sorted by filename)
2. For each file: SHA-256 of content, read `x-wiki-types` to map
   type names
3. Apply `wiki.toml` `[types.*]` overrides
4. Ensure `default` type exists (embedded fallback if needed)
5. Per-type hash = SHA-256(schema_path + sorted aliases + content hash)
6. Global hash = SHA-256(all per-type hashes sorted by type name)

Called by `indexing::index_status()` — no registry or engine state
needed.

## state.toml

```toml
schema_hash = "a1b2c3d4e5f6..."   # 64-char hex (SHA-256)
commit      = "a3f9c12..."
pages       = 142
sections    = 8
built       = "2025-07-17T14:32:01Z"

[types]
concept  = "e5f6a7b8..."           # 64-char hex (SHA-256)
paper    = "c9d0e1f2..."
skill    = "3a4b5c6d..."
```

## Staleness

```rust
let (current_hash, _) = compute_disk_hashes(repo_root)?;
stale = (state.commit != HEAD) || (state.schema_hash != current_hash)
```

Missing or malformed `state.toml` is treated as "never built".

Old `state.toml` files with non-SHA-256 hashes (from the previous
`DefaultHasher` implementation) will always mismatch, triggering a
full rebuild on first run. This is expected and correct.

## Startup Sequence Per Wiki

```
1. build_space(repo_root, tokenizer) → (type_registry, index_schema)
2. index_status() → calls compute_disk_hashes() internally
   - Missing state.toml → full rebuild
   - schema_hash mismatch → full rebuild
   - commit != HEAD → stale (rebuild if auto_rebuild)
   - All match → current
3. Store SpaceState { type_registry, index_schema, ... }
```

## What Triggers a Rebuild

| Change | Detected by | Action |
|--------|------------|--------|
| Schema file added/removed/modified | `schema_hash` mismatch | Full rebuild |
| `wiki.toml` `[types.*]` changed | `schema_hash` mismatch | Full rebuild |
| New commit (content change) | `commit` mismatch | Incremental update |
| `state.toml` missing/malformed | Parse failure | Full rebuild |

Because the full file content is hashed, any change to a schema file
— adding properties, modifying `x-graph-edges`, changing validation
rules — triggers a hash mismatch.

## Partial Rebuild (future)

Per-type hashes are stored in `state.toml` but not compared yet. Any
`schema_hash` mismatch triggers a full rebuild.
