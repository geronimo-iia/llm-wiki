# Study: wiki_search facets

Explore adding faceted search to `wiki_search` — return type, status,
and tag distributions alongside search results.

## Problem

Today, `wiki_search` returns a flat ranked list. To understand the
shape of results — how many concepts vs papers, which tags dominate,
how many drafts — you need to paginate through everything or run
multiple filtered queries.

Facets solve this by returning distribution counts in a single query.

## Decisions

- **Always returned** — facets are included in every search and list
  response. No `--facets` flag needed.
- **Fixed fields** — always facet on `type`, `status`, `tags`. No
  `defaults.facets_fields` config (fields are fixed in base schema).
- **Top-N tags** — `defaults.facets_top_tags = 10` in config. `type`
  and `status` return all values (low cardinality).
- **Hybrid filtering** — `type` facet is always unfiltered (shows
  full distribution even when `--type` filter is active, so the user
  can see what else is available). `status` and `tags` facets are
  filtered (describe the current result set).
- **`wiki_list`** — always includes facets, no flag.

## Open questions

- Tantivy implementation: `FacetCollector` vs `column_values()` on
  FAST fields — needs prototyping.

## What facets would look like

### CLI

```
llm-wiki search "mixture of experts"
```

Facets are always included in the response. No flag needed.

### MCP

```json
{
  "query": "mixture of experts"
}
```

Facets are always included in the response. No parameter needed.

### Response (JSON)

```json
{
  "results": [ ... ],
  "facets": {
    "type": {
      "concept": 12,
      "paper": 8,
      "article": 3
    },
    "status": {
      "active": 20,
      "draft": 3
    },
    "tags": {
      "mixture-of-experts": 15,
      "scaling": 9,
      "transformers": 7,
      "routing": 5
    }
  }
}
```

### Response (text)

Appended after results:

```
--- facets ---
type:    concept(12)  paper(8)  article(3)
status:  active(20)  draft(3)
tags:    mixture-of-experts(15)  scaling(9)  transformers(7)  routing(5)
```

## Tantivy support

Tantivy supports faceted search natively via `FAST` fields. The
current index schema already has `type` and `status` as keyword
fields. Questions to investigate:

- Are `type` and `status` already `FAST` fields? If not, they need
  to be — facet counting requires `FAST` on keyword fields.
- `tags` is a multi-valued keyword field — does tantivy support
  facet counting on multi-valued `FAST` fields?
- `slug` is already `FAST` (used for sorted pagination). Same
  pattern applies.
- What is the performance cost of facet collection on top of BM25
  ranking? Is it negligible for typical wiki sizes (<10K pages)?

## Tantivy implementation sketch

```rust
// At index build time: ensure type, status, tags are FAST keyword fields
// (may already be the case)

// At query time:
let facet_collector = FacetCollector::for_field("type");
let (top_docs, facet_counts) = searcher.search(
    &query,
    &(TopDocs::with_limit(top_k), facet_collector)
)?;
```

Tantivy's `FacetCollector` may not work directly on keyword fields —
it's designed for hierarchical facet fields. Alternative: use
`TermAggregation` or manually collect term frequencies from `FAST`
fields via `column_values()`.

Investigate which approach is simpler and more performant.

## Interaction with existing filters

Hybrid approach:

- **`type` facet is always unfiltered** — shows the full distribution
  even when `--type concept` is active. This lets the user see
  "there are also 8 papers" and switch filters.
- **`status` and `tags` facets are filtered** — they describe the
  current result set after type filtering.

Example: `wiki_search --type concept "moe"` returns:

```json
{
  "facets": {
    "type": { "concept": 12, "paper": 8, "article": 3 },
    "status": { "active": 11, "draft": 1 },
    "tags": { "mixture-of-experts": 10, "scaling": 5 }
  }
}
```

`type` shows all types (unfiltered). `status` and `tags` show only
the concept results (filtered).

## Interaction with wiki_list

`wiki_list` always includes facets — same implementation, match-all
query instead of BM25. Gives wiki composition for free (useful for
bootstrap).

## Impact on skills

- **bootstrap** — use facets to report wiki composition (page count
  per type) without paginating
- **research** — show result distribution to help narrow queries
- **lint** — use facets to find status distribution (how many drafts?)

## Tasks

### 1. Update specifications

- [ ] `docs/specifications/tools/search.md` — facets always in response, hybrid filtering
- [ ] `docs/specifications/tools/list.md` — facets always in response
- [ ] `docs/specifications/engine/index-management.md` — document FAST requirement on `type`, `status`, `tags`
- [ ] `docs/specifications/model/global-config.md` — add `defaults.facets_top_tags`

### 2. Index schema

- [ ] Ensure `type`, `status`, `tags` are FAST keyword fields
- [ ] Verify index rebuild picks up the FAST change

### 3. Core search

- [ ] Add `FacetCounts` struct (`HashMap<String, HashMap<String, u64>>`)
- [ ] Implement facet collection via `column_values()` on FAST fields
- [ ] `type` facet: always unfiltered (run against full query without type filter)
- [ ] `status` and `tags` facets: filtered (run against current result set)
- [ ] Cap tag facets to top N (from `defaults.facets_top_tags`)
- [ ] Return facets in `SearchResult` and `PageList`

### 4. Config

- [ ] Add `defaults.facets_top_tags` (default: 10) to global config
- [ ] Wire through to search/list ops

### 5. Ops layer

- [ ] Thread `facets_top_tags` config value to search and list

### 6. MCP

- [ ] Update `wiki_search` and `wiki_list` response schemas to include facets

### 7. CLI

- [ ] Render facet block in text and JSON output for search and list

### 8. Tests

- [ ] Facet counts match expected distribution
- [ ] Hybrid filtering: `type` unfiltered, `status`/`tags` filtered
- [ ] Empty facets when no results
- [ ] Top-N tag capping

### 9. Decision record

- [ ] `docs/decisions/search-facets.md` — always-on facets, hybrid
  filtering rationale, top-N tags, no `--facets` flag

### 10. Update skills

- [ ] `llm-wiki-skills/skills/research/SKILL.md` — mention facets
  in search results, use type facet to suggest narrowing
- [ ] `llm-wiki-skills/skills/bootstrap/SKILL.md` — use list facets
  to report wiki composition (page count per type) instead of
  paginating
- [ ] `llm-wiki-skills/skills/lint/SKILL.md` — use status facet to
  find draft count
- [ ] `llm-wiki-skills/skills/content/SKILL.md` — mention facets in
  list output

### 11. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md` — move facets from Active to Completed
- [ ] Remove `docs/prompts/study-search-facets.md`

## Success criteria

- `wiki_search("moe", facets: ["type"])` returns type distribution
  alongside results
- Facets work with existing filters (`--type`, `--status`)
- No measurable performance regression for non-faceted queries
- At least `type`, `status`, and `tags` supported as facet fields
