# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — Unreleased

### Added

- **Privacy redaction** — `wiki_ingest` accepts `redact: true`; 6 built-in patterns (GitHub PAT, OpenAI key, Anthropic key, AWS access key, Bearer token, email); per-wiki `[redact]` in `wiki.toml` (disable built-ins, add custom patterns); `redacted: Vec<RedactionReport>` in `IngestReport`; body-only, lossy by design
- **Incremental validation** — `wiki_ingest` now validates only git-changed files since the last indexed commit; `unchanged_count` added to `IngestReport`; `dry_run: true` continues to validate all files; fallback to full validation when `last_commit` is absent or git errors
- **`wiki_lint` tool** — 5 deterministic index-based lint rules (`orphan`, `broken-link`, `missing-fields`, `stale`, `unknown-type`); JSON report with `findings`, `errors`, `warnings`, `total`; `lint` CLI subcommand exits non-zero on any `error` finding; `[lint]` config section with `stale_days` and `stale_confidence_threshold`
- **Backlinks** — `backlinks: true` parameter on `wiki_content_read`; returns JSON `{ content, backlinks: [{slug, title}] }` via a term query on the `body_links` index field; no file writes, no index mutation; empty array when no pages link to the target
- **Confidence field** — `confidence: 0.0–1.0` on every page; numeric tantivy fast field; legacy string values (`high` / `medium` / `low`) mapped automatically on read
- **Lifecycle-aware search ranking** — `tweak_score` collector multiplies BM25 score by `status_multiplier × confidence`; ranking formula: `final_score = bm25 × status × confidence`
- **`[search.status]` map in config** — flat `HashMap<String, f32>` replaces four named fields; built-in defaults (`active=1.0`, `draft=0.8`, `archived=0.3`, `unknown=0.9`); custom statuses (`verified`, `stub`, `deprecated`, …) added with no code change; per-wiki `wiki.toml` overrides individual keys (key-level merge, not all-or-nothing)
- **`claims[].confidence` as float** — aligned with page-level confidence; was string enum `high/medium/low`; now `0.0–1.0` in `concept` and `paper` schemas
- **`confidence: 0.5` in page scaffold** — `wiki_content_new` emits the field by default
- **`format: "llms"` on existing tools** — `wiki_list`, `wiki_search`, `wiki_graph` accept `format: "llms"`; produces LLM-optimised output (type-grouped pages with summaries, compact search results, natural language graph description) directly in the tool response
- **`wiki_export` tool** — new MCP tool and `llm-wiki export` CLI command; writes full wiki to a file (no pagination); formats: `llms-txt` (default), `llms-full` (with bodies), `json`; path relative to wiki root; response is a confirmation report
- **Lint guide** — `docs/guides/lint.md` covering all 5 rules, fix guidance, CI usage, and stale rule tuning
- **Redaction guide** — `docs/guides/redaction.md` covering built-in patterns, per-wiki config, and lossy-by-design warning
- **Search ranking guide** — `docs/guides/search-ranking.md` covering the formula, status map, per-wiki overrides, and custom status examples
- **Graph guide** — `docs/guides/graph.md` covering community detection, cross-cluster suggestions, and threshold tuning
- **Graph community detection** — Louvain clustering on `petgraph::DiGraph`; `communities` field in `wiki_stats` output (`count`, `largest`, `smallest`, `isolated` slugs); suppressed below `graph.min_nodes_for_communities` (default 30); deterministic via sorted-slug processing order
- **Community-aware suggestions** — strategy 4 in `wiki_suggest`: pages in the same Louvain community not already linked; score 0.4, reason `"same knowledge cluster"`; `graph.community_suggestions_limit` (default 2)
- **Cross-wiki links** — `wiki://name/slug` URIs as first-class link targets in frontmatter edge fields and body `[[wikilinks]]`; `ParsedLink` enum in `links.rs`; external placeholder nodes in single-wiki graph (dashed border); `build_graph_cross_wiki` for unified multi-wiki graph; `cross_wiki: bool` param on `wiki_graph` MCP tool and `--cross-wiki` CLI flag
- **`broken-cross-wiki-link` lint rule** — detects `wiki://` URIs pointing to unmounted wikis; reported as `Warning` (unmounted ≠ wrong)
- **Integration test fixtures** — `tests/fixtures/` with two wiki spaces (`research`, `notes`), 8 pre-built pages, and 5 inbox source documents covering paper, article, note, data, redaction, cross-wiki, and contradiction scenarios
- **Engine validation script** — `docs/testing/scripts/validate-engine.sh`; end-to-end CLI coverage of all 19+ tools including every v0.2.0 feature; pass/fail/skip report
- **Skills validation guide** — `docs/testing/validate-skills.md`; 12 interactive scenarios for validating the Claude plugin against the test fixtures

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
