# Accept yanked `chacha20 0.10.1` — transitive via rmcp

## Decision

Accept the `cargo audit` yanked warning for `chacha20 0.10.1` as a known, low-risk
situation. No `audit.toml` change is possible (cargo-audit offers no per-crate
yanked suppression). Re-evaluate after each `rmcp` release.

## Context

`cargo audit` reports `chacha20 0.10.1` as yanked. The dependency chain is:

```
llm-wiki-engine → rmcp 3.1.4 → rand 0.10.2 → chacha20 0.10.1
```

`chacha20 0.10.1` carries no RUSTSEC advisory ID — it was yanked from crates.io
without a published security advisory. Yanks without an advisory typically indicate
a packaging mistake, a regression, or an API issue in the release, not a confirmed
security vulnerability.

`rmcp 3.1.4` is the latest published release of the MCP SDK (verified 2026-08-28).
Running `cargo update rmcp` resolves to `rmcp 3.1.4` with no change — no newer
version exists that pulls a different `rand` or `chacha20`.

`cargo audit` exits 0; the warning is already in the "allowed warnings" output.
There is no `[[advisories]] ignore` entry to add because there is no RUSTSEC ID.

## Risk assessment

- No confirmed security vulnerability. No CVE. No RUSTSEC.
- `chacha20` is used by `rand` for random number generation inside `rmcp` internals.
  `llm-wiki-engine` does not call `rand` or `chacha20` directly.
- The yanked version continues to function; yanking does not make it uninstallable
  or dangerous, only undiscoverable via `cargo add` for new projects.

Risk level: **low**.

## Consequences

- No `audit.toml` change. `cargo audit` continues to report this as an allowed warning.
- **Tracking:** after each `rmcp` release, run `cargo update && cargo audit` to check
  whether a newer version resolves the `chacha20` dependency. Remove this decision when
  the warning disappears.
