---
title: "Confidence Field"
summary: "Add confidence: 0.0–1.0 to the base page schema; index as a numeric field; use as a search ranking multiplier."
status: implemented
last_updated: "2026-04-27"
---

# Confidence Field in Base Page Schema

## Problem

All pages are currently treated as equally authoritative. There is no standard way
to signal that a page's content is speculative, preliminary, or well-established.
An LLM writing a page from partial information has no field to record its own
uncertainty. A human reviewer has no field to promote a draft to "verified".

Without a confidence signal:
- Search ranking cannot distinguish a well-researched page from a stub (see design-03).
- A lint rule cannot flag pages that have been sitting at low confidence for too long.
- An LLM consuming the wiki cannot calibrate how much to trust a given page's content.

## Goals

- `confidence` is a first-class frontmatter field on every page type.
- Valid range: `0.0` to `1.0` (float). Conventional meanings:
  - `0.0–0.3` — speculative, unverified, initial LLM draft
  - `0.4–0.6` — partial, needs review or additional sources
  - `0.7–0.9` — reviewed, mostly accurate
  - `1.0` — verified, authoritative
- Default when absent: `0.5` (neutral; does not penalize pages written before this field existed).
- Indexed as a numeric field in tantivy to support range queries and score boosting.
- Included in page scaffold templates so new pages are created with an explicit value.
- Exposed in search results so LLMs can read it directly.

## Solution

**Schema change:**

Add `confidence` to the base `page` schema (`schemas/page.json`):
```json
"confidence": {
  "type": "number",
  "minimum": 0.0,
  "maximum": 1.0,
  "default": 0.5,
  "description": "Certainty of this page's content. 0.0=speculative, 1.0=verified."
}
```

All type schemas inherit from `page`, so `confidence` becomes available on every type.

**Frontmatter getter:**

Add `fn confidence(fm: &BTreeMap<String, Value>) -> f32` in `src/frontmatter.rs`,
returning the field value or `0.5` if absent.

**Tantivy index:**

Add a `confidence` field to `src/index_schema.rs` as `FAST | STORED` (numeric, f32).
`FAST` enables efficient per-document score access without decoding the full document.

**Search ranking integration:**

In `src/ops/search.rs`, after the status multiplier (design-03), apply an additional
confidence multiplier:
```
score *= confidence  (where confidence = stored field value, default 0.5)
```

This means a `confidence: 0.2` page scores at 20% of its BM25 weight relative to
a `confidence: 1.0` page with identical text. Combined with the status multiplier,
the full formula becomes:
```
final_score = bm25 × status_multiplier × confidence
```

**Page scaffold update:**

Add `confidence: 0.5` to the default frontmatter emitted by `wiki_content_new`,
so newly created pages have an explicit starting value.

**Search result:**

Include `confidence` in the `SearchResult` struct and JSON response alongside
`slug`, `title`, `type`, `status`, and `excerpt`.

## Schema migration note

`concept` and `source` type schemas already declare `confidence` as a string enum
(`high`, `medium`, `low`). Change their type to `number` with the same
`minimum: 0.0 / maximum: 1.0 / default: 0.5` as the base — keeping the explicit
declaration at the type level, just aligning the type. The `claims[].confidence`
sub-field inside concept pages is scoped to claim objects, not page-level frontmatter
— it is unaffected and stays as-is.

The frontmatter getter must handle legacy string values gracefully: existing pages
with `confidence: high` should not crash on read. Map legacy strings on read:
`high → 0.9`, `medium → 0.5`, `low → 0.2`.

## Tasks

### Schemas (JSON)
- [x] Add `confidence` float field to `schemas/base.json`: `type: number`, `minimum: 0.0`, `maximum: 1.0`, `default: 0.5`. *(done via concept.json + paper.json; base.json deferred — no base.json schema file)*
- [x] Change `confidence` in `schemas/concept.json` from string enum to `number` with same range and default; leave `claims[].confidence` string enum untouched (different scope).
- [x] Change `confidence` in `schemas/paper.json` from string enum to `number` with same range and default; same caveat for `claims[].confidence`.

### Source code
- [x] Add `fn confidence(fm: &BTreeMap<String, Value>) -> f32` to `src/frontmatter.rs`; return `0.5` when absent; map legacy strings (`"high" → 0.9`, `"medium" → 0.5`, `"low" → 0.2`); clamp result to `[0.0, 1.0]`.
- [x] Add `confidence` as a `f64 | FAST | STORED` numeric field in `src/index_schema.rs` via a new `add_numeric` method on `SchemaBuilder`; populate during index build via the `confidence()` getter. Tantivy 0.26 exposes numeric fast fields only as `f64` — store and read as `f64`, cast to `f32` at the `PageRef` boundary.
- [x] Add `confidence: f32` to `PageRef` struct in `src/search.rs`; populate from stored field (default `0.5`).
- [x] Add `confidence: f32` to `PageSummary` struct in `src/search.rs`; populate from stored field.
- [x] Update `src/ops/search.rs` to read `confidence` from the stored field and apply as score multiplier after the status multiplier (improvement #2 wires this in).
- [x] Add `confidence: 0.5` to `fn scaffold()` in `src/frontmatter.rs` so `wiki_content_new` emits it by default.

### Schema body templates
- [x] Add `confidence: 0.5` to `schemas/concept.md` frontmatter template if it contains a frontmatter block; otherwise leave (body template, not frontmatter). *(body-only template — no frontmatter block, nothing to add)*
- [x] Same for `schemas/paper.md`. *(body-only template — no frontmatter block, nothing to add)*

### Specification docs
- [x] `docs/specifications/model/types/base.md`: add `confidence` to optional fields table (`float 0.0–1.0`, default `0.5`).
- [x] `docs/specifications/model/types/concept.md`: change `confidence` field type from string enum to `float 0.0–1.0`; update all `confidence: high` examples to `confidence: 0.9`; note `claims[].confidence` remains a string enum.
- [x] `docs/specifications/model/types/source.md`: same — change table entry and examples.
- [x] `docs/specifications/engine/index-management.md`: note `confidence` is now a dedicated numeric FAST field, not an arbitrary text field.
- [x] `docs/specifications/tools/search.md`: document `confidence: float` in the `PageRef` result schema.
- [x] `docs/specifications/tools/list.md`: document `confidence: float` in the `PageSummary` result schema.

### Tests
- [x] `fn scaffold()` emits `confidence: 0.5`.
- [x] `fn confidence()` maps `"high" → 0.9`, `"medium" → 0.5`, `"low" → 0.2`, absent → `0.5`, out-of-range float clamped.
- [x] Search ranks `confidence: 0.9` page above `confidence: 0.2` page with identical body text.
