---
title: "Planned Improvements"
summary: "Known engineering improvements — not bugs, not features, just better code."
status: ready
last_updated: "2025-07-20"
---

# Planned Improvements

Engineering improvements that don't change behavior but improve
performance, maintainability, or correctness. Not tracked in the
roadmap (those are features).

## Distribution channels

**Problem:** `docs/release.md` is a verbatim copy of agentctl's
release process — still says "agentctl" in places. Distribution
channels haven't been decided for llm-wiki.

**Fix:**

- [ ] Fix `docs/release.md` — replace agentctl references with
  llm-wiki equivalents
- [ ] Decide final channel list. Candidates:
  - `cargo install llm-wiki` — always supported (source build)
  - `cargo-binstall` — pre-built binaries via GitHub releases
    (already configured in `Cargo.toml` `[package.metadata.binstall]`)
  - Homebrew tap — macOS/Linux, low maintenance with a formula repo
  - asdf plugin — version manager integration
- [ ] Document only the supported channels in README and release.md
- [ ] Verify `cargo-binstall` works with current `pkg-url` config

**Blocked by:** First stable release (need binaries to distribute).

## User-facing documentation

**Problem:** The README has quick-start snippets but no detailed
guides for installation, platform-specific issues, or integration
beyond MCP config.

**Fix:**

- [ ] Installation guide (cargo install, pre-built binaries, platform
  notes, prerequisites)
- [ ] IDE integration guides (VS Code, Cursor, Windsurf — beyond the
  MCP config snippets, covering workflow examples)
- [ ] CI/CD integration (using llm-wiki in automated pipelines —
  ingest on PR merge, index rebuild in CI, schema validation as
  a pre-commit check)

**Blocked by:** Distribution channels (need installable binaries
before writing installation guides for non-Rust users).

## ~~Implementation documentation refresh~~ ✓

Done. All `docs/implementation/` files updated to reference
`WikiEngine`, `EngineState`, `refresh_index`, `index_page`.

## ~~`in_degree` unused public API~~ ✓

Removed from `graph.rs` and tests.
