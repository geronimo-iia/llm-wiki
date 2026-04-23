---
title: "Roadmap"
summary: "Development roadmap for llm-wiki."
status: ready
last_updated: "2025-07-21"
---

# Roadmap

## Completed

| Phase | What                                                          | Status |
| ----- | ------------------------------------------------------------- | ------ |
| 0     | Specification rationalization                                 | ✓      |
| 1     | Focused engine — 16 tools, MCP/ACP, tantivy 0.26              | ✓      |
| 2     | Type system — JSON Schema, discovery, aliasing, `wiki_schema` | ✓      |
| 3     | Typed graph — `x-graph-edges`, labeled edges, target warnings | ✓      |
| —     | Upgrade `agent-client-protocol` 0.10 → 0.11                   | ✓      |
| —     | Replace `_slug_ord` with native string sort                   | ✓      |
| —     | Page body templates — `schemas/<type>.md` naming convention     | ✓      |
| —     | `wiki_stats` — wiki health dashboard                           | ✓      |
| —     | `wiki_suggest` — suggest related pages to link                 | ✓      |
| —     | `wiki_watch` — filesystem watcher, auto-ingest on save          | ✓      |
| —     | `wiki_history` — git commit history for a page                | ✓      |
| —     | Search facets — type/status/tag distributions                  | ✓      |

372 tests. Single Rust binary. No runtime dependencies.

## Active

| Task                             | Prompt                                                   | Notes                                           |
| -------------------------------- | -------------------------------------------------------- | ----------------------------------------------- |
| Semantic search                  | `docs/prompts/study-semantic-search.md`                  | BM25 + vector embeddings                        |
| Cross-wiki links                 | `docs/prompts/study-cross-wiki-links.md`                 | `wiki://` URIs resolved in graph                |

## Next: Phase 4 — Skill Registry

The wiki becomes a full skill registry. Pages with `type: skill` are
searchable, listable, and readable like any other page.

- [ ] Verify `wiki_search --type skill` works end-to-end with
  `x-index-aliases`
- [ ] Verify `wiki_list --type skill` returns skill-specific metadata
- [ ] Verify `wiki_graph` renders skill edges correctly
- [ ] Cross-wiki skill discovery: `wiki_search --type skill --cross-wiki`

### Milestone

Agents discover skills via search, read them via `wiki_content_read`,
activate them by injecting the body into context.

## Future

Engine improvements not tied to a phase:

### High value

- `wiki_export` — static site, PDF, or EPUB

### Medium value

- Persistent graph index — maintain petgraph across ingests, avoid rebuilding on every call
- Incremental graph — update petgraph on ingest instead of full rebuild
- `wiki_export` — static site, PDF, or EPUB

### Lower priority

- Webhook on ingest — notify external systems
- ACP workflows beyond `research` (ingest, explore, summarize)

## Related Projects

Each project has its own roadmap:

| Project                                                                | Roadmap                                     |
| ---------------------------------------------------------------------- | ------------------------------------------- |
| [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)     | `docs/roadmap.md` — skill sync + new skills |
| [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) | `docs/roadmap.md` — Hugo site scaffold      |
| [homebrew-tap](https://github.com/geronimo-iia/homebrew-tap)           | Formula updates per release                 |
| [asdf-llm-wiki](https://github.com/geronimo-iia/asdf-llm-wiki)         | Plugin updates per release                  |
