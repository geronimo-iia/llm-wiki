# Decisions

Architectural decisions and their rationale, grouped by release.

## v1.0.0

### Concurrency

| Decision | Summary |
| -------- | ------- |
| [acp-sessions-parking-lot-mutex](1.0.0/acp-sessions-parking-lot-mutex.md) | `parking_lot::Mutex` for ACP `Sessions` — `tokio::sync::Mutex` rejected because helper functions are sync; `std::sync::Mutex` rejected due to poison crash vector; `parking_lot` already in transitive tree, zero new packages |
| [watcher-rebuild-guard-atomic-bool](1.0.0/watcher-rebuild-guard-atomic-bool.md) | `Arc<AtomicBool>` per `SpaceContext` to skip redundant concurrent rebuilds — `JoinHandle` abort rejected (Tantivy non-cancellable); per-wiki watch channel rejected (disproportionate); flag resets on re-mount; stuck-flag edge case on shutdown is benign |
| [rebuild-lock-mutex-space-index-manager](1.0.0/rebuild-lock-mutex-space-index-manager.md) | `Mutex<()>` on `SpaceIndexManager` serialises concurrent `rebuild()` calls — Tantivy writer lock rejected (different `Index` instances); `state` write-lock rejected (blocks all reads); per-wiki channel rejected (disproportionate); complements `AtomicBool` watcher guard |
| [spaces-atomic-rollback](1.0.0/spaces-atomic-rollback.md) | Direct `state.write()` for `set_default` rollback — `set_default("")` rejected (fails `contains_key` for empty string); rollback runs inside `with_config_lock` closure to prevent concurrent observers seeing intermediate state |

### Stable API

| Decision | Summary |
| -------- | ------- |
| [binary-and-library-crate](1.0.0/binary-and-library-crate.md) | Keep `[lib]` target — embedding value (non-trivial engine reuse without forking), test architecture (direct unit testing of internals), docs.rs façade; stable surface = 5 re-exported types; internal modules remain accessible to test layer until Post-1.0 refactor |
| [normalized-slug-newtype](1.0.0/normalized-slug-newtype.md) | `NormalizedSlug(String)` newtype — slug normalisation was convention-only; `Slug::normalize()` is the only public constructor; `from_normalized` bypass for internal index reads; `PartialEq<str>` impls keep test assertions unchanged; serializes as plain string |
| [pub-crate-partial-migration](1.0.0/pub-crate-partial-migration.md) | Only 4 of 22 modules converted to `pub(crate)` (`cli`, `server`, `watch`, `pathutil`) — 18 remain `pub mod` because `tests/*.rs` imports them directly; `#[allow(unreachable_pub)]` per module suppresses noise; full migration deferred to Post-1.0 test-layer refactor |
| [wiki-graph-json-format](1.0.0/wiki-graph-json-format.md) | `json` format for `wiki_graph` deferred to Post-1.0 — `GraphReport` is already `Serialize` so implementation cost is near-zero, but field names must be stable before exposing as a versioned JSON contract |
| [wiki-lint-scalability-parameters](1.0.0/wiki-lint-scalability-parameters.md) | Add `summary`, `path_prefix`, `page_size`, `cursor` to `wiki_lint` — full-wiki runs at 1,000+ pages exceed LLM context budgets; four parameters used in combination (summary → rule+prefix scope → paginate) make large wikis workable without server-side state |
| [wiki-graph-scale-summary-format](1.0.0/wiki-graph-scale-summary-format.md) | Add `format: "summary"` to `wiki_graph` — unfiltered `mermaid`/`dot`/`json` on 1,315 nodes is 270–450KB; summary returns aggregate metrics only (<2KB); also cap isolated titles in `format: "llms"` at 20; document scoped-first call sequence |
| [wiki-stats-detail-parameter](1.0.0/wiki-stats-detail-parameter.md) | Remove `communities.isolated: Vec<String>` from `CommunityStats` permanently — redundant with `wiki_lint rules: "periphery,orphan"` and a divergent second source of truth; add `detail: "summary" \| "full"` gating `center` slug list; fixes 275KB response at 1,315 pages; also fix `structural_note` silent null when `structural_algorithms: false` |

### Performance

| Decision | Summary |
| -------- | ------- |
| [louvain-sigma-tot-precompute](1.0.0/louvain-sigma-tot-precompute.md) | Full Louvain ΔQ formula (join gain − leave cost) + `sigma_tot` precomputed per pass — original formula was incomplete (join-only), causing oscillation and wrong partitions; `test_louvain_two_clusters` failed on original code; formula fix is correctness, sigma_tot is performance (O(N³)→O(M)); pass cap retained |
| [fast-field-facet-collector](1.0.0/fast-field-facet-collector.md) | `KeywordFacetCollector` + `StrColumn` fast fields for facet counting — built-in `FacetCollector` rejected (wrong tantivy type for STRING fields); accumulate-in-TopDocs rejected (top-K only); `MultiCollector` reduces search/list from 4 to 2 segment passes; `type` field schema bug fixed as prerequisite |
| [last-updated-keyword-fast-column](1.0.0/last-updated-keyword-fast-column.md) | `last_updated` promoted from TEXT to `STRING\|STORED\|FAST` — enables `StalenessCollector` to read ISO 8601 dates via `StrColumn` with zero `searcher.doc()` calls; `title` evaluated for same promotion but rejected (in `QueryParser` field list, keyword would break word-level title search) |

### Dependency hygiene

| Decision | Summary |
| -------- | ------- |
| [suppress-lru-rustsec-2026-0253](1.0.0/suppress-lru-rustsec-2026-0253.md) | Suppress `lru` use-after-free advisory — upstream-blocked via `tantivy ^0.16.3` pin; no fixed version in range; risk low (trigger requires panic inside tantivy LRU internals); re-evaluate on each `tantivy` release |
| [suppress-atomic-polyfill-rustsec-2023-0089](1.0.0/suppress-atomic-polyfill-rustsec-2023-0089.md) | Suppress `atomic-polyfill` unmaintained advisory — upstream-blocked via `postcard → heapless ^0.7.0` chain; crate not compiled into binary on any supported target; risk negligible; re-evaluate on each `postcard` release |

### Schema management

| Decision | Summary |
| -------- | ------- |
| [schema-overlay-model](1.0.0/schema-overlay-model.md) | Embedded defaults + on-disk overrides replace copy-on-create — `spaces::create` stops copying schemas; `space_builder` merges embedded + on-disk on every mount; `wiki migrate` backed by SHA manifest (`schemas/manifest.json`) cleans up stock copies from existing wikis without touching user customizations — implementation spec: [design-schema-overlay-migration](../improvements/design-schema-overlay-migration.md) |

### Windows compatibility

| Decision | Summary |
| -------- | ------- |
| [rebuild-close-handles-before-rename](1.0.0/rebuild-close-handles-before-rename.md) | Drop `tantivy_index`/`index_reader` from `inner` before live→backup rename — Windows denies rename on directories with open mmap handles (os error 5); reopen fresh after rename instead of `reload_reader()`; `close()` escape hatch for tests that corrupt mmap'd files (os error 1224) |
| [windows-compat-test-hygiene](1.0.0/windows-compat-test-hygiene.md) | Six cross-platform rules for tests: `#[cfg(unix)]` gating, `USERPROFILE` fallback, `Path::ends_with` for path suffix checks, canonicalize both sides for path equality, `encoding="utf-8"` on all subprocess/file I/O, no hardcoded `/tmp` in pytest config |

### Known gaps

| Decision | Summary |
| -------- | ------- |
| [redact-error-windows-paths](1.0.0/redact-error-windows-paths.md) | `redact_error` does not redact Windows drive-letter or UNC paths — deferred; no maintainer can test the fix end-to-end; information-leak concern only, not correctness; fix design documented for a Windows contributor |

## v0.6.0

### Bug fixes

| Decision | Summary |
| -------- | ------- |
| [graph-snapshot-stale-key](0.6.0/graph-snapshot-stale-key.md) | Fix stale graph snapshot: `key_fn` switched from `generation()` (resets to 0 on process start) to `last_commit()` (git HEAD SHA, stable across restarts). Supersedes keying section of [0.3.0/graph-cache](0.3.0/graph-cache.md) for `WithSnapshot` variant. |
| [reject-page-id](0.6.0/reject-page-id.md) | Permanent page identity via ULID was evaluated and rejected — the lint loop already covers reorganization |

## v0.5.8

### Bug fixes

| Decision | Summary |
| -------- | ------- |
| [mermaid-node-id-from-petgraph-index](0.5.8/mermaid-node-id-from-petgraph-index.md) | Use `N{idx.index()}` as mermaid node ID — eliminates sanitization logic and collision risk; `mermaid_id` deleted entirely; labels unchanged ([#128](https://github.com/geronimo-iia/llm-wiki/issues/128)) |
| [markdown-parser-for-link-extraction](0.5.8/markdown-parser-for-link-extraction.md) | Adopt `pulldown-cmark` for body link extraction — replaces manual walker; TOML `[[section]]` headers and inline code no longer extracted as wikilinks. Supersedes "manual walker, not a Markdown parser" in [0.2.0/commonmark-body-links](0.2.0/commonmark-body-links.md). |

## v0.5.7

### New tools

| Decision | Summary |
| -------- | ------- |
| [wiki-info-tool](0.5.7/wiki-info-tool.md) | Add `wiki_info` MCP tool for server identity and health |

### Bug fixes

| Decision | Summary |
| -------- | ------- |
| [commonmark-link-normalization](0.5.7/commonmark-link-normalization.md) | Normalize CommonMark relative link destinations (`./page.md`, `../dir/page.md`) before storing in `body_links`; `source_dir` threaded as `Option<&str>` through `index_page` → `extract_body_wikilinks` → `extract_commonmark_links`. Supersedes "No callers change" in [0.2.0/commonmark-body-links](0.2.0/commonmark-body-links.md). |

## v0.5.0

### Search & Export

| Decision | Summary |
| -------- | ------- |
| [config-search-status](0.5.0/config-search-status.md) | `search.status.<key>` is now a valid dot-notation path in `config set/get`, enabling CLI access to search ranking multipliers |
| [export-custom-frontmatter](0.5.0/export-custom-frontmatter.md) | JSON export now includes a `frontmatter` object per page with fields not already surfaced at the top level |

## v0.4.1

### Testing

| Decision | Summary |
| -------- | ------- |
| [pytest-integration-suite](0.4.1/pytest-integration-suite.md) | Replace bash integration scripts with pytest suite under `tests-integration/`; eliminates false-positive grep patterns, provides automatic teardown and structured JSON inspection |

## v0.4.0

### Graph

| Decision | Summary |
| -------- | ------- |
| [petgraph-live](0.4.0/petgraph-live.md) | Adopt `petgraph-live` — replace bespoke `CachedGraph` with `GenerationCache`; enables snapshot warm-start and algorithm suite |

## v0.3.0

### Transport & Protocol

| Decision | Summary |
| -------- | ------- |
| [acp-workflows](0.3.0/acp-workflows.md) | Six ACP workflows, cooperative cancellation, session cap, watcher push, `--http` flag requirement |

### Graph

| Decision | Summary |
| -------- | ------- |
| [graph-cache](0.3.0/graph-cache.md) | In-memory WikiGraph cache keyed on index generation; community map co-located; filtered requests bypass cache |

### Space Management

| Decision | Summary |
| -------- | ------- |
| [configurable-wiki-root](0.3.0/configurable-wiki-root.md) | `wiki_root` in `wiki.toml`; `spaces register` command; eliminates all hardcoded `.join("wiki")` |

## v0.2.0

### Skill / Engine Boundary

| Decision | Summary |
| -------- | ------- |
| [no-format-adapters-in-engine](0.2.0/no-format-adapters-in-engine.md) | Format normalization for external session stores stays outside the engine; crystallize skill handles extraction from raw files |

### Tools & Output Formats

| Decision | Summary |
| -------- | ------- |
| [llms-format-on-existing-tools](0.2.0/llms-format-on-existing-tools.md) | `format: "llms"` added to `wiki_list`/`wiki_search`/`wiki_graph`; `wiki_export` writes a file (default `llms.txt` at wiki root), response is a report not content |
| [local-path-content](0.2.0/local-path-content.md) | `wiki_resolve` tool + `path` in `wiki_content_new` response + `path` in `LintFinding`; `wiki_ingest` pages array dropped as redundant |

### Graph

| Decision | Summary |
| -------- | ------- |
| [cross-wiki-links](0.2.0/cross-wiki-links.md) | `wiki://` URIs resolved at graph build time (no schema change); `cross_wiki` flag opt-in; lint validates, ingest does not |

### Links & Indexing

| Decision | Summary |
| -------- | ------- |
| [commonmark-body-links](0.2.0/commonmark-body-links.md) | Both `[[slug]]` and `[text](slug)` supported as body links; manual walker not a Markdown parser; code-block false positives are a known shared limitation |

## v0.1.1

### Architecture (2026-04-18)

| Decision | Summary |
| -------- | ------- |
| [engine-vs-skills](0.1.1/engine-vs-skills.md) | Engine is a stateless tool provider; workflow intelligence lives in skills |
| [tool-surface](0.1.1/tool-surface.md) | 15 tools, stateful access criterion, CLI consistency |
| [wiki-as-skill-registry](0.1.1/wiki-as-skill-registry.md) | No separate skill protocol; the wiki is the registry |
| [schema-md-eliminated](0.1.1/schema-md-eliminated.md) | Type registry to wiki.toml, conventions to skills |
| [three-repositories](0.1.1/three-repositories.md) | Engine, skills, hugo-cms as independent repos |

### Type System & Index (2026-04-18)

| Decision | Summary |
| -------- | ------- |
| [json-schema-validation](0.1.1/json-schema-validation.md) | JSON Schema for per-type validation, x- extensions for engine behavior |
| [typed-graph-edges](0.1.1/typed-graph-edges.md) | x-graph-edges in JSON Schema for labeled directed edges |
| [dynamic-index-schema](0.1.1/dynamic-index-schema.md) | Tantivy schema computed from type registry, not hardcoded |
| [untyped-frontmatter](0.1.1/untyped-frontmatter.md) | BTreeMap instead of fixed struct; type registry validates |
| [rationalize-specs](0.1.1/rationalize-specs.md) | How the specifications were rationalized |

### Engine Internals (2026-04-18 to 2026-04-19)

| Decision | Summary |
| -------- | ------- |
| [engine-manager](0.1.1/engine-manager.md) | Centralized mutation handling with cascade reports |
| [ops-module](0.1.1/ops-module.md) | Extract duplicated CLI/MCP business logic into src/ops.rs |
| [schema-driven-types](0.1.1/schema-driven-types.md) | Types discovered from schemas via x-wiki-types; wiki.toml as overrides |

### Refactoring from Spec-Gap Analysis (2026-04-20)

| Decision | Summary |
| -------- | ------- |
| [engine-manager-redesign](0.1.1/engine-manager-redesign.md) | Rename Engine→EngineState/WikiEngine, extract mount_wiki, interior mutability in SpaceIndexManager |
| [graceful-shutdown](0.1.1/graceful-shutdown.md) | Coordinated shutdown via watch channel + AtomicBool |
| [list-pagination](0.1.1/list-pagination.md) | Native string fast field sort replaces _slug_ord u64 hack |
| [space-context](0.1.1/space-context.md) | Per-wiki SpaceContext bundles registry + index + paths |
| [unspec-code](0.1.1/unspec-code.md) | Logs CLI and wiki-link extraction spec'd; rest is impl detail |
| [wiki-page-struct](0.1.1/wiki-page-struct.md) | Not needed — 3 call sites, all local to index_manager.rs |
| [index-query-pattern](0.1.1/index-query-pattern.md) | Not worth it — 3 consumers with different return types |
| [rename-ops-ingest](0.1.1/rename-ops-ingest.md) | Left as-is — stutter is 1 internal line |
| [yaml-value-extraction](0.1.1/yaml-value-extraction.md) | Left as-is — intentionally different Sequence handling |

### Transport & Protocol (2026-04-21 to 2026-04-22)

| Decision | Summary |
| -------- | ------- |
| [acp-builder-pattern](0.1.1/acp-builder-pattern.md) | Agent builder replaces Agent trait; no LocalSet/channel/thread |
| [rmcp-streamable-http](0.1.1/rmcp-streamable-http.md) | rmcp 1.x, SSE → Streamable HTTP, config rename, ACP bridge deferred |

### Tools & Search (2026-04-22 to 2026-04-23)

| Decision | Summary |
| -------- | ------- |
| [search-facets](0.1.1/search-facets.md) | Always-on facets, hybrid filtering, top-N tags |
| [wiki-history](0.1.1/wiki-history.md) | Shell git log, follow config, NUL-delimited parsing |
| [no-embedding-search](0.1.1/no-embedding-search.md) | BM25-only for v0.1; no vector search dependency |
| [page-body-templates](0.1.1/page-body-templates.md) | Naming convention in schemas/, fallback chain, watcher ignores .md |
| [wiki-diff-not-a-tool](0.1.1/wiki-diff-not-a-tool.md) | git diff via bash, not a tool — design principle |
| [wiki-stats](0.1.1/wiki-stats.md) | Composed from existing primitives, fixed staleness buckets |
| [wiki-suggest](0.1.1/wiki-suggest.md) | Three strategies, edge field suggestion, suggest only |
| [wiki-watch](0.1.1/wiki-watch.md) | Notify crate, debounce, smart schema rebuild, CLI flag only |

## Backlog

| Decision | Summary |
| -------- | ------- |
| [config-crate](backlog/config-crate.md) | Reject `config` crate — current TOML loading sufficient; revisit if env-var overrides needed at scale |
| [replace-serde-yaml](backlog/replace-serde-yaml.md) | Migrate off abandoned `serde_yaml 0.9` — blocked: `saphyr-serde v0.0.0` is a stub, `serde_yaml2` serializer output format is broken; revisit when `saphyr-serde >= 0.1.0` ships |
| [engine-state-embedding-api](backlog/engine-state-embedding-api.md) | Hide `engine.state` behind `WikiEngine::with_state<F,T>` — decouples embedders from the lock type and `EngineState` shape; purely additive |
