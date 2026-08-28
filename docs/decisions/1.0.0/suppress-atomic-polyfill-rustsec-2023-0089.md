# Suppress RUSTSEC-2023-0089 (`atomic-polyfill` unmaintained)

## Decision

Suppress advisory RUSTSEC-2023-0089 in `audit.toml` as an allowed warning.
Do not attempt to remove `atomic-polyfill` from the dependency graph directly.
Re-evaluate after each `postcard` release.

## Context

`atomic-polyfill 1.0.3` is flagged as unmaintained (RUSTSEC-2023-0089).
The crate has no known vulnerabilities — the advisory is a maintenance signal,
not a soundness or security issue.

The transitive dependency chain is:

```
llm-wiki-engine
  └── petgraph-live
        └── postcard ^1
              └── heapless ^0.7.0
                    └── atomic-polyfill 1.0.3
```

`heapless 0.9` dropped `atomic-polyfill`, but `postcard` pins `heapless ^0.7.0`.
Until `postcard` relaxes its constraint to `heapless ^0.8` or later, there is
no upgrade path available without forking `postcard`.

`cargo tree -i atomic-polyfill` returns nothing on the default build
target — the crate is present in `Cargo.lock` but is not reachable in the
compiled binary on standard targets (x86_64, aarch64). It exists only as a
dependency for embedded/no-std targets that `postcard` supports but
`llm-wiki-engine` does not use.

Two alternatives were considered and rejected:

**Fork or patch `postcard`.** Out of scope. `postcard` is a well-maintained
crate and the constraint will be relaxed in a future release.

**Replace `petgraph-live`.** `petgraph-live` is a first-party dependency
(authored by the same maintainer). Replacing it to avoid a transitive
unmaintained crate that is not even compiled into the binary would be
disproportionate.

Risk level: **negligible**. The crate is not compiled into the binary on any
supported target. "Unmaintained" means no future security fixes will be
published for `atomic-polyfill` itself — but since it is not present in the
runtime binary, there is no attack surface.

## Consequences

- `audit.toml` contains an `[[advisories]]` entry for RUSTSEC-2023-0089 with
  `ignore = true` and an explanatory comment referencing this decision.
- `cargo audit` exits 0 with this advisory listed as an allowed warning.
- **Tracking:** after each `postcard` release, run
  `cargo update && cargo tree -i atomic-polyfill` to check whether the chain
  has been broken. Remove the suppression entry from `audit.toml` as soon as
  `atomic-polyfill` no longer appears in `Cargo.lock`.
