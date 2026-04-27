# Improvements

Ordered by implementation priority — each item either stands alone or unblocks the
items that follow it.

| #   | Status | File                                                   | Summary                                                                                              | Depends on |
| --- | :----: | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ---------- |
| 1   | ✅ | [confidence.md](confidence.md)                         | Add `confidence: 0.0–1.0` to base schema; numeric tantivy field; search ranking multiplier           | —          |
| 1b  | ✅ | [claims-confidence-float.md](claims-confidence-float.md) | Align `claims[].confidence` with page-level — change from string enum to float `0.0–1.0`           | #1         |
| 2   | ✅ | [search-ranking.md](search-ranking.md)                 | `tweak_score` inside collector: status × confidence multipliers, true top-k ranking                  | #1         |
| 2b  | ✅ | [search-ranking-custom-status.md](search-ranking-custom-status.md) | Flat `[search.status]` map in `SearchConfig` for arbitrary status multipliers | #2         |
| 3   | ✅ | [backlinks.md](backlinks.md)                           | `backlinks: bool` on `wiki_content_read`; tantivy term query on `body_links`                         | —          |
| 4   | — | [lint.md](lint.md)                                     | `wiki_lint` engine tool (5 deterministic rules) + skill update to call it                            | #1         |
| 5   | — | [incremental-validation.md](incremental-validation.md) | Restrict `wiki_ingest` validation to git-changed files via `collect_changed_files`                   | —          |
| 6   | — | [redaction.md](redaction.md)                           | Opt-in `redact: true` on `wiki_ingest`; built-in patterns + per-wiki `wiki.toml` config              | —          |
| 7   | — | [crystallize.md](crystallize.md)                       | Two-step extraction pass, confidence calibration table, post-ingest lint step in `crystallize` skill | #1, #4     |
| 8   | — | [community-detection.md](community-detection.md)       | Louvain clustering on `petgraph::DiGraph`; `communities` in `wiki_stats`; strategy 4 in `wiki_suggest` | —        |
| 9   | — | [export.md](export.md)                                 | `format: "llms"` on `wiki_list`/`wiki_search`/`wiki_graph`; `wiki_export(path:)` writes full wiki to file | #1    |
| 10  | — | [cross-wiki-links.md](cross-wiki-links.md)             | `wiki://` URIs as link targets; cross-wiki edges resolved at graph build time; `wiki_graph(cross_wiki: true)` | — |
| 11  | — | [ingest-two-step.md](ingest-two-step.md)               | Explicit analysis pass in `ingest` skill before writes: enumerate entities, detect contradictions, produce ingest plan | — |
| 12  | — | [review-skill.md](review-skill.md)                     | New `review` skill: prioritized queue from `wiki_lint` + draft/low-confidence pages; guided review loop per page | #1, #4 |

Items 1–2 are coupled: implement confidence first, then wire it into search ranking
in the same pass. Items 3, 5, 6 are fully independent. Item 4 (lint) can start
without confidence but the `stale` rule becomes richer once #1 is in place.
Item 7 is skill-only: no engine work required, but benefits from confidence (#1)
being in the index and lint (#4) being available as an engine tool.
