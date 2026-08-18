# Partial `pub(crate)` migration — Phase 5

## Decision

Convert **one** module to `pub(crate)` in `src/lib.rs`: `pathutil`. Leave all
other 21 modules as `pub mod`. Add `#![warn(unreachable_pub)]` crate-wide to
catch future leaks in the converted scope.

Add a curated `pub use` façade at the crate root re-exporting the five stable
surface types: `WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`,
`WikiGraph`.

## Context

`src/lib.rs` at v0.5.9 declared all 22 modules as `pub mod`. Implementation
internals (`acp`, `space_builder`, `mcp`, `cli`, `watch`, `server`,
`index_schema`) were part of the public crate API. Any internal refactor was
technically a semver violation.

The 1.0.0 goal is a stable public API surface. The ideal outcome is all
internal modules `pub(crate)` with a thin `pub use` façade. Two separate
constraints block a full migration.

**Constraint 1 — integration test layer.** `tests/*.rs` files import internal
modules directly (`use llm_wiki::graph::*`, `use llm_wiki::spaces`,
`use llm_wiki::index_manager::SpaceIndexManager`, etc.). Integration tests
compile as separate crates; they can only access `pub` items. Making those
modules `pub(crate)` would break the test suite without a significant test
refactor.

**Constraint 2 — `[[bin]]` crate boundary.** `src/main.rs` is compiled as a
separate crate (the `llm-wiki` binary). Even though it shares a `Cargo.toml`
with the library, it accesses the library as an external consumer. `pub(crate)`
visibility in the lib is **not** visible to `src/main.rs`. Modules used by the
binary (`cli`, `server`, `watch`) must remain `pub mod` even though no
integration test imports them.

## Why only one module

`pathutil` is used only within `src/` (never imported in `tests/*.rs` or
`src/main.rs`). It is the only module where the `pub(crate)` conversion was
both correct and feasible.

`cli`, `server`, `watch` have zero imports in `tests/*.rs` but are imported by
`src/main.rs` via `use llm_wiki::cli::...` — which crosses the `[[bin]]` crate
boundary. Converting them to `pub(crate)` breaks the binary.

The 21 modules that remain `pub mod` are:
`acp`, `cli`, `config`, `default_schemas`, `engine`, `frontmatter`, `git`,
`graph`, `index_manager`, `index_schema`, `ingest`, `links`, `markdown`, `mcp`,
`ops`, `search`, `server`, `slug`, `space_builder`, `spaces`, `type_registry`,
`watch`.

## Why no per-module `#[allow(unreachable_pub)]`

`#![warn(unreachable_pub)]` fires on `pub` items inside `pub(crate)` modules.
With only `pathutil` converted, and all other modules remaining `pub mod`, the
lint produces zero false positives — items inside `pub mod` modules are
reachable by definition. No per-module suppression is needed today. If future
modules are converted to `pub(crate)`, items inside them that can't be demoted
(because tests import them) would require per-module `#[allow(unreachable_pub)]`
suppressions at that time.

## Alternatives considered

**Refactor `tests/*.rs` to use only the stable façade.** The correct long-term
solution. Deferred to Post-1.0 — the test layer imports internal types
(`SpaceIndexManager`, `SpaceTypeRegistry`, `IndexSchema`, `WikiGraphCache`)
that are not part of the stable surface and would require significant test
restructuring or additional façade exports.

**Move integration tests into `src/` as `#[cfg(test)]` modules.** Would
eliminate the external import problem. Rejected for 1.0.0 — the test files
are large and the migration would be a significant refactor with no functional
change.

**Export all currently-tested types through the façade.** Would make the
stable surface too wide — `SpaceIndexManager`, `SpaceTypeRegistry`, and
`IndexSchema` are implementation details that should not be part of the public
API contract.

**Skip `#![warn(unreachable_pub)]` entirely.** Rejected — the lint is valuable
for the four converted modules and for future conversions. Suppressing it
entirely loses the signal.

## Consequences

- `pathutil` is `pub(crate)`. External consumers cannot import it.
- The five stable types are re-exported at the crate root:
  `WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`, `WikiGraph`.
- 21 modules remain `pub mod`. This is a known limitation with two root causes
  (test layer + `[[bin]]` boundary), not an oversight.
- **Tracking (Post-1.0):** two independent workstreams could expand the
  migration:
  1. Refactor `tests/*.rs` to use only the stable façade → enables
     converting test-blocking modules to `pub(crate)`.
  2. Move `cli`/`server`/`watch` dispatch code into `src/main.rs` directly,
     removing the binary's dependency on those lib modules → enables
     converting them to `pub(crate)`.
  The lint will guide each step.
