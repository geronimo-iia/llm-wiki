# Decisions

Architectural decisions and their rationale, grouped by release.

---

## v0.3.0 — 2026-05-01

Decisions made during ACP workflow expansion and transport stabilization.

### Transport & Protocol

| Decision | Summary |
| -------- | ------- |
| [acp-workflows](0.3.0/acp-workflows.md) | Six ACP workflows, cooperative cancellation, session cap, watcher push, `--http` flag requirement |

### Graph

| Decision | Summary |
| -------- | ------- |
| [graph-cache](0.3.0/graph-cache.md) | In-memory WikiGraph cache keyed on index generation; community map co-located; filtered requests bypass cache |

---

## v0.2.0 — 2026-04-27

Decisions made during improvement design phase.

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

---

## v0.1.1 — 2026-04-18 to 2026-04-23

All decisions made during the initial development cycle leading to the first public release.

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
