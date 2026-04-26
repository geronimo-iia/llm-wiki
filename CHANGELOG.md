# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] — 2026-04-26

### Fixed

- Renamed crate to `llm-wiki-engine` on crates.io (name `llm-wiki` was
  unavailable); binary name `llm-wiki` is unchanged
- Updated `cargo install` instructions in README and install scripts
- Vendored libgit2 and disabled SSH feature to remove OpenSSL system
  dependency (fixes cross-platform CI builds)
- Committed `Cargo.lock` — required for reproducible binary builds

## [0.1.0] — 2026-04-26

First release. Single Rust binary, 19 MCP tools, ACP agent.

### Engine

- `WikiEngine` / `EngineState` architecture with `mount_wiki` per space
- `Arc<SpaceContext>` in wiki map — in-flight requests survive unmount
- Hot reload — `mount_wiki` / `unmount_wiki` / `set_default` at runtime
- Interior mutability in `SpaceIndexManager` (`RwLock<IndexInner>`)
- Graceful shutdown via `watch` channel + `AtomicBool` across all transports
- tantivy 0.26 for full-text search
- Sorted list pagination via `order_by_string_fast_field` on slug

### ACP

- ACP agent via `agent-client-protocol` 0.11 builder pattern
- Session management — create, load, list, cancel
- Prompt dispatch — `llm-wiki:research <query>` prefix convention
- Streaming workflow steps — search, read, report results
- `src/acp/` module — helpers, research, server

### Tools — Space Management

- `wiki_spaces_create` — initialize wiki repo + register space (hot-reloaded if server running)
- `wiki_spaces_list` — list registered wikis
- `wiki_spaces_remove` — unregister (optionally delete, unmounted if server running)
- `wiki_spaces_set_default` — set default wiki (updated immediately if server running)

### Tools — Configuration

- `wiki_config` — get, set, list config values (global + per-wiki)
- `wiki_schema` — list, show, add, remove, validate type schemas

### Tools — Content

- `wiki_content_read` — read page by slug or `wiki://` URI
- `wiki_content_write` — write file into wiki tree
- `wiki_content_new` — create page or section with scaffolded frontmatter
- `wiki_content_commit` — commit pending changes to git

### Tools — Search & Index

- `wiki_search` — BM25 search with type filter and cross-wiki support
- `wiki_watch` — filesystem watcher, auto-ingest on save, smart schema rebuild
- Page body templates — `schemas/<type>.md` naming convention, fallback chain
- `wiki_stats` — wiki health dashboard (orphans, connectivity, staleness)
- `wiki_suggest` — suggest related pages to link (tag overlap, graph, BM25)
- `wiki_history` — git commit history for a page (trust, staleness, session tracking)
- `wiki_search` facets — always-on type/status/tags distributions, hybrid filtering
- `wiki_list` — paginated listing with type/status filters, sorted by slug, with facets
- `wiki_ingest` — validate frontmatter, update index, commit
- `wiki_graph` — concept graph in Mermaid or DOT with relation filtering
- `wiki_index_rebuild` — full index rebuild from committed files
- `wiki_index_status` — index health check

### Type System

- JSON Schema validation per page type (Draft 2020-12)
- Type discovery from `schemas/*.json` via `x-wiki-types`
- `wiki.toml` `[types.*]` overrides
- Field aliasing via `x-index-aliases`
- Typed graph edges via `x-graph-edges` (fed-by, depends-on, cites, etc.)
- Schema change detection with per-type hashing
- Embedded default schemas (base, concept, paper, skill, doc, section)
- Edge target type warnings on ingest

### Server

- MCP stdio transport (always on)
- MCP Streamable HTTP transport (opt-in, retry on bind failure)
- ACP transport (opt-in, runs as tokio task)
- `async-trait` removed (was only used for ACP `Agent` trait)
- Panic isolation (`catch_unwind` around tool dispatch)
- File logging with rotation (daily/hourly/never, max files, text/json)
- Heartbeat task (configurable interval)
- MCP resource listing and update notifications
- MCP `notifications/resources/list_changed` on space operations

### Index

- Dynamic tantivy schema computed from type registry
- FAST on all keyword fields for filtering and facet counting
- Rust 1.95 MSRV
- Incremental update via two-diff merge (working tree + committed changes)
- Partial rebuild per changed type
- Auto-recovery on index corruption
- Staleness detection (`StalenessKind` enum)
- Skip warnings with `tracing::warn` + `skipped` count in `IndexReport`

### CLI-only

- `llm-wiki logs tail/list/clear` — log file management
- `llm-wiki serve --dry-run` — show what would start

### Distribution

- `cargo install llm-wiki`
- `cargo binstall llm-wiki` (pre-built binaries)
- Homebrew tap (`brew install geronimo-iia/tap/llm-wiki`)
- asdf plugin (`asdf install llm-wiki latest`)
- `install.sh` (macOS/Linux) and `install.ps1` (Windows)
- GitHub Actions CI + release workflows
