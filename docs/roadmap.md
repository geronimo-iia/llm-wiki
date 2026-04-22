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
| —     | `wiki_history` — git commit history for a page                | ✓      |
| —     | Search facets — type/status/tag distributions                  | ✓      |

372 tests. Single Rust binary. No runtime dependencies.

## Active

| Task                             | Prompt                                                   | Notes                                           |
| -------------------------------- | -------------------------------------------------------- | ----------------------------------------------- |
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

- `wiki_watch` — filesystem watcher that auto-ingests on save
- `wiki_search` hybrid/semantic search — BM25 + vector embeddings for terminology-independent retrieval
- `wiki_suggest` — given a page, suggest related pages to link (graph + search candidates)
- `wiki_stats` — wiki health dashboard (page count, orphans, avg connections, staleness)

### Medium value

- Page body templates via `wiki_content_new --type` — scaffold full page body, not just frontmatter
- Persistent graph index — maintain petgraph across ingests, avoid rebuilding on every call
- Incremental graph — update petgraph on ingest instead of full rebuild
- `wiki_diff` — changes between two commits for a page
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
