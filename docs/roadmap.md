---
title: "Roadmap"
summary: "Roadmap planning for llm-wiki."
status: ready
last_updated: "2026-05-03"
---

<<<<<<< Updated upstream
# Roadmap

## v0.4.0 (in progress)

| Area  | What |
|-------|------|
| Graph | petgraph-live Phase 1: replace bespoke `CachedGraph` with `GenerationCache` ✓ implemented |
| Graph | petgraph-live Phase 2: snapshot warm-start via `GraphState` ✓ implemented |
| Graph | petgraph-live Phase 3: `wiki_health` MCP tool + structural algorithms — see [docs/improvements/2026-05-03-petgraph-live-algorithms.md](improvements/2026-05-03-petgraph-live-algorithms.md) |
||||||| Stash base
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

## v0.2.0 — Released 2026-04-28

| Area        | What                                                                                                                                             |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Type system | `confidence: 0.0–1.0` field; `claims[].confidence` as float                                                                                      |
| Search      | Lifecycle-aware ranking; flat `[search.status]` multiplier map                                                                                   |
| Content     | Backlinks on `wiki_content_read`; incremental validation (git-diff scoped); `wiki_resolve` tool; `wiki_content_new` returns `path` + `wiki_root` |
| Lint        | `wiki_lint` tool with 5 rules; `broken-cross-wiki-link` rule; `path` field on every finding                                                      |
| Redaction   | `redact:` flag on `wiki_ingest`; built-in and custom patterns                                                                                    |
| Graph       | Louvain community detection; `wiki://` cross-wiki edges; `--cross-wiki` flag                                                                     |
| Export      | `wiki_export` + `llms` format on list, search, and graph                                                                                         |
| Links       | CommonMark `[text](slug)` body links indexed alongside `[[wikilinks]]`                                                                           |
| Skills      | Crystallize two-step; ingest analysis pass; review skill; `v0.4.0`                                                                               |

## v0.3.0 — Current

| Area    | What                                                                                                                             |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- |
| ACP     | Six workflows (`research`, `lint`, `graph`, `ingest`, `use`, `help`); `step_read` streams page body; `llm-wiki:` prefix dispatch |
| ACP     | Cooperative cancellation via `Arc<AtomicBool>`; session cap via `serve.acp_max_sessions`; watcher push via mpsc channel          |
| ACP     | `--http` flag required alongside `--acp` to give ACP exclusive stdio (MCP displaces to HTTP)                                     |
| Testing | `validate-acp.sh` + `docs/testing/scripts/acp/` section scripts; `setup-test-env.sh` configures ACP test settings                |
| Graph   | In-memory `WikiGraph` cache keyed on index generation; shared community map; automatic invalidation on ingest                    |
| Configurable Wiki Root                          | see docs/improvements/design-configurable-wiki-root.md   |
=======
# Roadmap                                                                    |

## v0.3.0 — Current

| Area    | What                                                                                                                             |
| ------- | -------------------------------------------------------------------------------------------------------------------------------- |
| ACP     | Six workflows (`research`, `lint`, `graph`, `ingest`, `use`, `help`); `step_read` streams page body; `llm-wiki:` prefix dispatch |
| ACP     | Cooperative cancellation via `Arc<AtomicBool>`; session cap via `serve.acp_max_sessions`; watcher push via mpsc channel          |
| ACP     | `--http` flag required alongside `--acp` to give ACP exclusive stdio (MCP displaces to HTTP)                                     |
| Testing | `validate-acp.sh` + `docs/testing/scripts/acp/` section scripts; `setup-test-env.sh` configures ACP test settings                |
| Graph   | In-memory `WikiGraph` cache keyed on index generation; shared community map; automatic invalidation on ingest                    |
| Configurable Wiki Root                          | see docs/improvements/design-configurable-wiki-root.md   |
>>>>>>> Stashed changes

## Future

| Area                                            | What                                                     |
| ----------------------------------------------- | -------------------------------------------------------- |
| IDE                                             | Zed agent panel validation; Cursor MCP config validation |
| Remote Wiki Registration and Version Management | see docs/improvements/design-spaces-register-remote.md   |
| REST / OpenAPI API                              | see docs/improvements/2026-05-03-rest-api-design.md      |

