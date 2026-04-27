---
title: "Roadmap"
summary: "Release history and version planning for llm-wiki."
status: ready
last_updated: "2026-04-27"
---

# Roadmap

## v0.1.1 — Released 2026-04-25

| Area        | What                                                          |
| ----------- | ------------------------------------------------------------- |
| Engine      | 19 MCP tools, ACP transport, tantivy 0.26                     |
| Type system | JSON Schema validation, type discovery, field aliasing        |
| Graph       | `x-graph-edges`, labeled directed edges, target type warnings |
| Search      | Facets (type/status/tag), BM25 ranking, cross-wiki            |
| Tools       | `wiki_stats`, `wiki_suggest`, `wiki_watch`, `wiki_history`    |
| Internals   | Native string sort, page body templates, 372 tests            |

## v0.2.0 — In progress

Improvements specified in [`docs/improvements/`](improvements/README.md),
ordered by priority:

| #   | Status | Improvement                                                                   | Engine | Skills |
| --- | :----: | ----------------------------------------------------------------------------- | :----: | :----: |
| 1   | ✅ | Confidence field (`confidence: 0.0–1.0` in base schema)                       |   ✦    |   —    |
| 1b  | ✅ | `claims[].confidence` aligned to float                                        |   ✦    |   ✦    |
| 2   | ✅ | Lifecycle-aware search ranking (`tweak_score`)                                |   ✦    |   —    |
| 2b  | ✅ | Flat `[search.status]` map for arbitrary status multipliers                   |   ✦    |   —    |
| 3   | ✅ | Backlinks (`backlinks:` param on `wiki_content_read`)                         |   ✦    |   ✦    |
| 4   | ✅ | Lint system (`wiki_lint` tool, 5 deterministic rules)                         |   ✦    |   ✦    |
| 5   | ✅ | Incremental validation (git-diff scoped)                                      |   ✦    |   —    |
| 6   | ✅ | Privacy redaction (`redact:` flag on `wiki_ingest`)                           |   ✦    |   —    |
| 7   | ✅ | Crystallize skill improvements (two-step extraction, confidence calibration)  |   —    |   ✦    |
| 8   | ✅ | Graph community detection (Louvain, `wiki_stats` + `wiki_suggest`)            |   ✦    |   ✦    |
| 9   | ✅ | `llms` format + `wiki_export` (file-writing, default `llms.txt` at wiki root) |   ✦    |   ✦    |
| 10  | ✅ | Cross-wiki links (`wiki://` URIs in graph, `wiki_graph(cross_wiki: true)`)    |   ✦    |   ✦    |
| 11  | ✅ | Ingest two-step: analysis pass before write (entities, contradictions, plan)  |   —    |   ✦    |
| 12  | ✅ | Review skill: prioritized queue from lint + draft/low-confidence pages        |   —    |   ✦    |
| —   | — | **Pre-release doc pass** — rustdocs, spec/guide audit, CHANGELOG date         |   ✦    |   ✦    |

Full specs, task lists, and dependency order: [`docs/improvements/README.md`](improvements/README.md).

## Related Projects

| Project                                                                | Roadmap                     |
| ---------------------------------------------------------------------- | --------------------------- |
| [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)     | `docs/roadmap.md`           |
| [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) | `docs/roadmap.md`           |
| [homebrew-tap](https://github.com/geronimo-iia/homebrew-tap)           | Formula updates per release |
| [asdf-llm-wiki](https://github.com/geronimo-iia/asdf-llm-wiki)         | Plugin updates per release  |
