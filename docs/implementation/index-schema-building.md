---
title: "Index Schema Building"
summary: "How the tantivy schema is derived from type schemas — field collection, classification, and document building."
status: ready
last_updated: "2025-07-18"
---

# Index Schema Building

How the tantivy `IndexSchema` is derived from the JSON Schema files in
`schemas/`. This is a one-pass construction at startup or rebuild —
the schema files are read, fields are classified, and the tantivy
schema is built. No schema file content is kept in memory after
construction.

For the tantivy-specific details (tokenizers, collectors, writers),
see [tantivy.md](tantivy.md). For the type registry that feeds this
process, see [type-registry.md](type-registry.md).

## Overview

```
schemas/*.json  ──read once──►  classify fields  ──►  tantivy Schema
                                                          │
                                                     IndexSchema
                                                     (Schema + field handles)
```

The `IndexSchema` is the bridge between the type system and tantivy.
It holds the compiled tantivy `Schema` and a `HashMap<String, Field>`
for fast field handle lookup during document building.

The type registry (`SpaceTypeRegistry`) is not involved in this
process. The index schema builder reads the schema files directly —
the registry holds validators and aliases for ingest, the index schema
holds tantivy field handles for indexing. They are built from the same
source files but serve different purposes.

## Algorithm

### Input

- `schemas/` directory in the wiki repository (or embedded defaults)
- `wiki.toml` `[types.*]` overrides (for remapped schema paths)
- Tokenizer name from config (default: `en_stem`)

### Step 1: Collect schema files

Build the set of schema files to read:

1. Scan `schemas/*.json` — collect all file paths
2. Read `[types.*]` from `wiki.toml` — collect any override schema
   paths that point to files not already in the set
3. Deduplicate by file path — each schema file is read once

If `schemas/` is missing, use the embedded default schemas.

### Step 2: Extract fields from each schema file

For each schema file, read the JSON and extract:

- `properties` — the field definitions
- `x-index-aliases` — field name remapping
- `x-graph-edges` — fields that hold slug lists (Phase 3, but read
  now for classification)

For each property in `properties`:
1. Check if the field name has an alias in `x-index-aliases`. If so,
   **skip this property** — it will be indexed under its canonical
   name (e.g. `name` → `title`), which is already collected from
   another schema. The alias resolution itself happens at ingest time
   (the type registry's job), not at schema building time.
2. If no alias, check if the field name appears in `x-graph-edges`.
   If so, mark it as a slug field.
3. Record `(field_name, json_schema_type, is_slug_field)`

Collect across all schema files. Deduplicate by field name — if two
schemas define the same field (e.g. `title` appears in `base.json`,
`concept.json`, `paper.json`), the first classification wins. In
practice our schemas are consistent — `title` is always
`{"type": "string"}` everywhere.

Aliased properties (those listed as keys in `x-index-aliases`) are
never added to the field set. They don't exist in the index — only
their canonical targets do. The type registry resolves aliases at
ingest time: when indexing a skill page, the registry maps `name` →
`title` before the value reaches the index.

### Step 3: Classify fields

Each collected field is classified into a tantivy field type based on
its JSON Schema definition:

| JSON Schema pattern                              | Tantivy type       | Role                                       |
| ------------------------------------------------ | ------------------ | ------------------------------------------ |
| `{"type": "string"}`                             | `TEXT \| STORED`   | BM25 searchable text                       |
| `{"type": "string", "enum": [...]}`              | `STRING \| STORED` | Keyword filter (type, status, confidence)  |
| `{"type": "string", "const": ...}`               | `STRING \| STORED` | Keyword filter                             |
| `{"type": "string"}` + in `x-graph-edges`        | `STRING \| STORED` | Slug keyword (superseded_by)               |
| `{"type": "array", "items": {"type": "string"}}` | `TEXT \| STORED`   | Tokenized list (tags, read_when)           |
| `{"type": "array", ...}` + in `x-graph-edges`    | `STRING \| STORED` | Slug keyword per entry (sources, concepts) |
| `{"type": "boolean"}`                            | `STRING \| STORED` | Keyword ("true"/"false")                   |
| `{"type": "object"}`                             | `TEXT \| STORED`   | Serialized as JSON string                  |
| `{"type": "array", "items": {"type": "object"}}` | `TEXT \| STORED`   | Serialized as JSON string (claims)         |
| `{"oneOf": [...]}`                               | `TEXT \| STORED`   | Polymorphic, treat as text                 |

The `x-graph-edges` check is the key distinction between text arrays
(tags, read_when) and slug arrays (sources, concepts). Without it,
they have the same JSON Schema shape. Reading `x-graph-edges` for
classification only does not implement graph edge building (Phase 3)
— it just tells us which array fields hold slugs.

### Step 4: Add fixed fields

Always added regardless of schema content:

| Field        | Tantivy type       | Purpose                                   |
| ------------ | ------------------ | ----------------------------------------- |
| `slug`       | `STRING \| STORED` | Unique key for delete+insert              |
| `uri`        | `STRING \| STORED` | wiki:// URI for results                   |
| `body`       | `TEXT \| STORED`   | Page body, BM25 searchable                |
| `body_links` | `STRING \| STORED` | Multi-valued keyword for `[[wiki-links]]` |

### Step 5: Build tantivy schema

Feed all classified fields into `tantivy::Schema::builder()`. Store
the resulting `Schema` and `HashMap<String, Field>` in `IndexSchema`.

## Default Field Mapping

The following table shows how fields from the embedded schemas map to
tantivy index fields. This is the result of running the classification
algorithm on the 6 default schema files.

### Fixed fields (always present)

| Field        | Tantivy type | Source                     |
| ------------ | ------------ | -------------------------- |
| `slug`       | KEYWORD      | Engine-generated           |
| `uri`        | KEYWORD      | Engine-generated           |
| `body`       | TEXT         | Page body content          |
| `body_links` | KEYWORD      | Extracted `[[wiki-links]]` |

### From base.json (shared by all types)

| Field           | Tantivy type | Rationale                                                                      |
| --------------- | ------------ | ------------------------------------------------------------------------------ |
| `title`         | TEXT         | BM25 searchable                                                                |
| `type`          | KEYWORD      | `enum` → filterable                                                            |
| `summary`       | TEXT         | BM25 searchable                                                                |
| `status`        | KEYWORD      | `enum` → filterable                                                            |
| `last_updated`  | TEXT         | String without enum                                                            |
| `tags`          | TEXT         | Array of strings, no edges → tokenized                                         |
| `owner`         | TEXT         | String without enum                                                            |
| `superseded_by` | TEXT         | String without enum (becomes KEYWORD when `x-graph-edges` is added in Phase 3) |

### From concept.json (concept, query-result)

| Field        | Tantivy type | Rationale                                                   |
| ------------ | ------------ | ----------------------------------------------------------- |
| `read_when`  | TEXT         | Array of strings, no edges → tokenized                      |
| `tldr`       | TEXT         | String without enum                                         |
| `sources`    | TEXT         | Array of strings, no edges yet (becomes KEYWORD in Phase 3) |
| `concepts`   | TEXT         | Array of strings, no edges yet (becomes KEYWORD in Phase 3) |
| `confidence` | KEYWORD      | `enum: [high, medium, low]`                                 |
| `claims`     | TEXT         | Array of objects → serialized as JSON                       |

### From skill.json

| Field                      | Tantivy type | Rationale                      |
| -------------------------- | ------------ | ------------------------------ |
| `argument-hint`            | TEXT         | String                         |
| `paths`                    | TEXT         | oneOf (polymorphic)            |
| `disable-model-invocation` | KEYWORD      | Boolean                        |
| `user-invocable`           | KEYWORD      | Boolean                        |
| `allowed-tools`            | TEXT         | oneOf (polymorphic)            |
| `context`                  | TEXT         | String                         |
| `agent`                    | TEXT         | String                         |
| `model`                    | TEXT         | String                         |
| `effort`                   | KEYWORD      | `enum`                         |
| `shell`                    | KEYWORD      | `enum`                         |
| `hooks`                    | TEXT         | Object → serialized            |
| `document_refs`            | TEXT         | Array of strings, no edges yet |
| `compatibility`            | TEXT         | String                         |
| `license`                  | TEXT         | String                         |
| `metadata`                 | TEXT         | Object → serialized            |

Note: `name`, `description`, `when_to_use` from skill.json are
**skipped** (aliased to `title`, `summary`, `read_when`).

### From doc.json

No additional fields beyond base.json — `read_when` and `sources`
already collected from concept.json.

### From section.json

No additional fields beyond base.json.

### Phase 3 changes

When `x-graph-edges` is added to the schemas, the following fields
will change from TEXT to KEYWORD:
- `sources` → KEYWORD (slug per entry, `fed-by`/`cites` edges)
- `concepts` → KEYWORD (slug per entry, `depends-on`/`informs` edges)
- `superseded_by` → KEYWORD (slug, `superseded-by` edge)
- `document_refs` → KEYWORD (slug per entry, `documented-by` edge)

## The IndexSchema struct

```rust
pub struct IndexSchema {
    pub schema: tantivy::Schema,
    pub fields: HashMap<String, Field>,
}
```

No aliases, no edges, no raw JSON — just the tantivy schema and field
handles. The aliases live in the type registry (used at ingest time to
resolve field names before indexing). The edges live in the type
registry (used at graph build time).

## Document building

When indexing a page, the caller (search.rs `build_doc`) does:

1. Get the page's type from frontmatter
2. Get the aliases for that type from the type registry
3. For each frontmatter field:
   a. Apply alias if one exists (e.g. `name` → `title`)
   b. Look up the canonical field name in `IndexSchema.fields`
   c. If found → add the value to the tantivy document
   d. If not found → index as body text (unrecognized field)
4. Add fixed fields: slug, uri, body, body_links

This means `build_doc` needs both the `IndexSchema` (for field
handles) and the type registry (for aliases). The index schema does
not store aliases — it only knows field names and their tantivy types.

## Backward compatibility

`IndexSchema::build(tokenizer)` (no registry) continues to work with
the current hardcoded fields. This is used when no wiki is mounted.

`IndexSchema::build_from_schemas(schemas_dir, wiki_toml_types, tokenizer)`
is the new constructor that reads schema files and classifies fields.

Both produce the same struct — the difference is how the field set is
determined.

## What is NOT stored in IndexSchema

- Raw JSON Schema content — read once during construction, discarded
- Aliases — live in SpaceTypeRegistry, used at ingest time
- Edge declarations — live in SpaceTypeRegistry, used at graph time
- Validators — live in SpaceTypeRegistry, used at ingest time
- Type descriptions — live in SpaceTypeRegistry, used by CLI/MCP

The index schema is a thin tantivy wrapper. All type intelligence
lives in the registry.
