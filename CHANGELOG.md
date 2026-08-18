# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

### Security

- **Path traversal in `type_registry.rs` closed** — `SpaceTypeRegistry::build` and
  `compute_disk_hashes` now validate every `schema` path from `wiki.toml` via
  `validate_schema_path`: absolute paths and `..` components are rejected, and the
  resolved path must be inside `repo_root`. Previously an adversarial
  `schema = "../../etc/passwd"` entry would read arbitrary files from the filesystem.
- **MCP parameter length limit** — all MCP tool calls are now rejected before dispatch
  if any string argument exceeds `serve.mcp_max_param_len` bytes (default: 8192).
  Configurable via `llm-wiki config set serve.mcp_max_param_len <n> --global`.
- **MCP slug validation** — `slug` arguments in `wiki_history` and `wiki_suggest` are
  now validated through `Slug::try_from` before reaching the ops layer, rejecting
  `..` components, hidden path segments, and invalid characters.
- **`redact_error` covers tilde-prefixed paths** — the path-scrubbing regex previously
  matched only absolute paths starting with `/`; `~/wikis/foo` and `~user/repo` would
  pass through unredacted. Extended to also match `~[…]{2,}`, so tilde-expanded paths
  are fully replaced with `<path>`.

### Dependencies

- `memmap2` updated — resolves RUSTSEC-2026-0186 (unchecked pointer offset).
- `event-listener` updated — resolves RUSTSEC-2026-0221 (`!Send` tags crossing thread
  boundaries via `StackSlot`).
- `h2` updated to 0.4.16 — resolves RUSTSEC-2026-0258 (HTTP/2 request smuggling via
  malformed `Content-Length`).
- `boxfnonce` RUSTSEC-2019-0040 suppression dropped — `agent-client-protocol` updated
  to v2.0.0 and no longer pulls in `boxfnonce`; the advisory entry is removed from
  `audit.toml`.
- `lru` RUSTSEC-2026-0253 (use-after-free in `LruCache::pop()`) — suppressed as an
  allowed warning. Upstream-blocked: `tantivy 0.26.1` pins `lru ^0.16.3` and no fixed
  version exists in that range. Will be resolved when `tantivy` bumps to `lru ^0.17`.
  See `docs/decisions/1.0.0/suppress-lru-rustsec-2026-0253.md`.
- `atomic-polyfill` RUSTSEC-2023-0089 (unmaintained) — suppressed as an allowed
  warning. Upstream-blocked via `postcard → heapless ^0.7.0` chain; crate is not
  compiled into the binary on any supported target. Will be resolved when `postcard`
  relaxes its `heapless` constraint to `^0.8` or later.
  See `docs/decisions/1.0.0/suppress-atomic-polyfill-rustsec-2023-0089.md`.

### Changed

- **Stable public API surface (Phase 5)** — `lib.rs` now re-exports the five primary
  types (`WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`, `WikiGraph`) at
  the crate root. Internal helpers are `pub(crate)`. `#![warn(unreachable_pub)]` enabled
  crate-wide to keep the boundary stable going forward.
- **`WikiEntry.path` and `WikiConfig.wiki_root` changed from `String` to `PathBuf`** —
  eliminates manual `PathBuf::from(&entry.path)` conversions throughout the codebase.
  TOML round-trips via `crate::pathutil::path_as_string` (UTF-8 string on disk,
  `PathBuf` in memory). Callers that stored `entry.path` as `String` must use
  `entry.path.display()` or `.to_string_lossy()`.
- **`PageRef.slug` and `PageSummary.slug` changed from `String` to `NormalizedSlug`** —
  `NormalizedSlug` is a newtype that carries the invariant "this slug is already
  lowercased". Serializes as a plain JSON string (no structure change for API consumers).
  Use `.as_str()` / `.to_string()` to extract the inner string; compare with `==`
  against `&str`, `String`, or `NormalizedSlug` directly.
- **`IndexStatus.degraded_reason`** — new optional field (`Option<String>`) on
  `wiki_index_status` JSON output. Present only when `stale`, `!openable`, or
  `!queryable`; omitted from JSON when `None`. Explains why the index is unhealthy.
- **`wiki_config` tool description expanded** — MCP `action` values (`"get"`, `"set"`,
  `"list"`), example key paths, and `--wiki` scoping behaviour are now documented in the
  tool description visible to LLM clients.
- **MCP error messages redact filesystem paths** — all tool error strings returned to
  LLM clients are processed by `redact_error`, replacing absolute paths with `<path>` to
  avoid leaking workspace layout. Tilde-prefixed paths (`~/…`) are now also covered
  (see Security above).
- **`wiki_info` degraded detail** — `index_status` in `wiki_info` responses now returns
  a per-wiki map when any wiki is degraded: `{"<name>": {"status": "degraded", "reason":
  "…; run wiki_index_rebuild to recover"}}` instead of the flat string `"degraded"`.
  LLM clients can surface the actionable reason directly.
- **`degraded_reason` includes remediation hint** — messages appended with
  `"; run wiki_index_rebuild to recover"` for `openable=false` and `queryable=false`
  cases so operators and LLM clients know what action to take.

- `serve.mcp_max_param_len` added to `ServeConfig` (default: 8192 bytes). Accessible
  via `wiki_config get/set serve.mcp_max_param_len` (global-only key).

### Concurrency

- **Blocking I/O moved off Tokio thread** — `schema_rebuild` is now dispatched via
  `tokio::task::spawn_blocking` in the watcher loop; file I/O, `git log`, and Tantivy
  fsync no longer block MCP request handling or ACP sessions during schema changes.
- **Read lock scope narrowed in `schema_rebuild`** — `state.read()` is held only long
  enough to clone `Arc<SpaceContext>`, then dropped before any I/O. `mount_wiki` and
  `unmount_wiki` (which need `state.write()`) are no longer blocked for the full
  rebuild duration.
- **Redundant concurrent rebuilds eliminated** — `SpaceContext` carries a per-wiki
  `Arc<AtomicBool>` rebuild guard; a second watcher event arriving while a rebuild is
  in progress is skipped with a `debug` log instead of queuing a redundant rebuild.
  See `docs/decisions/1.0.0/watcher-rebuild-guard-atomic-bool.md`.
- **ACP session mutex hardened** — `Sessions` now uses `parking_lot::Mutex` instead of
  `std::sync::Mutex`. A panic in a critical section no longer poisons the mutex and
  permanently crashes the ACP server task. Helper functions remain sync.
  See `docs/decisions/1.0.0/acp-sessions-parking-lot-mutex.md`.

### Correctness

- **`is_wiki_md` deleted** — the watcher previously silently dropped all `.md` events
  when `wiki_root` was not named `"wiki"`. The hardcoded `/wiki/` path check is
  replaced by `path.extension() == Some("md")`; the existing `starts_with(wiki_root)`
  guard already scopes events correctly.
- **Louvain HashMap lookups hardened** — bare `.unwrap()` on `community.get()` and
  `id_remap.get()` in `louvain_phase1` and `build_community_data` replaced with
  `.expect("...")` carrying an invariant message. A `debug_assert!` at
  `louvain_phase1` entry verifies the community map covers all adjacency nodes.
- **Rollback errors logged** — `spaces_create` and `spaces_register` previously called
  `let _ = spaces::remove(...)` on mount failure, silently discarding any error from
  the rollback itself. The error is now logged via `tracing::error!` so a stranded
  config entry is visible in logs.
- **Startup index failures promoted to `error`** — permanent degradation at engine
  startup (`build_space` failure, unrecoverable `open()`) was logged at `warn`;
  promoted to `error` so operators are not misled into thinking the condition is
  transient. Incremental watcher failures remain `warn` (the watcher retries on the
  next commit).
- **Resource notification failures correlated to source operation** — `tracing::warn!`
  for failed MCP resource-updated and resource-list-changed notifications now carries a
  `tool` structured field, allowing log correlation back to the originating tool call.
- **Stale-dir removal error context includes path** — the two `.context("…")` sites in
  `index_manager.rs` that remove stale directories now use `.with_context(|| format!("…
  at {}", dir.display()))` so the failing path is visible in the error chain.

### Performance

- **Louvain community detection correctness + O(N³) fix** — `louvain_phase1` previously
  used an incomplete gain formula (join-only, missing the leave cost), allowing
  modularity-decreasing moves and causing oscillation that hit the pass cap without
  converging. The full Louvain ΔQ formula is now implemented: `net_gain = join_gain −
  leave_gain`. Additionally, `sigma_tot` is precomputed once per pass (O(N)) and
  updated incrementally on each move instead of being rebuilt per node (O(N²) per
  pass). Combined fix: correctness restored, complexity reduced from O(N³) to O(M)
  per pass. Regression test `test_louvain_two_clusters` added.
  See `docs/decisions/1.0.0/louvain-sigma-tot-precompute.md`.
- **Graph truncation warning** — `build_graph` now emits `tracing::warn!` when
  `TopDocs::with_limit(100_000)` is reached, making silent graph truncation visible
  to operators on wikis with >100 000 pages.
- **Accurate page count after incremental index update** — `update()` previously wrote
  `pages: 0` to `state.toml` after every watcher-triggered incremental update;
  `wiki_index_status` always showed 0 pages. Now reads the actual total via
  `searcher.num_docs()` after `reload_reader()`.
- **`subgraph` edge scan O(E_subgraph)** — previously iterated over every edge in the
  full graph (`E_total`) to find edges within the subgraph; now uses
  `graph.edges_directed(node, Outgoing)` to walk only edges reachable from visited
  nodes, reducing work to O(E_subgraph).
- **`build_graph_cross_wiki` builds each wiki graph once** — previously called
  `get_or_build_graph` inside the per-page loop, re-entering the cache on every page;
  moved to a pre-build phase so each wiki graph is fetched exactly once.
- **`redact_error` regex compiled once** — regex previously compiled on every call site
  invocation; hoisted to a `LazyLock<Regex>` static so compilation happens once at
  first use.

### Documentation

- **Lenient query parser fallback documented** — both `parse_query_lenient` call sites in
  `search.rs` now carry a comment explaining why the lenient parser is used (Tantivy
  rejects free-text queries containing `:` or field specifiers) and pointing to the
  pinning test.

- **`generation()` cache-key contract documented** — added code comments in
  `get_or_build_graph`, `get_cached_community_map`, and `get_cached_community_stats`
  explaining why `generation()` is used as the cache invalidation key instead of
  `last_commit()`: same-commit schema-triggered rebuilds produce a new index without
  changing the commit hash and must still invalidate downstream graph caches.
  Extended the `generation()` doc comment in `index_manager.rs` with the same
  rationale.

### Tests

- **Colon-query regression test** — Python integration test for `parse_query_lenient`
  fallback path (fixed 0.5.6): `search "Layer 1: Attention"` must exit 0 even though
  the colon would fail Tantivy's strict query parser.
- **Cross-wiki body link lint regression test** — Python integration test for the
  false-positive fix (fixed 0.5.9): `[text](wiki://other/slug)` in a body link must
  not trigger `broken-link` when the target wiki is mounted.
- **`spaces set-default` config state assertion** — `test_spaces_set_default` now also
  asserts `config get global.default_wiki` matches the newly-set wiki, verifying
  the engine persists the change and not just the display layer.
- **Invariant-pinning tests (review 2026-08-17)** — unit and integration tests added to
  pin previously untested invariants:
  - `NormalizedSlug`: case-folding, traversal rejection, hidden-component rejection,
    extension rejection, round-trips, and `PartialEq` impls (`src/slug.rs`).
  - `set_default` atomicity: `spaces_set_default` failure must not update disk config
    (`tests/ops/hot_reload.rs`).
  - Graph snapshot key: snapshot filename must contain the git SHA, not the generation
    counter (`tests/graph_snapshot.rs`).
  - `redact_error`: absolute paths, multiple paths, short paths, and tilde paths
    (`src/mcp/handlers.rs`).


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
