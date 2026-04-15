# Architecture

## Design principles

1. **No LLM calls.** The `wiki` binary has zero LLM dependency. It manages Markdown
   files, git history, and tantivy search indexes. All intelligence is external.
2. **analysis.json is the boundary.** External LLMs produce `analysis.json`;
   the wiki engine consumes it. The contract is documented in
   [`docs/design/design.md`](../design/design.md).
3. **Git is the backend.** Every ingest session is a commit. The full history of
   how the knowledge base evolved is in `git log`.
4. **Contradictions are knowledge.** Contradiction pages are first-class nodes,
   never deleted, only enriched.

## Module map

Modules marked ✅ are fully implemented. Others are stubs for future phases.

```
src/
├── main.rs          ✅ Entry point — parse CLI, dispatch to modules
├── cli.rs           ✅ clap Command enum (no logic)
│
├── analysis.rs      ✅ Analysis JSON schema — DocType, Claim, SuggestedPage,
│                       Contradiction, Action, Status, Dimension, PageType
├── markdown.rs      ✅ PageFrontmatter, parse_frontmatter, write_page,
│                       frontmatter_from_page, today_iso8601
│                       Phase 8: slug_for, resolve_slug, promote_to_bundle,
│                       is_bundle
├── config.rs        ✅ WikiConfig — per-wiki .wiki/config.toml
│
├── ingest.rs        ✅ Deserialise analysis.json → validate → call integrate → commit
│                       Phase 8: IngestReport gains bundles_created
├── integrate.rs     ✅ Write pages (create/update/append) + contradictions
│                       Phase 8: write_asset_colocated, write_asset_shared,
│                       assets_index_path, regenerate_assets_index
├── git.rs           ✅ init_if_needed, stage_all, commit via git2
├── init.rs          ✅ Phase 5 — init_wiki: git init + create dirs + config.toml
│
├── search.rs        ✅ tantivy index build + BM25 query + search_all (Phase 6)
│                       Phase 8: slug_for in walkers, bundle asset skipping,
│                       SearchResult gains path field, update_index uses resolve_slug
├── context.rs       ✅ top-K pages as Markdown context for an external LLM
│                       Phase 8: uses resolve_slug for page path resolution
│
├── lint.rs          ✅ structural audit: orphans, missing stubs, active contradictions
│                       Phase 8: orphan_asset_refs detection, LintReport gains field
├── graph.rs         ✅ petgraph concept graph → DOT / Mermaid output
│                       Phase 8: slug_for in walkers, bundle asset skipping,
│                       missing_stubs uses resolve_slug
├── contradiction.rs ✅ contradiction page list + filter by status
│
├── server.rs        ✅ Phase 4+6 — rmcp WikiServer — MCP tools + prompts + resources
│                       Phase 6: registry field, new_with_registry, multi-wiki tools,
│                       namespaced wiki:// URIs, SSE via SseServer::with_service
│                       Phase 8: resolve_slug in do_read_resource, bundle asset URIs,
│                       slug_for in list_pages_in_root
├── instructions.md  ✅ Phase 5 — embedded LLM guide (all 6 workflow sections complete)
└── registry.rs      ✅ Phase 6 — WikiRegistry, WikiEntry, load, resolve,
                        global_config_path, register_wiki
```

## Dependency graph

```
main ──▶ cli
     ──▶ ingest ──▶ config
                ──▶ analysis
                ──▶ integrate ──▶ analysis
                               ──▶ git
                               ──▶ markdown
     ──▶ search
     ──▶ context ──▶ search
     ──▶ lint    ──▶ graph
                 ──▶ contradiction
                 ──▶ git
     ──▶ graph
     ──▶ contradiction ──▶ analysis
     ──▶ git
     ──▶ init    ──▶ git
                 ──▶ config
     ──▶ server  ──▶ ingest
                 ──▶ context
                 ──▶ search
                 ──▶ lint
     ──▶ registry ──▶ config
```

No cycles. `analysis`, `config`, and `markdown` are leaf modules with no internal
dependencies.

## Implementation phases

| Phase | Status | Key module(s) |
|-------|--------|---------------|
| 0 | ✅ done | All — typed skeletons, no logic |
| 1 | ✅ done | `ingest`, `integrate`, `git`, `markdown` — `wiki ingest` works end-to-end |
| 2 | ✅ done | `search`, `context` — `wiki search` + `wiki context` work end-to-end |
| 3 | ✅ done | `lint`, `graph`, `contradiction` — `wiki lint`, `wiki contradict`, `wiki graph`, `wiki list`, `wiki diff` |
| 4 | ✅ done | `server` (rmcp MCP server — `wiki serve`, `wiki instruct`) |
| 5 | ✅ done | `init` (`wiki init`), `.claude-plugin/` commands + `plugin.json`, complete `instructions.md` |
| 6 | ✅ done | `registry` (multi-wiki registry, `--wiki` flag, `search_all`, SSE transport) |
| 7 | ✅ done | `search` (incremental index update, `open_or_build_index`, `update_index`) |
| 8 | ✅ done | `markdown` (slug_for, resolve_slug, promote_to_bundle), `integrate` (asset writing), all walkers updated, `wiki read` |
