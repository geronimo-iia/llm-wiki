# `last_updated` promoted to keyword for FAST column access

## Decision

Add `"x-keyword": true` to `last_updated` in `schemas/base.json`, changing the
tantivy field type from `TEXT | STORED` (tokenized) to `STRING | STORED | FAST`
(keyword). This makes `last_updated` readable via `StrColumn` during segment
traversal, enabling a zero-doc-fetch `StalenessCollector` in `compute_staleness`.

## Context

`compute_staleness` in `src/ops/stats.rs` previously ran `AllQuery +
DocSetCollector` then called `searcher.doc(*addr)?` in a loop — one stored-field
fetch per page. On a 1,315-page wiki: 1,315 individual doc reads on every
`wiki_stats` call.

A custom `Collector` can read FAST fields directly during segment traversal via
`reader.fast_fields().str("field_name")`, returning a `StrColumn` that maps doc
IDs to term ordinals without touching the stored-doc store. This is the same
pattern used by `KeywordFacetCollector` (see `fast-field-facet-collector.md`).

`last_updated` was originally `type: string` with no `x-keyword` annotation,
so `classify_field` mapped it to `FieldClass::Text` → `add_text` →
`TEXT | STORED`. It had no columnar fast storage and could not be read during
collection.

## Alternatives considered

**Keep `last_updated` as TEXT, accept the N+1**: the 1,315 doc reads happen once
per `wiki_stats` call and are not on the hot path. Rejected for 1.0 because the
fix is low-risk (ISO 8601 date strings are not tokenized in practice — the
tokenizer produces a single token anyway) and the pattern is consistent with
other FAST field promotions in the codebase.

**Replace `AllQuery + DocSetCollector` with `TopDocs` and read stored fields in
one collector sweep**: still requires `searcher.doc()` per result. No reduction
in doc reads. Rejected.

**Add a separate `last_updated_kw` mirror field**: would avoid the schema
migration but doubles storage for date values. Rejected as unnecessary
complexity.

## Why keyword is appropriate for `last_updated`

ISO 8601 date strings (`2025-03-14`) are never full-text searched by field name.
They are stored for display and compared for staleness. Keyword storage is
semantically correct — the value is an atomic token, not a text to tokenize.
The field is already used as an exact-match value in `rule_stale`
(`NaiveDate::parse_from_str`) with no reliance on tokenization.

## Schema migration

Changing TEXT → keyword changes the tantivy field type. The `schema_hash`
changes, triggering `StalenessKind::FullRebuildNeeded` on first startup after
deploy (if `auto_rebuild = true`) or on the next `wiki_index_rebuild` call.
No data is lost; `last_updated` values are re-indexed as keywords.

## `rule_stale` in `src/ops/lint.rs`

`rule_stale` reads `last_updated` via `get_first(...).as_str()` — a stored
field read from the shared `DocRecord` pass, not a query. Changing TEXT →
`STRING|STORED|FAST` keeps the field stored; `get_first(...).as_str()` continues
to work identically. No change to `rule_stale` required.

## `StrColumn` API note

`reader.fast_fields().str("field_name")` returns `tantivy::Result<Option<StrColumn>>`.
`term_ords(RowId)` returns an iterator over term ordinals for a given doc.
`ord_to_str(u64, &mut String)` returns `io::Result<bool>` — the return value
must be explicitly discarded (`let _ = col.ord_to_str(...)`) to satisfy
`-D warnings`.

## Consequences

- `compute_staleness` replaces 1,315 `searcher.doc()` calls with zero — the
  date is read inline during the single `AllQuery` sweep via `StalenessCollector`
  and `StalenessSegmentCollector`.
- All wikis require a full index rebuild on first deploy of this change.
- Fields used with the FAST column pattern must be `STRING | STORED | FAST`;
  using TEXT fields with `fast_fields().str()` returns `None` silently.
- `title` was evaluated for the same promotion (to eliminate `backlinks_query`
  doc reads) but rejected: `title` is included in the `QueryParser` field list
  in `src/search.rs:186` and promoting it to keyword would break word-level
  full-text search on titles.
