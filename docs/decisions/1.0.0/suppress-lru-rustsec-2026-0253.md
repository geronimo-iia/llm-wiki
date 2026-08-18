# Suppress RUSTSEC-2026-0253 (`lru` use-after-free)

## Decision

Suppress advisory RUSTSEC-2026-0253 in `audit.toml` as an allowed warning.
Do not attempt to patch or replace `lru` directly. Re-evaluate after each
`tantivy` release.

## Context

`lru 0.16.4` has a confirmed unsoundness: potential use-after-free due to lack
of panic safety in `LruCache::pop()` (RUSTSEC-2026-0253).

`lru` is not a direct dependency of `llm-wiki-engine`. It is pulled in
transitively by `tantivy 0.26.1`, which pins `lru = "^0.16.3"`. At the time
of Phase 1 work (2026-08-17), `tantivy 0.26.1` is the latest release and no
version of `tantivy` in the 0.26.x series pins `lru ^0.17` or later.

`cargo update` was run and resolved all other advisories. This one survived
because no compatible version of `lru` in the `^0.16.3` range has a fix
published.

Two alternatives were considered and rejected:

**Replace `lru` with a different crate.** `lru` is used internally by
`tantivy`, not by `llm-wiki-engine` code. Replacing it would require forking
or patching `tantivy`, which is out of scope for a dependency hygiene task.

**Pin a `[patch]` override in `Cargo.toml`.** A `[patch]` to `lru 0.17`
would break `tantivy`'s API expectations since `tantivy` calls `lru` directly.
This would require patching `tantivy` source as well — equivalent to forking.

## Risk Assessment

The unsoundness requires a panic to occur *inside* `LruCache::pop()` to
trigger the use-after-free. `llm-wiki-engine` does not call `LruCache::pop()`
directly. The call originates inside `tantivy` internals during index
operations. A panic inside `tantivy`'s LRU cache during normal search or
indexing is not a realistic scenario under expected workloads.

Risk level: **low**. The unsoundness is real but the trigger condition is
unlikely in practice and not reachable from user-controlled input.

## Consequences

- `audit.toml` contains an `[[advisories]]` entry for RUSTSEC-2026-0253 with
  `ignore = true` and an explanatory comment referencing this decision.
- `cargo audit` exits 0 with this advisory listed as an allowed warning.
- **Tracking:** after each `tantivy` release, run `cargo update && cargo audit`
  to check whether the advisory has been resolved upstream. Remove the
  suppression entry from `audit.toml` as soon as it is no longer needed.
