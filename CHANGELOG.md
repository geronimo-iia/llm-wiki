# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — Unreleased

### Added

- **In-memory graph cache** — full wiki graph and Louvain community data cached per space, keyed on index generation; invalidated automatically after any index write; `wiki_graph`, `wiki_stats`, and `wiki_suggest` skip rebuild on cache hit in serve mode; cross-wiki path uses per-space cached graphs via `merge_cached_graphs`

- **ACP cooperative cancellation** — `AcpSession` carries a `cancelled: Arc<AtomicBool>` flag; the `cancel` notification handler sets the flag immediately; every workflow polls between steps (`research`: after search, `lint`: between each finding, `graph`/`ingest`: before dispatch); a `"Cancelled."` message is sent and the run exits cleanly; the flag resets to `false` on each new `Prompt`

- **ACP session cap** — `serve.acp_max_sessions` config key (default: 20, global-only); `NewSession` returns `InvalidParams` with `"Session limit reached (max: N)"` when the cap is exceeded; configurable via `llm-wiki config set serve.acp_max_sessions <n> --global`

- **ACP `ListSessions` active-run state** — sessions with an ongoing tool run are reported with a `[active]` prefix in the title field (e.g. `[active] my-session`); clients can distinguish idle from busy sessions without polling

- **Proactive watcher push** — `llm-wiki serve --acp --watch` now pushes `"Wiki \"<name>\" updated: <N> page(s) changed."` to all idle ACP sessions targeting the changed wiki after each watcher-triggered ingest; delivered via `tokio::sync::mpsc` from the watcher task; the ACP push task blocks on a `tokio::sync::watch` channel until the first `Prompt` establishes the connection handle — watcher events that arrive before the first prompt are buffered (channel capacity 64) and delivered once the connection is ready; sessions with an active run are skipped

## [0.2.0] — 2026-04-28

### Added

- **`wiki_resolve` tool** — resolves a slug or `wiki://` URI to its local filesystem path (`slug`, `wiki`, `wiki_root`, `path`, `exists`, `bundle`); enables direct file writes without MCP content round-trips (tool count: 21 → 22)
- **`wiki_content_new` returns JSON** — response now includes `uri`, `slug`, `path`, `wiki_root`, `bundle`; LLM gets the local path immediately after page creation with no follow-up `wiki_resolve` call
- **`LintFinding.path` field** — every lint finding now includes the absolute filesystem path to the offending file; enables direct `Edit` without a follow-up resolve call

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
- **Lint guide** — `docs/guides/lint.md` covering all 5 rules, fix guidance, CI usage, and stale rule tuning; `path` field documented in finding example
- **Redaction guide** — `docs/guides/redaction.md` covering built-in patterns, per-wiki config, and lossy-by-design warning
- **Search ranking guide** — `docs/guides/search-ranking.md` covering the formula, status map, per-wiki overrides, and custom status examples
- **Graph guide** — `docs/guides/graph.md` covering community detection, cross-cluster suggestions, and threshold tuning
- **Writing content guide** — `docs/guides/writing-content.md`; direct write pattern (`wiki_content_new` → write to `path` → `wiki_ingest`); `wiki_resolve` usage; backlinks; tool selection table
- **Guides README reorganized** — grouped by audience: Getting started / Writing and managing content / Configuration and integration / Search, graph, and output / Operations
- **Diagram #4 updated** — LLM Ingest Workflow diagram updated to show `wiki_list(format: "llms")`, `wiki_content_new` direct write, and post-ingest `wiki_lint` steps
- **Rustdoc pass** — all public items in the crate now have `///` documentation; zero `missing_docs` warnings
- **Graph community detection** — Louvain clustering on `petgraph::DiGraph`; `communities` field in `wiki_stats` output (`count`, `largest`, `smallest`, `isolated` slugs); suppressed below `graph.min_nodes_for_communities` (default 30); deterministic via sorted-slug processing order
- **Community-aware suggestions** — strategy 4 in `wiki_suggest`: pages in the same Louvain community not already linked; score 0.4, reason `"same knowledge cluster"`; `graph.community_suggestions_limit` (default 2)
- **Cross-wiki links** — `wiki://name/slug` URIs as first-class link targets in frontmatter edge fields and body `[[wikilinks]]`; `ParsedLink` enum in `links.rs`; external placeholder nodes in single-wiki graph (dashed border); `build_graph_cross_wiki` for unified multi-wiki graph; `cross_wiki: bool` param on `wiki_graph` MCP tool and `--cross-wiki` CLI flag
- **CommonMark body links** — `[text](slug)` and `[text](wiki://name/slug)` inline links in page bodies are now indexed alongside `[[wikilinks]]`; appear in `body_links`, `wiki_graph`, backlinks, and the `broken-link` lint rule; image links, external URLs, `mailto:`, and anchor-only links are filtered; `#anchor` suffixes stripped before indexing
- **`broken-cross-wiki-link` lint rule** — detects `wiki://` URIs pointing to unmounted wikis; reported as `Warning` (unmounted ≠ wrong)
- **Integration test fixtures** — `tests/fixtures/` with two wiki spaces (`research`, `notes`), 8 pre-built pages, and 5 inbox source documents covering paper, article, note, data, redaction, cross-wiki, and contradiction scenarios
- **Engine validation script** — `docs/testing/scripts/validate-engine.sh`; end-to-end CLI coverage of all 19+ tools including every v0.2.0 feature; pass/fail/skip report
- **Skills validation guide** — `docs/testing/validate-skills.md`; 12 interactive scenarios for validating the Claude plugin against the test fixtures
- **MCP validation suite** — `docs/testing/scripts/validate-mcp.sh`; end-to-end MCP coverage via mcptools stdio transport (52 tests across 11 sections mirroring the CLI suite); `lib/mcp-helpers.sh` with `run_mcp` / `run_mcp_json` helpers
- `--config <path>` global flag to override the config file path
- `LLM_WIKI_CONFIG` environment variable as a fallback config path override

### Fixed

- `llm-wiki stats` and any command using community detection hung indefinitely — `louvain_phase1` could oscillate forever when node moves mid-pass altered `sigma_tot` for subsequent nodes; capped at `n × 10` passes
- `SpaceIndexManager::status()` now uses `ReloadPolicy::Manual` to avoid spawning a competing file_watcher thread against the open `IndexReader`
- **IndexReader stale after rebuild in serve mode** — `rebuild()` opened a fresh `Index::open_or_create()` instance; with `ReloadPolicy::Manual`, `writer.commit()` only notifies readers on the same instance, so the held reader stayed frozen; added `reload_reader()` helper called after every `writer.commit()` in `rebuild()`, `update()`, `delete_by_type()`, and `rebuild_types()`; fixes `wiki_search` / `wiki_list` / `wiki_graph` returning stale results after `wiki_index_rebuild` in `llm-wiki serve`
- `wiki_graph` MCP tool now returns the rendered graph text (mermaid/dot/llms) instead of a bare stats report
- `validate-engine.sh` and `validate-mcp.sh` reset inbox fixtures and clear logs before each run for idempotent sequential execution

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
