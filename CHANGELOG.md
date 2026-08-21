# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Security

Path traversal, info-disclosure, and oversized-input vectors closed across the MCP surface. Absolute filesystem paths (index path, config path, error strings) no longer leak to LLM clients — all tool errors are redacted before dispatch. MCP string parameters are bounded at 8 192 bytes by default (`serve.mcp_max_param_len`); content writes are capped at 10 MB. Slug inputs in `wiki_history`, `wiki_suggest`, `wiki_content_commit`, and `resolve_read_target` validated before any filesystem access. Three CVEs resolved in dependencies (`memmap2`, `event-listener`, `h2`).

### Added

- **`wiki_graph` JSON format** — `format: "json"` returns nodes, edges, metrics, and community map as structured JSON; machine-readable alternative to Mermaid/DOT.
- **Library crate renamed to `llm_wiki_engine`** — embed the engine in your own binary with `use llm_wiki_engine::…`; a minimal `examples/embed.rs` shows the pattern.

### Changed

- **Actionable error messages** — `wiki_search` on a closed index, `wiki_content_commit` with nothing staged, and ACP session-limit rejections now include the exact command to run to recover.
- **`wiki_index_status` degraded detail** — per-wiki degraded map with a `"; run wiki_index_rebuild to recover"` hint replaces the flat `"degraded"` string.
- **More configurable, fewer magic numbers** — content size limit, index memory budget, suggest strategy weights, graph depth default, community min-nodes, and ACP `top_k` are all now tunable in `config.toml`; previous hardcoded values become the documented defaults.

### Fixed

- **Concurrent index rebuilds no longer corrupt state** — overlapping watcher-triggered and API-triggered rebuilds are serialized; the `rebuilding` flag is set and cleared atomically.
- **`spaces_set_default` rollback on disk failure** — in-memory default is restored if the config write fails; previously the in-memory and on-disk states could diverge.
- **Watcher silently swallowing errors** — filesystem watcher errors, ingest git-diff failures, cross-wiki search errors, and staleness-check failures all now emit `tracing::warn!`; operators can diagnose unexpected full rebuilds and inotify exhaustion.
- **ACP server no longer crashes on mutex poison** — `parking_lot::Mutex` replaces `std::sync::Mutex`; a panicking session no longer takes down the whole server.
- **Blocking Tantivy I/O off the async thread** — `schema_rebuild` and index writes dispatched via `spawn_blocking`/`block_in_place`; no longer risks stalling the Tokio runtime.

### Performance

- **`wiki_suggest` ~70 → ~5 queries per call** on a 1 000-page wiki — per-candidate doc fetches replaced with bulk `TermSetQuery` per strategy.
- **`wiki_lint` 5× fewer Tantivy passes** — five lint rules share one `AllQuery + N doc reads` instead of 5 AllQuery + 8×N reads.
- **`wiki_stats` staleness with zero doc reads** — `StalenessCollector` reads `last_updated` via FAST column; eliminates ~1 300 `searcher.doc()` calls per call.
- **Search and list: 4 → 2 Tantivy query passes** — facet collection ported to fast-field collector.
- **Louvain community detection correctness and speed restored** — full ΔQ formula with incremental `sigma_tot`; was O(N³), now O(M).
- **Index rebuild triggers** — `type` and `last_updated` fields promoted to FAST storage; automatic rebuild on deploy picks up both changes.

## [0.5.9] — 2026-08-17

### Fixed

- `wiki_spaces_create`: `set_default=true` now updates the in-memory engine default without requiring a restart ([#131](https://github.com/geronimo-iia/llm-wiki/issues/131))
- `wiki_spaces_create` / `wiki_spaces_register`: config entry is rolled back when `mount_wiki` fails, preventing a registered-but-unmountable wiki from stranding the server ([#131](https://github.com/geronimo-iia/llm-wiki/issues/131))
- `wiki_lint`: cross-wiki body links `[text](wiki://name/slug)` no longer produce
  false `broken-link` errors when the target wiki is mounted. Previously the
  `wiki://` prefix was stripped before indexing, losing the routing information.
  As a side effect, `wiki_graph` now produces correct edges for cross-wiki body
  links (`graph.rs:resolve_or_external` already handled `wiki://` prefixes).
  **Action required:** run `llm-wiki index rebuild` on any wiki that uses
  `wiki://` body links to pick up the corrected `body_links` index entries. ([#132](https://github.com/geronimo-iia/llm-wiki/issues/132))

## [0.5.8] — 2026-08-16

### Fixed

- `wiki_graph` mermaid output: node IDs now derived from petgraph `NodeIndex` (`N0`, `N1`, …) — eliminates parse errors from spaces and special characters in page titles; labels unchanged ([#128](https://github.com/geronimo-iia/llm-wiki/issues/128))
- `[[wikilinks]]` and `[text](dest)` links inside fenced code blocks and
  inline code spans are no longer extracted — TOML `[[section]]` headers
  and code examples no longer produce false broken-link findings
  ([#127](https://github.com/geronimo-iia/llm-wiki/issues/127))

### Changed

- Body link extraction now uses `pulldown-cmark` instead of a manual text
  walker — all public APIs unchanged

## [0.5.7] — 2026-08-15

### Added

- `wiki_info` MCP tool (no arguments) — returns server `version`, `config_path`, registered `spaces`, `default_wiki`, and `index_status` ("ok" / "degraded") ([#122](https://github.com/geronimo-iia/llm-wiki/issues/122))

### Fixed

- **CommonMark relative links stored raw in body_links** — `[text](./page.md)` and `[text](../path.md)` destinations were written verbatim into the Tantivy `body_links` field; the lint checker, orphan rule, and graph builder all compare against slugs (no `.md`, no relative prefix), so nothing matched. Destinations are now normalized at index time: `.md` is stripped, `./` and `../` are resolved against the source page's containing directory, computed correctly for both flat pages (`parent/slug.md` → dir is `parent/`) and bundle pages (`parent/slug/index.md` → dir is `parent/slug/`) (issue #124).

## [0.5.6] — 2026-08-14

### Fixed

- **search query with `:` caused parse error** — tantivy treats `:` as field separator, so free-text queries like `"Layer 1: ..."` failed with `Field does not exist`; `search()` now falls back to `parse_query_lenient` when strict parsing fails (issue #120)

## [0.5.5] — 2026-08-12

### Changed

- **rmcp 2.x → 3.x** — updated `ServerHandler` impl (`src/mcp/mod.rs`) to use `CallToolResponse`, `ReadResourceResponse` return types and `..Default::default()` for `ListToolsResult`/`ListResourcesResult` struct literals, required by the new response-envelope model introduced in rmcp 3.0

## [0.5.4] — 2026-08-04

### Fixed

- **`graph` stale snapshot on fresh process** — `llm-wiki graph` returned an empty graph after index rebuild because the snapshot key was the in-memory `generation` counter (always `0` on startup); replaced with `last_commit()` (git HEAD SHA from `state.toml`), which is stable across restarts and changes on every index rebuild (issue #112)
- **`index rebuild` graph cache not refreshed** — CLI `index rebuild` did not refresh the graph snapshot after rebuilding the tantivy index; now matches MCP handler behaviour by calling `graph_cache.rebuild()` after each index rebuild

### Changed

- **`ops::index_rebuild` owns graph cache refresh** — graph snapshot refresh moved from `handle_index_rebuild` (MCP handler) into `ops::index_rebuild`; CLI and MCP now share identical post-rebuild behaviour with no duplication

## [0.5.3] — 2026-07-31

### Changed

- **`agent-client-protocol` 2.0** — bumped from 1.3; migrated ACP catch-all dispatch handler to 2.0 variant-match API (`Dispatch::respond_with_error` removed upstream)

## [0.5.2] — 2026-07-31

### Fixed

- **Windows verbatim path prefix** — `std::fs::canonicalize` returns `\\?\`-prefixed paths on Windows; stripped before use so `file://` URLs are valid; UNC paths (`\\?\UNC\srv\share`) correctly normalised to `\\srv\share` (thanks to [@cristianm123](https://github.com/cristianm123))
- **`validate_wiki_root` absolute path on Windows** — `/absolute` paths without a drive letter were not rejected by `is_absolute()`; added explicit `starts_with('/')` guard
- **integration tests `is_error` attribute** — MCP `call_raw` helper used camelCase `isError` instead of snake_case `is_error`; negative-path tests always reported success

## [0.5.1] — 2026-07-25

### Fixed

- **`search` tags** — multi-word tags (e.g. `"machine learning"`) were split on whitespace; multi-value tags truncated to first value; uppercase variants not normalized; all corrected — tags stored per-value and lowercased at index time; triggers automatic full index rebuild on next open
- **`frontmatter::parse` silent YAML failure** — malformed frontmatter returned empty data with no diagnostic; now logs a warning with the file path and continues indexing; `parse()` gains an `Option<&Path>` parameter (see Semver note)
- **`frontmatter::write` panic** — non-serializable values caused a hard panic; `write()` now returns `Result<String>` (see Semver note)
- **`index_manager` atomic rebuild** — rebuild promotes via atomic renames; live index untouched until commit succeeds; `reload_reader()` failure triggers full rollback instead of silently serving stale results; misleading "manual intervention required" log no longer emitted on first-ever build
- **`export` CRLF frontmatter** — `\r` leaked into page body on Windows line endings; fixed
- **`export` bundle body not loaded** — bundle pages (`{slug}/index.md`) had body silently skipped; now resolved correctly alongside flat pages
- **`export` stale index entry** — missing on-disk page now emits a warning instead of silently leaving body empty
- **`export` 100k page limit** — silent truncation at 100k results now emits a warning
- **`ops/content` path traversal** — type names with `..` components in `resolve_body_template` could read outside the schemas directory; rejected at input
- **`markdown::promote_to_bundle` missing pre-checks** — panicked on missing source or existing bundle destination; now returns descriptive errors before any mutation
- **`frontmatter::confidence` NaN** — `confidence: .nan` returned `Some(NaN)`; now returns `None`
- **`frontmatter::parse` empty frontmatter block** — `"---\n---\n"` returned raw content as body; now parsed correctly
- **`search::list` page_size=0 panic** — `page_size: 0` reached tantivy's `TopDocs::with_limit(0)`; now returns an error
- **`slug` path traversal** — bare `..` and `concepts/..` were accepted; rejected via `Component::ParentDir` check
- **`slug` dotfile components** — hidden path segments (`.env`, `concepts/.hidden`) were accepted; now rejected
- **`index_manager`/`git` wrong-prefix fallback** — silent `unwrap_or("wiki")` on prefix mismatch replaced with explicit error propagation
- **`lint` stale rule false positive** — draft and archived pages were flagged as stale; rule now applies only to `status: active` pages

### Semver note

`frontmatter::parse` and `frontmatter::write` are `pub` — signature changes ship in this patch because the previous signatures were latent panics, not stable contracts.

## [0.5.0] — 2026-07-23

### Added

- **`config set/get search.status.<key>`** — `search.status` multipliers are now accessible via dot-notation in `config set`/`config get`; custom status keys (e.g. `superseded`) are supported alongside built-ins; works at global and per-wiki scope (ported from [como-technologies/llm-wiki#6](https://github.com/como-technologies/llm-wiki/pull/6))
- **`export --format json` custom frontmatter** — JSON export now includes a `frontmatter` object per page with all frontmatter fields not already surfaced at the top level (e.g. `created`, `reference`, `deciders`); object is omitted when the page has no extra fields (ported from [como-technologies/llm-wiki#11](https://github.com/como-technologies/llm-wiki/pull/11))

### Fixed

- **Windows slug separator** — nested pages now always use forward-slash slugs (`components/foo` not `components\foo`); fixes broken graph edges, false `wiki_lint` broken-link errors, and duplicate search results on Windows (#91)
- **Engine type-registry build failure** — panic on startup when the type registry fails to build now surfaces as a clear error instead of a silent crash
- **`schema add` truncation** — adding a schema no longer truncates a source file that lives inside the `schemas/` directory
- **Search index stale after `schema add`** — search index is now rebuilt after `schema add` so new schema fields are immediately queryable
- **Confidence indexing** — pages without a `confidence` frontmatter field are no longer indexed with a fabricated `0.5`; absence is now represented as absent in the index

### Changed

- **CI: `cargo-audit`** — replaced slow `cargo install cargo-audit` with `taiki-e/install-action@v2` (prebuilt binary) in `ci.yml`; added `test` job (fmt + clippy + tests + audit) as gate before release builds in `release.yml`

### Dependencies

- Bump `petgraph-live` `0.4.0` → `0.5.0`; snapshot format migrated from bincode to postcard — existing snapshots detected as `LegacyFormat` and transparently rebuilt on first use

- Bump `agent-client-protocol` `0.15` → `1.3`; migrate all `schema::*` imports to `schema::v1::*`
- Bump `rmcp` `1.8` → `2.2`; `Content` → `ContentBlock`, `RawResource` → `Resource`, drop `AnnotateAble`, use `ResourceUpdatedNotificationParam::new()`
- Bump `crossbeam-epoch` `0.9.18` → `0.9.20` (fixes RUSTSEC-2026-0204: invalid pointer dereference)

## [0.4.1] -  2026-05-08

### Added

- **pytest integration suite** — `tests-integration/` replaces bash test scripts; three suites: `engine/` (CLI subprocess), `mcp/` (MCP stdio via official `mcp` Python SDK), `acp/` (ACP NDJSON stdio via `asyncio`); managed by `uv`; root `Makefile` targets `validate-py`, `validate-py-engine`, `validate-py-mcp`, `validate-py-acp`
- **GitHub Actions integration workflow** — `.github/workflows/integration.yml` runs the pytest suite on pushes/PRs touching `src/**` or `tests-integration/**`; `workflow_dispatch` with `suite` input (`all`, `engine`, `mcp`, `acp`)

### Changed

- **MCP integration test quality** — hardened `tests-integration/mcp/` from smoke tests into correctness tests: centralized `rebuild()` helper on `McpEnv`; replaced hard-coded slugs/space names with `conftest` constants; strengthened assertions to validate JSON structure and field types; parametrized lint/structural rule tests; added `call_raw()` helper and negative-path tests for missing pages and bad input

### Fixed
- `wiki_graph` / `graph` now rejects unknown `format` values with an error instead of silently falling back to Mermaid; valid values are `mermaid`, `dot`, `llms`
- `schema show` no longer fails when a schema file is absent from disk — falls back to embedded defaults
- `spaces register` now calls `ensure_structure`, creating `wiki.toml` and the
  standard directory scaffold (`inbox/`, `raw/`, `schemas/`, content dir) when
  absent, matching the behaviour of `spaces create` (fixes #62)

## [0.4.0] — 2026-05-03

### Added

- `wiki_lint` rules: `articulation-point`, `bridge`, `periphery` — structural graph health
- `wiki_stats` fields: `diameter`, `radius`, `center`, `structural_note` — aggregate topology metrics
- `graph.structural_algorithms` config key (default `true`) — enable/disable structural fields in `wiki_stats`
- `graph.max_nodes_for_diameter` config key (default 2000) — guards O(n²) algorithms

### Changed

- **petgraph-live 0.3.1** — bumped dependency; snapshot directory creation now handled by the library (removed manual `create_dir_all` workaround in `mount_space`)
- **Snapshot zstd format** — `bincode+zstd` now valid `graph.snapshot_format` value; requires `snapshot-zstd` feature (enabled)
- **Graph cold-build cost reduced** — `build_fn` closure now captures `IndexSchema` (by clone) and `Arc<SpaceTypeRegistry>` directly; eliminates schema re-parse per cold build; `SpaceContext.type_registry` is now `Arc<SpaceTypeRegistry>`
- **Graph warm-start** — `SpaceContext.graph_cache` replaced with `WikiGraphCache` enum; `WithSnapshot` variant uses `petgraph_live::live::GraphState` to persist the graph to disk and reload on process restart; cold builds only on first launch or after `wiki_index_rebuild`; `graph.snapshot = false` disables (preserves Phase 1 behaviour)
- **Graph cache** — replaced bespoke `CachedGraph` + `RwLock<Option<CachedGraph>>` with `petgraph_live::GenerationCache<WikiGraph>` and `GenerationCache<CommunityData>`; `SpaceContext` no longer requires an explicit `RwLock` wrapper for the graph cache; zero behaviour change

## [0.3.0] — 2026-05-01

### Added

- **ACP workflows** — six built-in workflows dispatched by `llm-wiki:` prefix: `research`, `lint`, `graph`, `ingest`, `use`, `help`; `step_read` streams page body directly into the IDE; bare prompts fall through to `research`; `--http` flag required alongside `--acp` to give ACP exclusive stdio (MCP displaces to HTTP port)
- **In-memory graph cache** — full wiki graph and Louvain community data cached per space, keyed on index generation; invalidated automatically after any index write; `wiki_graph`, `wiki_stats`, and `wiki_suggest` skip rebuild on cache hit in serve mode; cross-wiki path uses per-space cached graphs via `merge_cached_graphs`
- **ACP cooperative cancellation** — `AcpSession` carries a `cancelled: Arc<AtomicBool>` flag; the `cancel` notification handler sets the flag immediately; every workflow polls between steps (`research`: after search, `lint`: between each finding, `graph`/`ingest`: before dispatch); a `"Cancelled."` message is sent and the run exits cleanly; the flag resets to `false` on each new `Prompt`
- **ACP session cap** — `serve.acp_max_sessions` config key (default: 20, global-only); `NewSession` returns `InvalidParams` with `"Session limit reached (max: N)"` when the cap is exceeded; configurable via `llm-wiki config set serve.acp_max_sessions <n> --global`
- **ACP `ListSessions` active-run state** — sessions with an ongoing tool run are reported with a `[active]` prefix in the title field (e.g. `[active] my-session`); clients can distinguish idle from busy sessions without polling
- **Proactive watcher push** — `llm-wiki serve --acp --watch` now pushes `"Wiki \"<name>\" updated: <N> page(s) changed."` to all idle ACP sessions targeting the changed wiki after each watcher-triggered ingest; delivered via `tokio::sync::mpsc` from the watcher task; the ACP push task blocks on a `tokio::sync::watch` channel until the first `Prompt` establishes the connection handle — watcher events that arrive before the first prompt are buffered (channel capacity 64) and delivered once the connection is ready; sessions with an active run are skipped
- **Configurable `wiki_root`** — `wiki_root` key in `wiki.toml` (default `"wiki"`); all hardcoded `wiki/` paths replaced by `SpaceContext.wiki_root`; supports multi-component paths (e.g. `"src/wiki"`); validated at registration time using canonicalized paths (symlink-safe, reserved-dir checks); zero behavior change for existing wikis
- **`wiki_spaces_register` tool** — new MCP tool and `llm-wiki spaces register` CLI subcommand; registers a pre-existing repository without creating files or git commits; validates `wiki_root` exists before completing; errors on conflicting `--wiki-root` vs `wiki.toml` value (no `--force`); hot-mounts the wiki if the server is running (tool count: 22 → 23)
- **`--wiki-root` flag on `spaces create`** — creates the specified directory instead of `wiki/`; writes `wiki_root` into the generated `wiki.toml` when non-default
- **ACP validation suite** — `docs/testing/scripts/validate-acp.sh` + per-section scripts in `docs/testing/scripts/acp/`; `setup-test-env.sh` configures ACP test settings; integrated into `.github/workflows/integration.yml` as `suite: acp`

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
