# Decision: Sorted List Pagination

## Problem

`list()` fetched all documents into memory, sorted by slug in Rust,
then sliced for the requested page. O(n) per request regardless of
page size.

## Decision (v1 — superseded)

Added a `_slug_ord` u64 FAST field. Encoded first 8 bytes of slug as
big-endian u64. Used `order_by_fast_field::<u64>("_slug_ord", Asc)`.
Ties broken by in-window slug sort.

## Decision (v2 — current)

tantivy 0.26 added `order_by_string_fast_field`. The `slug` field
now has `STRING | STORED | FAST`. List uses
`order_by_string_fast_field("slug", Order::Asc)` — native
lexicographic sort, no encoding, no tie-breaking.

Removed: `_slug_ord` field, `slug_ordinal()` function, in-window sort.

## Schema change

- `slug`: STRING | STORED | FAST (was STRING | STORED)
- Requires index rebuild (no production wikis exist yet)
