---
title: "Lifecycle-Aware Search Ranking"
summary: "Apply status and confidence multipliers inside the tantivy collector via tweak_score, not as a post-retrieval sort."
status: implemented
last_updated: "2026-04-27"
depends_on: confidence
---

# Lifecycle-Aware Search Ranking

## Problem

`wiki_search` ranks results purely by BM25 score. A `status: archived` or
`status: draft` page can rank above a `status: active` page on the same topic if it
has higher term density. An LLM trusts the ranking order; receiving a retired or
incomplete page at the top is misleading.

The current design (post-retrieval re-sort) compounds this: tantivy collects the
top-k by raw BM25, *then* multipliers are applied. A high-quality active page ranked
k+1 by BM25 is never seen regardless of its status or confidence.

## Goal

Apply quality multipliers **during collection** so the collector's priority queue
already reflects the final ranking. The top-k results are the true top-k, not just
the top-k by raw BM25.

## Solution

Use tantivy 0.26's `TopDocs::tweak_score` API. The closure receives a
`SegmentReader` once per segment, reads fast fields per `DocId`, and returns the
adjusted score. This runs inside the tantivy collector with no extra allocation.

```rust
// src/search.rs (sketch)
let status_field  = is.field("status");
let conf_field    = is.field("confidence");

let collector = TopDocs::with_limit(options.top_k)
    .tweak_score(move |segment_reader: &SegmentReader| {
        let status_col = segment_reader
            .fast_fields()
            .str(&status_field)          // STRING | STORED | FAST — already the case
            .unwrap();
        let conf_col = segment_reader
            .fast_fields()
            .f64(&conf_field)            // requires confidence improvement (#1)
            .unwrap();

        move |doc: DocId, score: Score| {
            let status_mult = match status_col.term_ords(doc).next() {
                Some(ord) => match status_col.ord_to_str(ord) {
                    "active"   => 1.0,
                    "draft"    => 0.8,
                    "archived" => 0.3,
                    _          => 0.9,
                },
                None => 0.9,
            };
            let confidence = conf_col.first(doc).unwrap_or(0.5) as f32;
            score * status_mult * confidence
        }
    });

let top_docs = searcher.search(&final_query, &collector)?;
```

**Why `tweak_score` over post-retrieval re-sort:**
- Correctness: a page ranked k+1 by raw BM25 but high quality (active + confidence
  0.9) is included in results; with post-sort it would never appear.
- Performance: no second sort pass; multiplier is applied once per doc during
  collection at negligible cost (two fast field reads per doc).

**Fast field availability:**
- `status` — already `STRING | STORED | FAST` via `add_keyword()` in `SchemaBuilder`.
  No schema change needed.
- `confidence` — requires improvement #1 (confidence field) which adds it as a
  dedicated `f64` FAST numeric field. The `tweak_score` approach is the reason
  `FAST` is required there, not just `STORED`.

**Multiplier table:**

| `status` | Multiplier |
|---|---|
| `active` | ×1.0 |
| `draft` | ×0.8 |
| `archived` | ×0.3 |
| absent / unknown | ×0.9 |

Combined formula: `final_score = bm25 × status_multiplier × confidence`

**Configuration:** multipliers follow the standard two-level resolution — global
default in `config.toml`, per-wiki override in `wiki.toml`. Per-wiki wins, same
pattern as `[ingest]`, `[graph]`, `[suggest]`.

```toml
# config.toml (global default) or wiki.toml (per-wiki override)
[search]
status_active   = 1.0
status_draft    = 0.8
status_archived = 0.3
status_unknown  = 0.9
```

Any key omitted falls back up the resolution chain:
```
CLI flag → wiki.toml [search] → config.toml [search] → built-in default
```

The multipliers are resolved once per `wiki_search` call from `ResolvedConfig`
and passed into the `tweak_score` closure by copy — no change to the closure shape.

## Tasks

### Config
- [x] Add `SearchConfig` struct to `src/config.rs` with four `f32` multiplier fields and `Default` impl matching the table above.
- [x] Wire `SearchConfig` into `WikiConfig` under `[search]`; expose via `ResolvedConfig`.
- [x] Update `wiki.toml` spec (`docs/specifications/model/wiki-toml.md`): add `[search]` section with multiplier fields.

### Source code
- [x] Add `add_numeric` method to `SchemaBuilder` in `src/index_schema.rs`: `f64 | FAST | STORED`; use it to register the `confidence` field (coordinate with improvement #1).
- [x] Replace `TopDocs::with_limit(...).order_by_score()` in `src/search.rs::search()` with `TopDocs::with_limit(...).tweak_score(...)`; pass resolved `SearchConfig` multipliers into the closure by copy.
- [x] In the `tweak_score` closure: read `status` via `fast_fields().str()` and `confidence` via `fast_fields().f64()`; apply multipliers from config.
- [x] Populate `confidence` on `PageRef` from the stored field (already a task in improvement #1).

### Tests
- [x] Index three pages with identical body text, `status: active/draft/archived`; assert active ranks first, archived last (default config).
- [x] Index two pages with identical body and status; `confidence: 0.9` vs `confidence: 0.2`; assert high-confidence ranks first.
- [x] Combined: `archived + confidence 1.0` ranks below `active + confidence 0.5` (0.3 × 1.0 = 0.3 < 1.0 × 0.5 = 0.5).
- [x] Custom config: set `status_archived = 0.0`; assert archived pages never appear in results.

### Spec docs
- [x] Update `docs/specifications/tools/search.md`: document ranking formula, multiplier table, and `[search]` config keys.
- [x] Update `docs/specifications/model/global-config.md`: add `[search]` to the overridable defaults table with all four keys and their defaults.
- [x] Update `docs/specifications/model/wiki-toml.md`: add `[search]` to the per-wiki overridable settings section.
- [x] Update `docs/specifications/model/global-config.md` example block: add `[search]` section.
