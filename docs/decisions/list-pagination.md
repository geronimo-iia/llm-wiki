# Decision: Sorted List Pagination via _slug_ord

## Problem

`list()` fetched all documents into memory, sorted by slug in Rust,
then sliced for the requested page. O(n) per request regardless of
page size.

## Decision

Add a `_slug_ord` u64 FAST field to the tantivy index. At index time,
encode the first 8 bytes of the slug as a big-endian u64. At query
time, use `TopDocs::order_by_fast_field("_slug_ord", Asc)` with
offset + page_size limit. Extract full fields only for the page window.

Ties (slugs sharing the same 8-byte prefix) are broken by an
in-window slug sort.

## Why not text FAST field sorting?

Tantivy's `order_by_fast_field` requires `FastValue`, which is
numeric-only (u64, i64, f64, bool, DateTime). No lexicographic
text sorting is available.

## Schema change

- `_slug_ord`: u64, FAST | STORED
- Requires index rebuild (no production wikis exist yet)
- tantivy bumped from 0.22 to 0.25 in the same change
