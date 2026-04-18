# Dynamic Index Schema

## Decision

The tantivy index schema is not hardcoded. It's computed at runtime as
the union of all fields across all registered types, after alias
resolution.

## Context

With a static type system, you can define the tantivy schema as a fixed
set of fields at compile time. With a dynamic type system where wiki
owners define custom types with arbitrary fields, the schema must be
derived from the type registry.

## Alternatives Considered

| Approach                           | Why not                                                                    |
| ---------------------------------- | -------------------------------------------------------------------------- |
| Fixed schema with catch-all fields | Custom type fields lose their identity — everything goes into `extra_text` |
| One index per type                 | Cross-type search requires querying multiple indexes and merging results   |
| No custom fields in index          | Custom type fields become unsearchable                                     |

## How It Works

1. Read all type schemas from `wiki.toml` + `schemas/`
2. Resolve `x-index-aliases` (e.g. `name` → `title`)
3. Collect every field name across all types
4. Classify by JSON Schema type → tantivy field type
5. Build the tantivy schema from that union

The computed schema is cached as `schema.json`. A `schema_hash` in
`state.toml` detects when the type registry changes and triggers a
rebuild.

## Consequences

- Adding a custom type with new fields automatically extends the index
- `schema_hash` change detection replaces manual version bumping
- Per-type hashes enable partial rebuilds (only re-index affected pages)
- The `schema.json` cache avoids re-deriving the schema on every CLI
  invocation
