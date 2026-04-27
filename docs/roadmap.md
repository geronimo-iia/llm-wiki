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

| Area        | What                                                                          |
| ----------- | ----------------------------------------------------------------------------- |
| Type system | `confidence: 0.0–1.0` field; `claims[].confidence` as float                  |
| Search      | Lifecycle-aware ranking; flat `[search.status]` multiplier map                |
| Content     | Backlinks on `wiki_content_read`; incremental validation (git-diff scoped)    |
| Lint        | `wiki_lint` tool with 5 rules; `broken-cross-wiki-link` rule                  |
| Redaction   | `redact:` flag on `wiki_ingest`; built-in and custom patterns                 |
| Graph       | Louvain community detection; `wiki://` cross-wiki edges; `--cross-wiki` flag  |
| Export      | `wiki_export` + `llms` format on list, search, and graph                      |
| Skills      | Crystallize two-step; ingest analysis pass; review skill                      |

## Related Projects

| Project                                                                | Roadmap                     |
| ---------------------------------------------------------------------- | --------------------------- |
| [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)     | `docs/roadmap.md`           |
| [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) | `docs/roadmap.md`           |
| [homebrew-tap](https://github.com/geronimo-iia/homebrew-tap)           | Formula updates per release |
| [asdf-llm-wiki](https://github.com/geronimo-iia/asdf-llm-wiki)         | Plugin updates per release  |
