# Partial `pub(crate)` migration — Phase 5

## Decision

Convert four modules to `pub(crate)` in `src/lib.rs`: `cli`, `server`,
`watch`, `pathutil`. Leave the remaining 18 modules as `pub mod`. Add
`#![warn(unreachable_pub)]` with `#[allow(unreachable_pub)]` at the top of
each `pub mod` module to suppress noise from items that are `pub` only because
integration tests import them directly.

Add a curated `pub use` façade at the crate root re-exporting the five stable
surface types: `WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`,
`WikiGraph`.

## Context

`src/lib.rs` at v0.5.9 declared all 22 modules as `pub mod`. Implementation
internals (`acp`, `space_builder`, `mcp`, `cli`, `watch`, `server`,
`index_schema`) were part of the public crate API. Any internal refactor was
technically a semver violation.

The 1.0.0 goal is a stable public API surface. The ideal outcome is all
internal modules `pub(crate)` with a thin `pub use` façade. The constraint
blocking a full migration is the integration test layer: `tests/*.rs` files
import internal modules directly via `use llm_wiki::graph::*`,
`use llm_wiki::spaces`, `use llm_wiki::index_manager::SpaceIndexManager`, etc.
Making those modules `pub(crate)` would break the test suite.

## Why only four modules

The four converted modules (`cli`, `server`, `watch`, `pathutil`) have zero
imports in `tests/*.rs`. Confirmed by grep at Phase 5 execution time. All
other modules have at least one test import and cannot be converted without
refactoring the test layer.

The 18 modules that remain `pub mod` are:
`acp`, `config`, `default_schemas`, `engine`, `frontmatter`, `git`, `graph`,
`index_manager`, `index_schema`, `ingest`, `links`, `markdown`, `mcp`, `ops`,
`search`, `slug`, `space_builder`, `spaces`, `type_registry`.

## Why `#[allow(unreachable_pub)]` per module rather than removing the lint

`#![warn(unreachable_pub)]` crate-wide would fire on every `pub` item in the
18 remaining `pub mod` modules — hundreds of warnings, all legitimate (they
are `pub` because tests need them). This would make the lint misleading rather
than useful. The per-module `#[allow]` suppresses noise on modules that cannot
be converted yet, while keeping the lint active on the four converted modules
where it catches real issues.

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

- `cli`, `server`, `watch`, `pathutil` are `pub(crate)`. Their `pub fn` items
  are downgraded to `pub(crate)`. External consumers cannot import them.
- The five stable types are re-exported at the crate root:
  `WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`, `WikiGraph`.
- 18 modules remain `pub mod` with `#[allow(unreachable_pub)]`. This is a
  known limitation, not an oversight.
- **Tracking:** when the test layer is refactored (Post-1.0), remove the
  `#[allow(unreachable_pub)]` suppressions one module at a time and convert
  each to `pub(crate)`. The lint will guide the remaining work.
