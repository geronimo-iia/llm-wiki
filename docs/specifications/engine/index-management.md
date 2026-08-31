---
title: "Index Management"
summary: "Tantivy index — how fields are indexed, staleness, schema change detection, rebuild, and recovery."
read_when:
  - Understanding how the search index works
  - Understanding staleness detection and auto-recovery
  - Understanding incremental vs full rebuild
status: ready
last_updated: "2026-08-19"
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
  keyword per entry. Arrays with `"x-keyword": true` are stored as one
  keyword value per entry with values lowercased at index time (`tags`
  uses this). See [types/base.md](../model/types/base.md).
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
- **Keyword fields** (`type`, `status`, `tags`) are `STRING | STORED | FAST` —
  STORED for result output, FAST enables both exact-match filtering and facet
  counting via `StrColumn`. `type` was historically `TEXT | STORED` due to a
  bug in `classify_field`'s string arm; it is now `STRING | STORED | FAST`.
- **`last_updated`** is `STRING | STORED | FAST` (`"x-keyword": true` in
  `base.json`). ISO 8601 date strings are atomic tokens — keyword storage is
  semantically correct and avoids tokenization. FAST enables `StalenessCollector`
  to read dates via `StrColumn` with zero `searcher.doc()` calls. `title` was
  evaluated for the same promotion but rejected: it is in the `QueryParser`
  field list and keyword indexing would break word-level full-text title search.
- **Numeric fields** (`confidence`) are `f64 | FAST | STORED` — stored
  for result output, FAST for per-document score access inside the
  `tweak_score` collector. `confidence` is written via the dedicated
  `frontmatter::confidence()` getter (not the generic text path), so
  legacy string values (`"high"` → 0.9, `"medium"` → 0.5, `"low"` → 0.2)
  are normalised to floats at index time.
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

Builds a new index in an isolated temp directory, then promotes it via atomic
renames. The live index is untouched until `commit()` succeeds.

```
wipe search-index-building/            (handles crash leftovers from prior run)
create search-index-building/
open Index in search-index-building/
walk wiki/ -> parse each .md -> add_document()
writer.commit()

// close open handles before rename (required on Windows)
inner.write():
    inner.tantivy_index = None
    inner.index_reader  = None
// mmap handles released; rename now safe on all platforms

// three-rename atomic swap
search-index/          -> search-index-prev/
search-index-building/ -> search-index/

// open fresh index and reader on the new live directory
open Index from new search-index/
create IndexReader (ReloadPolicy::Manual)
  ok  -> inner.write(): set tantivy_index, index_reader
         rm -rf search-index-prev/
         update state.toml
  err -> search-index/      -> search-index-building/   (rollback)
         search-index-prev/ -> search-index/             (rollback)
         return error (fatal — previous index restored)
```

**Windows note:** `fs::rename` on a directory fails with os error 5 (Access
Denied) if any memory-mapped file is open inside it. Clearing `tantivy_index`
and `index_reader` from `IndexInner` before Phase 1 releases all mmap handles.
After Phase 2, a fresh `Index` and `IndexReader` are opened — `reload_reader()`
cannot be used because there is no live reader to reload at that point.

Cost: O(n) where n = total pages.

Concurrent rebuild calls on the same space are serialised by
`SpaceIndexManager.rebuild_lock: Mutex<()>`. The second caller blocks until the
first completes, then runs a fresh rebuild on the now-updated index. See
[lock-patterns.md](../../implementation/lock-patterns.md) for details.

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

## IndexReader Lifecycle

The `IndexReader` is created once per wiki space in `SpaceIndexManager::open()`
and held for the engine's lifetime. All search operations call
`index_manager.searcher()` which is a cheap arc-clone of the current segment set.

**All readers use `ReloadPolicy::Manual`.** The tantivy default
(`OnCommitWithDelay`) spawns a file_watcher thread. If a second reader is opened
on the same directory (e.g. the health check in `status()`), the two watchers
compete on `meta.json` writes and loop infinitely. `Manual` skips the watcher;
the reader is refreshed internally by `writer.commit()`.

This applies to every reader in the codebase — both the long-lived reader in
`open()` and the temporary reader in `status()`.

## Ingest Config in Rebuild and Update

`rebuild`, `update`, and `rebuild_types` each accept `ingest_config: &IngestConfig` as a final parameter. The config is threaded into every WalkDir pass so exclusion and frontmatter filters are applied uniformly.

A `should_index(slug, content, config, exclude)` helper encapsulates both filters:

1. **Glob exclusion** — slug is matched against each pattern in `ingest.exclude`; a match skips the file.
2. **No-frontmatter** — when `ingest.skip_no_frontmatter` is `true` (default), any `.md` file whose content has no `---` YAML frontmatter block is skipped.

`should_index` is called at every WalkDir entry before parsing. Files that do not pass are silently skipped (not counted as errors).

The `open()` recovery tuple is `Option<(&Path, &Path, &SpaceTypeRegistry, &IngestConfig)>` — `IngestConfig` is the fourth element, forwarded to the rebuild triggered on corruption.

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
