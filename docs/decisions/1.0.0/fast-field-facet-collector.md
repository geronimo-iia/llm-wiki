# Fast-field facet collector: `KeywordFacetCollector` with `StrColumn`

## Decision

Replace the `collect_facet` doc-fetch loop with a custom `KeywordFacetCollector`
that reads tantivy columnar fast fields (`StrColumn`) directly. Bundle multiple
field collectors into a single `searcher.search()` pass via `MultiCollector`.

## Context

`wiki_search` and `wiki_list` return facet distributions over `type`, `status`,
and `tags`. The original `collect_facet` ran a `DocSetCollector` query pass and
then fetched every matching stored document to read field values — O(n) document
deserializations per facet field, 3 passes per `search()` call and 3 per
`list()` call (6 extra passes total).

Tantivy's columnar fast fields allow reading keyword values directly from
compressed columnar storage without touching the stored-doc store. Fields indexed
as `STRING | STORED | FAST` expose a `StrColumn` that maps doc IDs to term
ordinals in O(1) per doc.

## Alternatives considered

**Tantivy's built-in `FacetCollector`** — designed for tantivy's hierarchical
`Facet` type (e.g. `/category/subcategory`). Our fields (`type`, `status`,
`tags`) are `STRING` fields, not `FACET`. `FacetCollector` requires documents
to be indexed with `add_facet()`; it cannot read `STRING` fast fields. Rejected:
wrong tantivy index type.

**Accumulate facets during the main `TopDocs` pass** — extend `tweak_score`
(which already iterates docs with fast-field access) to accumulate counts.
Rejected: `tweak_score` only sees the top-K documents, not the full result set.
Facet counts must cover all matching documents, not just the page being returned.

**Post-process stored docs from `TopDocs` results** — read facet values from
the already-fetched result documents. Rejected: `TopDocs` is bounded by
`top_k`; facet distributions across hundreds of results require all matching
docs, not just the top page.

**Per-field separate search calls** — keep separate calls but switch each from
`DocSetCollector` + stored fetch to a lightweight counting collector. Rejected:
`MultiCollector` allows bundling with zero extra segment traversals; there is no
reason to keep the calls separate.

## Why `KeywordFacetCollector` + `MultiCollector`

- `StrColumn` reads from columnar fast storage — no document deserialization.
- `MultiCollector` fuses multiple collectors into one segment traversal.
  `TopDocs` + `status` + `tags` collectors run in one `searcher.search()` call.
- The implementation mirrors the existing `tweak_score` fast-field pattern
  already in `src/search.rs` (`fast_fields().str(name)` + `term_ords` +
  `ord_to_str`) — no new idioms introduced.
- `tantivy::columnar` is re-exported by tantivy 0.26 — no new dependency.

## Why `type` facet remains a separate pass

`type` facets are counted over all documents in the space (unfiltered query),
while `status` and `tags` facets are counted over the filtered result set. These
are different queries and cannot be folded into the same `MultiCollector` pass.
`type` uses a second `collect_facets` call on the unfiltered query.

## Schema prerequisite: `type` field must be FAST

`fast_fields().str("type")` returns `None` for TEXT fields. The `type` field was
originally classified as `TEXT | STORED` because `classify_field`'s string arm
did not respect `"x-keyword": true` (only the array arm did). Fixed in
`src/index_schema.rs` and all built-in schemas (`base.json`, `concept.json`,
`doc.json`, `paper.json`, `section.json`). Schema hash mismatch triggers an
automatic index rebuild on first deploy.

## Consequences

- `collect_facet` deleted; replaced by `KeywordFacetCollector`,
  `KeywordFacetSegmentCollector`, and `collect_facets` in `src/search.rs`.
- `search()` and `list()` each drop from 4 to 2 tantivy segment passes.
- Fields used as facets must be `STRING | STORED | FAST`; using TEXT fields
  silently produces empty facet counts (the column returns `None`).
- `search_all()` is unaffected — it calls `search()` and merges the returned
  `SearchResult.facets`; it does not call `collect_facets` directly.
