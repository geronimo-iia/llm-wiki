# Decisions

Architectural decisions and their rationale.

## Design Decisions

| Decision                                            | Summary                                                                    |
| --------------------------------------------------- | -------------------------------------------------------------------------- |
| [engine-vs-skills](engine-vs-skills.md)             | Engine is a stateless tool provider, workflow intelligence lives in skills |
| [tool-surface](tool-surface.md)                     | 15 tools, stateful access criterion, CLI consistency                       |
| [wiki-as-skill-registry](wiki-as-skill-registry.md) | No separate skill protocol, the wiki is the registry                       |
| [schema-md-eliminated](schema-md-eliminated.md)     | Type registry to wiki.toml, conventions to skills                          |
| [three-repositories](three-repositories.md)         | Engine, skills, hugo-cms as independent repos                              |
| [json-schema-validation](json-schema-validation.md) | JSON Schema for per-type validation, x- extensions for engine behavior     |
| [typed-graph-edges](typed-graph-edges.md)           | x-graph-edges in JSON Schema for labeled directed edges                    |
| [dynamic-index-schema](dynamic-index-schema.md)     | Tantivy schema computed from type registry, not hardcoded                  |
| [untyped-frontmatter](untyped-frontmatter.md)       | BTreeMap instead of fixed struct, type registry validates                  |
| [engine-manager](engine-manager.md)                 | Centralized mutation handling with cascade reports                         |
| [ops-module](ops-module.md)                         | Extract duplicated CLI/MCP business logic into src/ops.rs                  |
| [rationalize-specs](rationalize-specs.md)           | How the specifications were rationalized                                   |
| [schema-driven-types](schema-driven-types.md)       | Types discovered from schemas via x-wiki-types, wiki.toml as overrides    |
| [space-context](space-context.md)                   | Per-wiki SpaceContext bundles registry + index + paths                     |

## Refactoring Decisions (from spec-gap analysis)

| Decision                                                  | Summary                                                              |
| --------------------------------------------------------- | -------------------------------------------------------------------- |
| [engine-manager-redesign](engine-manager-redesign.md)     | Rename Engine→EngineState/WikiEngine, extract mount_wiki, interior mutability in SpaceIndexManager |
| [graceful-shutdown](graceful-shutdown.md)                  | Coordinated shutdown via watch channel + AtomicBool                   |
| [list-pagination](list-pagination.md)                      | Native string fast field sort replaces _slug_ord u64 hack            |
| [unspec-code](unspec-code.md)                              | Logs CLI and wiki-link extraction spec'd, rest is impl detail        |
| [wiki-page-struct](wiki-page-struct.md)                    | Not needed — 3 call sites, all local to index_manager.rs             |
| [index-query-pattern](index-query-pattern.md)              | Not worth it — 3 consumers with different return types               |
| [rename-ops-ingest](rename-ops-ingest.md)                  | Left as-is — stutter is 1 internal line                              |
| [yaml-value-extraction](yaml-value-extraction.md)          | Left as-is — intentionally different Sequence handling               |
| [acp-builder-pattern](acp-builder-pattern.md)              | Agent builder replaces Agent trait, no LocalSet/channel/thread       |
