# `llm-wiki-engine` as both binary and library crate

## Decision

Keep the `[lib]` target in `Cargo.toml`. Publish `llm-wiki-engine` to
crates.io as both a binary (`llm-wiki`) and a library (`llm_wiki_engine`). Define
a stable public surface via a `pub use` façade at the crate root. Leave
internal modules accessible to the test layer until a Post-1.0 test refactor
completes the `pub(crate)` migration.

## Context

`llm-wiki-engine` started as a CLI tool. The `[lib]` target was added early
so that `tests/*.rs` integration tests could import internal modules directly
(`use llm_wiki::graph::*`, `use llm_wiki::index_manager::SpaceIndexManager`,
etc.). This was pragmatic but unintentional as a library design — all 22
modules were `pub mod` by default, with no stable surface boundary.

At 1.0.0 the question was raised explicitly: should this be a binary-only
crate with no `lib.rs`?

## Why keep the library target

**Embedding value.** `WikiEngine` is non-trivial: Tantivy full-text search,
petgraph community detection, git integration, MCP and ACP transports. A
consumer building a custom server, a different CLI, or an automated pipeline
on top of the engine should not need to fork the repository. The library
target makes this possible without any additional work.

**Test architecture.** `tests/*.rs` importing internal types
(`SpaceIndexManager`, `SpaceTypeRegistry`, `IndexSchema`, `WikiGraphCache`)
is a strength, not a smell. These are the right units to test directly — going
through the CLI subprocess for every index or graph test would be slower,
noisier, and harder to isolate. The library target enables this cleanly.

**docs.rs.** `documentation = "https://docs.rs/llm-wiki-engine"` is already
established. The stable façade (`WikiEngine`, `GlobalConfig`, `SearchResult`,
`IngestReport`, `WikiGraph`) gives external consumers a documented entry point.

## What the stable surface is

Five types re-exported at the crate root:

| Type | Module | Role |
|------|--------|------|
| `WikiEngine` | `engine` | Central engine — mount, search, rebuild |
| `GlobalConfig` | `config` | Global configuration loaded from disk |
| `SearchResult` | `search` | Paginated search response |
| `IngestReport` | `ingest` | Result of an ingest operation |
| `WikiGraph` | `graph` | Directed concept graph |

Everything else is implementation detail. Internal modules are `pub(crate)`
where the test layer permits, `pub mod` with `#[allow(unreachable_pub)]`
where it does not.

## What is explicitly not stable

All internal modules not re-exported through the façade:
`acp`, `config` (internal types), `default_schemas`, `frontmatter`, `git`,
`graph` (internal functions), `index_manager`, `index_schema`, `ingest`
(internal types), `links`, `markdown`, `mcp`, `ops`, `search` (internal
types), `slug`, `space_builder`, `spaces`, `type_registry`.

These are accessible today because `tests/*.rs` needs them. They are not
part of the stability guarantee. A semver-minor release may restrict them
further as the test layer is refactored.

## Consequences

- `Cargo.toml` retains `[lib]` and `[[bin]]` targets.
- `pub use` façade at crate root re-exports the five stable types.
- `#![warn(unreachable_pub)]` added crate-wide; suppressed per-module where
  tests block conversion (see
  `docs/decisions/1.0.0/pub-crate-partial-migration.md`).

## Amendment — lib target renamed to `llm_wiki_engine`

The original decision named the lib target `llm_wiki`. This was corrected
before 1.0.0: the `[lib] name` in `Cargo.toml` is now `llm_wiki_engine`,
matching the package name `llm-wiki-engine`.

The mismatch meant embedders writing `use llm_wiki_engine::…` after
`cargo add llm-wiki-engine` would get a compile error; they had to know the
internal target name `llm_wiki` instead. The rename eliminates that
discoverability gap with no API or behaviour change. All 231 internal `llm_wiki::`
references were updated mechanically; the tracing default filter was updated from
`llm_wiki=info,warn` to `llm_wiki_engine=info,warn`.

