# Schema-Driven Type Discovery

## Decision

Types are discovered automatically from `schemas/*.json` via
`x-wiki-types`. `[types.*]` entries in `wiki.toml` are optional
overrides, not the primary registry.

Supersedes the "type registry in `wiki.toml`" part of
[schema-md-eliminated](schema-md-eliminated.md).

## Context

The initial design (schema-md-eliminated) moved the type registry from
`schema.md` to `wiki.toml` `[types.*]`. This worked but created
redundancy: every type had to be declared in both a JSON Schema file
(for validation) and in `wiki.toml` (for the engine to find it).

For the 15 default types, `spaces create` generated 15 `[types.*]`
entries in `wiki.toml` — all pointing to 6 schema files. The schema
files already knew which types they served. The `wiki.toml` entries
were just a lookup table duplicating information the schemas contained.

## Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| `wiki.toml` as primary registry | Explicit, one file to read | Redundant with schemas, verbose for defaults |
| Schema discovery only, no overrides | Simplest, zero config | Can't remap a type to a different schema |
| Schema discovery + `wiki.toml` overrides | Clean defaults, flexible overrides | Two sources to merge |

## How It Works

Each JSON Schema declares which types it serves:

```json
"x-wiki-types": {
  "paper": "Academic source — research papers, preprints",
  "article": "Editorial source — blog posts, news, essays"
}
```

The engine builds the type registry by:

1. Scanning `schemas/*.json` in the wiki repository
2. Reading `x-wiki-types` from each schema
3. Reading `[types.*]` from `wiki.toml` (if any)
4. Merging: `wiki.toml` entries override discovered entries

The type named `default` (from `base.json`) is the fallback for pages
with an unrecognized or missing `type` field.

## Consequences

- `spaces create` writes only schema files and a minimal `wiki.toml`
  (name + description). No `[types.*]` entries needed.
- Adding a custom type = drop a schema file with `x-wiki-types` into
  `schemas/`. No `wiki.toml` edit required.
- Overriding a type = add a `[types.*]` entry in `wiki.toml` pointing
  to a different schema file.
- The schema files are self-describing — `x-wiki-types` for type
  discovery, `x-index-aliases` for field aliasing, `x-graph-edges`
  for graph edges (Phase 3).
- `wiki.toml` stays clean in the common case.
- Backward compatible — existing wikis with `[types.*]` entries
  continue to work (overrides take precedence).
