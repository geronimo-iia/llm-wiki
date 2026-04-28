---
title: "Tantivy Implementation Notes"
summary: "Tantivy-specific implementation details — dynamic schema, TopDocs, index writer, tokenizer, and segment management."
status: ready
last_updated: "2026-04-28"
---

# Tantivy Implementation Notes

Implementation reference for working with tantivy in llm-wiki. Not a
specification — see [index-management.md](../specifications/engine/index-management.md)
for the design.

## Dynamic Schema Building

The tantivy schema is dynamic — it's the union of all fields across all
registered types. When a type adds a field that doesn't exist yet (e.g.
`document_refs` on skill, `allowed-tools` on skill, `attendees` on a
custom meeting-notes type), it becomes a new tantivy field.

### How it works

1. Read all type schemas from `wiki.toml` + `schemas/`
2. For each type, resolve `x-index-aliases` (e.g. `name` → `title`)
3. Collect every field name across all types (after alias resolution)
4. Classify each by JSON Schema type:
   - `string` → `TEXT | STORED` (tokenized for BM25)
   - `string` with `enum` → `STRING | STORED | FAST` (keyword)
   - `array` of `string` → `STRING | STORED` multi-valued (keyword per entry)
   - `string` with `format: date` → `DATE | STORED | FAST`
   - `object` / `array` of `object` → `JSON | STORED` (stored, not searched)
5. Add fixed fields: `slug` (STRING | STORED | FAST), `uri` (STRING | STORED),
   `body` (TEXT)
6. Build the tantivy schema

### Core struct

```rust
struct IndexSchema {
    /// The tantivy schema — rebuilt when type registry changes
    schema: tantivy::Schema,

    /// Field name → tantivy Field handle (for fast document building)
    fields: HashMap<String, Field>,

    /// Type name → alias map (source field name → canonical field name)
    aliases: HashMap<String, HashMap<String, String>>,

    /// Type name → edge declarations (for graph building)
    edges: HashMap<String, Vec<EdgeDecl>>,
}
```

`fields` is dynamic — grows with the union of all type schemas.
`aliases` and `edges` are read from `x-index-aliases` and
`x-graph-edges` in each type's JSON Schema.

### Caching

The computed schema is stored as `schema.json` at
`~/.llm-wiki/indexes/<name>/schema.json`. CLI commands load it from
cache instead of re-deriving from all schema files. `schema_hash` in
`state.toml` detects when the cache is stale.

For `llm-wiki serve`, built once at startup, kept in memory.

### When the schema changes

Adding a type, removing a type, or changing a type's schema may change
the tantivy field set. `schema_hash` mismatch triggers a rebuild with
the new schema. See
[index-management.md](../specifications/engine/index-management.md)
for the change detection logic.

## Top K Collectors

`wiki_search` uses the `TopDocs` collector to return the best-scoring
documents by BM25 relevance.

```rust
use tantivy::collector::TopDocs;

let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;
```

`top_k` comes from `--top-k` flag or `defaults.search_top_k` config.

The collector returns `Vec<(Score, DocAddress)>` sorted by descending
score. Each `DocAddress` is then used to retrieve stored fields (slug,
uri, title, excerpt).

Reference: https://docs.rs/tantivy/latest/tantivy/collector/struct.TopDocs.html

### Combined with type filter

When `--type` is specified, combine BM25 with a term query on the
`type` keyword field using a `BooleanQuery`:

```rust
use tantivy::query::{BooleanQuery, Occur, TermQuery};

let bm25 = parser.parse_query(query_text)?;
let type_filter = TermQuery::new(
    Term::from_field_text(type_field, type_value),
    IndexRecordOption::Basic,
);
let combined = BooleanQuery::new(vec![
    (Occur::Must, Box::new(bm25)),
    (Occur::Must, Box::new(type_filter)),
]);
```

### Sorted pagination for list

`wiki_list` uses the `slug` field (STRING | STORED | FAST) for native
lexicographic pagination:

```rust
use tantivy::collector::{Count, TopDocs};
use tantivy::Order;

let total = searcher.search(&query, &Count)?;
let sorted = searcher.search(
    &query,
    &TopDocs::with_limit(offset + page_size)
        .order_by_string_fast_field("slug", Order::Asc),
)?;
// Extract full fields only for sorted[offset..]
```

Native string sort — no encoding, no tie-breaking needed.

## IndexReader and ReloadPolicy

The `IndexReader` is held in `SpaceIndexManager::inner.index_reader` for the
lifetime of the engine process. All `searcher()` calls are cheap arc-clones of
the current segment set from this single reader.

### ReloadPolicy::Manual

All readers in llm-wiki are created with `ReloadPolicy::Manual`:

```rust
index
    .reader_builder()
    .reload_policy(tantivy::ReloadPolicy::Manual)
    .try_into()?
```

**Why Manual, not OnCommitWithDelay (the tantivy default):**

`OnCommitWithDelay` spawns a background file_watcher thread that polls
`meta.json` for changes. When a second `Index::reader()` is opened on the same
directory (e.g. `status()` opening a temporary reader for health checks), two
watcher threads compete on the same file. Each reload writes a new `meta.json`,
which the other watcher detects, triggering another reload — an infinite loop
that deadlocks the process.

`Manual` skips the file_watcher entirely. The reader is refreshed explicitly
after a write by calling `reader.reload()?` — which happens automatically
inside `writer.commit()` via tantivy's internal bookkeeping.

For llm-wiki, `Manual` is always correct:
- **CLI commands** are one-shot; they never need live reload.
- **`llm-wiki serve`** rebuilds the full engine on `wiki_rebuild` —
  `WikiEngine::build()` creates a fresh `SpaceIndexManager` with a new reader.
- **`llm-wiki watch`** routes detected changes through the same rebuild path.

### Reader lifecycle

```
WikiEngine::build()
  └─ mount_space()
       ├─ index_manager.status()     ← Manual reader, temporary, dropped immediately
       ├─ index_manager.rebuild()    ← writer.commit() refreshes the live reader implicitly
       └─ index_manager.open()       ← creates the long-lived Manual reader in inner.index_reader
              └─ held until engine is dropped
```

Every `searcher()` call is `inner.index_reader.searcher()` — a cheap arc clone.

## Index Writer

The writer manages in-memory segments and flushes to disk.

```rust
let writer = index.writer(memory_budget)?;
```

`memory_budget` comes from `index.memory_budget_mb` config (default:
50 MB), converted to bytes. Tantivy flushes a segment when this
threshold is reached or when `writer.commit()` is called.

### Delete + Insert Pattern

Tantivy does not support in-place document updates. To update a page:

```rust
writer.delete_term(Term::from_field_text(slug_field, slug));
writer.add_document(new_doc)?;
writer.commit()?;
```

The `slug` field is the unique key — exact match, no tokenization.

## Document Type

Use the default `TantivyDocument` unless a real limitation is hit.
Our documents are simple — text fields, keyword fields, a date, stored
slugs. No need for custom `Document` trait implementation.

If we later need structured edge data stored in the index (instead of
schema lookup at graph-build time), a custom document type could avoid
JSON serialization overhead. Revisit then.

## Tokenizer

Configurable per wiki via `index.tokenizer` (default: `en_stem`).

Built-in tantivy tokenizers:

| Name      | Pipeline                                        | Use case                                        |
| --------- | ----------------------------------------------- | ----------------------------------------------- |
| `default` | SimpleTokenizer + RemoveLongFilter + LowerCaser | Basic                                           |
| `raw`     | No tokenization                                 | Keywords (used automatically for STRING fields) |
| `en_stem` | default + English stemmer                       | English knowledge bases                         |

`en_stem` is the right default — "scaling" matches "scale", "routing"
matches "route".

For non-English wikis, register a custom tokenizer and set
`index.tokenizer` in `wiki.toml`:

```rust
index.register_tokenizer("fr_stem", my_french_tokenizer);
```

The tokenizer applies to all text fields (title, summary, read_when,
tldr, body). Keyword fields always use `raw`.

Changing the tokenizer invalidates the `schema_hash` → full rebuild.

References:
- https://docs.rs/tantivy/latest/tantivy/tokenizer/index.html
- https://docs.rs/tantivy/latest/tantivy/tokenizer/index.html#custom-tokenizer-library

## Segment Management

Tantivy creates segments as documents are added. Over time, many small
segments accumulate. Tantivy's merge policy handles this automatically,
but for full rebuilds consider:

```rust
// After a full rebuild, wait for merges to complete
writer.commit()?;
writer.wait_merging_threads()?;
```

## Useful Links

- [tantivy docs](https://docs.rs/tantivy/latest/tantivy/)
- [TopDocs collector](https://docs.rs/tantivy/latest/tantivy/collector/struct.TopDocs.html)
- [Schema builder](https://docs.rs/tantivy/latest/tantivy/schema/struct.SchemaBuilder.html)
- [IndexWriter](https://docs.rs/tantivy/latest/tantivy/struct.IndexWriter.html)
- [BooleanQuery](https://docs.rs/tantivy/latest/tantivy/query/struct.BooleanQuery.html)
- [Document trait](https://docs.rs/tantivy/latest/tantivy/trait.Document.html)
