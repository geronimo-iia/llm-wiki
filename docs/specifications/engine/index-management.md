---
title: "Index Management"
summary: "Tantivy index — how fields are indexed, staleness, schema change detection, rebuild, and recovery."
read_when:
  - Understanding how the search index works
  - Understanding staleness detection and auto-recovery
  - Understanding incremental vs full rebuild
status: ready
last_updated: "2025-07-21"
---

# Index Management

The search index is a tantivy BM25 index stored at
`~/.llm-wiki/indexes/<name>/search-index/`. It is a local build
artifact — never committed, never shared. Rebuildable from committed
files at any time.

The index is the engine's core data structure. All of `wiki_search`,
`wiki_list`, and `wiki_graph` operate on the index. Only
`wiki_content_read` goes to disk.

- [Index Schema](#index-schema)
- [Incremental Update](#incremental-update)
- [Full Rebuild](#full-rebuild)
- [State Tracking](#state-tracking)
- [Schema Change Detection](#schema-change-detection)
- [Staleness Detection](#staleness-detection)
- [Auto-Recovery](#auto-recovery)
- [Pipeline Position](#pipeline-position)

## Index Schema

The index schema is derived from the type system. At ingest time, the
engine reads each page's type, loads the JSON Schema, applies
`x-index-aliases`, and indexes fields by role.

The computed schema is stored at
`~/.llm-wiki/indexes/<name>/schema.json` alongside the search index.
It is regenerated from the type registry on rebuild.

Three index roles:

| Role | Index type | How it's used |
|------|-----------|---------------|
| Text | BM25 tokenized | Full-text search ranking |
| Keyword | Exact match | Filtering (`--type`, `--status`) and graph edges |
| Stored | Not searched | Identifiers returned in results |

How frontmatter fields map to roles:

- **Base fields** (`title`, `summary`, `tags`, `type`, `status`,
  `owner`, `superseded_by`, `last_updated`) are indexed according to
  their type — strings as text, enums as keywords, lists of slugs as
  keyword per entry. See [types/base.md](../model/types/base.md).
- **Type-specific fields** (`read_when`, `tldr`, `sources`, `concepts`,
  `confidence`, `claims`, `document_refs`, etc.) are indexed the same
  way when present. See the individual type docs under
  [types/](../model/types/).
- **Aliased fields** (`name` -> `title`, `description` -> `summary`,
  etc.) are resolved before indexing. The index sees canonical names
  only. See [type-system.md](../model/type-system.md).
- **Unrecognized fields** (not in the schema) are indexed as generic
  text.
- **Body text** is indexed as BM25 text.
- **Slug** is `STRING | STORED | FAST` — stored for results, FAST for
  sorted pagination via `order_by_string_fast_field`.
- **Keyword fields** (`type`, `status`, `tags`) are `STRING | FAST` —
  FAST enables both exact-match filtering and facet counting.
- **URI** is stored but not searched.

The `slug` field is the unique key for delete+insert operations.

## Incremental Update

Collects changed `.md` files from two git diffs, merges into one set,
then does a single delete+insert pass:

```
A = working tree vs HEAD           (uncommitted changes on disk)
B = state.toml.commit vs HEAD      (commits since last index update)

changed = A union B, deduplicated by path

for each changed path:
    delete_term(slug)
    if file still exists on disk:
        parse frontmatter + body -> add_document()
writer.commit()
```

**Why two diffs:** A catches uncommitted changes (ingest writes before
committing). B catches committed changes since last index update
(external commits, prior ingests with `auto_commit`).

Cost: O(k) where k = changed pages.

Triggered by: `wiki_ingest`.

## Full Rebuild

Drops all documents and re-indexes the entire wiki tree:

```
delete_all_documents()
walk wiki/ -> parse each .md -> add_document()
writer.commit()
update state.toml
```

Cost: O(n) where n = total pages.

Triggered by:
- `llm-wiki index rebuild` (explicit)
- First index creation
- Index corruption (auto-recovery)
- Schema hash mismatch (type registry changed)
- Incremental update failure (fallback)

## State Tracking

Stored at `~/.llm-wiki/indexes/<name>/state.toml`:

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

| Field | Type | Description |
|-------|------|-------------|
| `schema_hash` | string | SHA-256 of all per-type hashes combined (sorted by type name) |
| `commit` | string | Git HEAD at time of last complete index update |
| `pages` | integer | Total pages indexed |
| `sections` | integer | Section pages indexed |
| `built` | string | ISO 8601 datetime of last build |
| `[types]` | table | Per-type SHA-256 of `schema_path` + `x-index-aliases` + file content hash |

Missing or malformed `state.toml` is treated as "never built" — the
index is stale.

See [engine-state.md](engine-state.md) for the full engine state layout.

## Schema Change Detection

The engine detects type registry changes by comparing hashes of the
schema file content on disk against the hashes stored in `state.toml`
at last build time.

Two functions compute hashes:

- **`compute_hashes` (build time)** — called when building the type
  registry. Hashes `schema_path` + sorted `x-index-aliases` +
  SHA-256 of file content per type. Stored in `state.toml` after
  rebuild.
- **`compute_disk_hashes` (staleness check)** — reads schema files
  directly from disk without building a full registry. Same algorithm,
  same output. Called by `index_status` and at engine startup.

Algorithm per type:

```
type_hash = SHA-256(schema_path + sorted_aliases + content_hash)
```

Global hash:

```
schema_hash = SHA-256(all type_hashes sorted by type name)
```

Where `content_hash = SHA-256(schema file bytes)`.

Inputs considered:

1. All `schemas/*.json` files (sorted by filename)
2. All `[types.*]` override entries from `wiki.toml`
3. For each type: the schema file path, `x-index-aliases`, and the
   full file content (which includes `x-graph-edges`, properties, etc.)
4. The embedded `base.json` fallback if no `default` type is declared

Because the full file content is hashed, any change to a schema file
— adding properties, modifying `x-graph-edges`, changing validation
rules — triggers a hash mismatch.

On every ingest or search/list, the engine recomputes the hashes from
the current `schemas/` + `wiki.toml` overrides and compares with stored
values.

### When the global hash mismatches

A full rebuild is triggered. Per-type hashes in `state.toml` enable
future partial rebuilds (re-index only pages of changed types) but
currently any mismatch triggers a full rebuild.

### What triggers a mismatch

- Schema file added, removed, or modified in `schemas/`
- `[types.*]` override added, removed, or changed in `wiki.toml`
- Any content change in a schema file (properties, aliases, graph
  edges, validation rules, descriptions)

### What does not trigger a mismatch

- Page content changes (handled by incremental update via git diff)
- Config changes (`ingest.auto_commit`, etc.)
- `wiki.toml` changes outside `[types.*]` (name, description, settings)

## Staleness Detection

| Condition | Stale? |
|-----------|--------|
| `commit == HEAD` and `schema_hash` matches | No |
| `commit != HEAD` | Yes |
| `schema_hash` mismatch | Yes (full rebuild needed) |
| `state.toml` missing | Yes (never built) |
| `state.toml` malformed | Yes (treated as missing) |

## Auto-Recovery

### Staleness (`index.auto_rebuild`)

- `true` -> rebuild silently before search/list
- `false` (default) -> warn, continue with stale index

### Corruption (`index.auto_recovery`)

When `Index::open()` fails:

- `true` (default) -> rebuild, retry open, continue
- `false` -> error propagated

Recovery is attempted once. If rebuild produces a corrupt index, the
error propagates.

Both `index.*` keys are global-only. See
[global-config.md](../model/global-config.md).

## Pipeline Position

In the ingest pipeline, the index update runs after validation and
before the optional git commit:

```
validate -> alias -> update_index -> commit (if auto_commit)
```

See [ingest-pipeline.md](ingest-pipeline.md).
