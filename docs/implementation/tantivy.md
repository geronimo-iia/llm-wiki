---
title: "Tantivy Implementation Notes"
summary: "Tantivy-specific implementation details — dynamic schema, TopDocs, index writer, tokenizer, and segment management."
status: ready
last_updated: "2026-08-19"
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
   - `string` with `enum` or `const` → `STRING | STORED | FAST` (keyword)
   - `string` with `"x-keyword": true` → `STRING | STORED | FAST` (keyword; values lowercased at index time)
   - `array` with `"x-keyword": true` → `STRING | STORED | FAST` per value (keyword per entry; values lowercased at index time)
   - `array` of `string` items with `enum`/`const` → `STRING | STORED | FAST` per value (keyword per entry)
   - `array` of plain `string` items (no `x-keyword`) → `TEXT | STORED` (joined and tokenized)
   - `number` / `integer` → `f64 | FAST | STORED`
   - `object` / `array` of `object` → serialized as text
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

## Fast-Field Facet Counting

`wiki_search` and `wiki_list` return facet counts (distributions over `type`,
`status`, `tags`). These are collected without touching the stored-doc store via
a custom `KeywordFacetCollector` that reads tantivy columnar fast fields.

### KeywordFacetCollector

Implements `tantivy::Collector`. In `collect()`, retrieves a `StrColumn` fast
field for the target field name, iterates term ordinals per doc, and resolves
each ordinal to a string via `ord_to_str`. Counts are accumulated in a
`HashMap<String, u64>`.

```rust
struct KeywordFacetCollector {
    field_name: String,
    top_n: usize,
}

// Segment-level state
struct KeywordFacetSegmentCollector {
    column: Option<tantivy::columnar::StrColumn>,
    buf: String,
    counts: HashMap<String, u64>,
}
```

`StrColumn` is available at `tantivy::columnar::StrColumn` (tantivy 0.26
re-exports the `columnar` crate). Fields must be `STRING | FAST` for
`fast_fields().str(name)` to return `Some`.

### MultiCollector

Multiple `KeywordFacetCollector` instances are combined with `MultiCollector`
to run all facet field passes over the same query in a single
`searcher.search()` call:

```rust
fn collect_facets(
    searcher: &Searcher,
    query: &dyn tantivy::query::Query,
    fields: &[(&str, usize)],
) -> Result<Vec<HashMap<String, u64>>>
```

### Search pass reduction

Using `MultiCollector` and bundling facet collectors with the main `TopDocs`
collector reduces tantivy segment passes:

| Function   | Before | After |
|------------|--------|-------|
| `search()` | 4      | 2     |
| `list()`   | 4      | 2     |

The `type` facet uses an unfiltered query (all docs in the space); it cannot be
folded into the main filtered pass and runs as a separate call to
`collect_facets`.

## FAST-Field Segment Collectors

Beyond facet counting, the `Collector`+`SegmentCollector` pattern is used
wherever stored-doc reads can be eliminated by reading FAST fields directly
during traversal.

### StalenessCollector

`compute_staleness` in `src/ops/stats.rs` uses a custom `StalenessCollector`
that reads the `last_updated` FAST field during the single `AllQuery` sweep —
zero `searcher.doc()` calls.

`last_updated` is `STRING | STORED | FAST` (`"x-keyword": true` in `base.json`).
In the segment collector, `fast_fields().str("last_updated")` returns
`Result<Option<StrColumn>>`; `term_ords(doc)` gives the ordinal iterator;
`ord_to_str(ord, &mut buf)` resolves each ordinal to the ISO 8601 string.

```rust
struct StalenessSegmentCollector {
    col: Option<tantivy::columnar::StrColumn>,
    buf: String,
    counts: StalenessCounters,
}

impl SegmentCollector for StalenessSegmentCollector {
    type Fruit = StalenessCounters;

    fn collect(&mut self, doc: tantivy::DocId, _score: tantivy::Score) {
        let Some(col) = &self.col else {
            self.counts.no_date += 1;
            return;
        };
        self.buf.clear();
        if let Some(ord) = col.term_ords(doc).next() {
            let _ = col.ord_to_str(ord, &mut self.buf);
        }
        // parse self.buf as NaiveDate and bucket into counts
    }

    fn harvest(self) -> StalenessCounters { self.counts }
}
```

`let _ = col.ord_to_str(...)` — the return value must be explicitly discarded
to satisfy `-D warnings`.

Fields must be `STRING | FAST`; TEXT fields return `None` from
`fast_fields().str()` silently.

### DocRecord shared pass in run_lint

`run_lint` in `src/ops/lint.rs` performs a single `AllQuery + DocSetCollector`
pass at the top of the function, reading all needed stored fields into a
`Vec<DocRecord>`. Each lint rule then iterates this vec with zero additional
tantivy calls.

```rust
struct DocRecord {
    slug: String,
    doc_type: String,
    last_updated: Option<String>,
    confidence: Option<f64>,
    confidence_field_absent: bool,
    fields_present: HashMap<String, bool>,
    body_links: Vec<String>,
}
```

`confidence_field_absent: bool` distinguishes "field not in schema" (→
`is_low_confidence = true`) from "field in schema, no value" (→
`is_low_confidence = false`). Without this distinction, pages in a wiki with no
`confidence` field in the schema would all be incorrectly flagged as
low-confidence.

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

`Manual` skips the file_watcher entirely. The reader must be explicitly
reloaded after every write by calling `reader.reload()`. In llm-wiki this
is done via `reload_reader()` called after each `writer.commit()`.

Note: `writer.commit()` only notifies readers opened on the **same** `Index`
instance. `rebuild()` uses a separate `Index::open_or_create()` instance, so
an explicit `reload_reader()` call is required regardless.

For llm-wiki, `Manual` is always correct:
- **CLI commands** are one-shot; they never need live reload.
- **`llm-wiki serve`** keeps a long-lived reader; `reload_reader()` after each
  write ensures all tools see the latest index without restarting.
- **`llm-wiki watch`** routes detected changes through the same write paths.

### Reader lifecycle

```
WikiEngine::build()
  └─ mount_space()
       ├─ index_manager.status()     ← Manual reader, temporary, dropped immediately
       ├─ index_manager.rebuild()    ← writer.commit() + reload_reader()
       └─ index_manager.open()       ← creates the long-lived Manual reader in inner.index_reader
              └─ held until engine is dropped; refreshed after every write via reload_reader()
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
