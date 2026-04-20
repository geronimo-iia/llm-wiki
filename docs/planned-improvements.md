---
title: "Planned Improvements"
summary: "Known engineering improvements — not bugs, not features, just better code."
status: ready
last_updated: "2025-07-18"
---

# Planned Improvements

Engineering improvements that don't change behavior but improve
performance, maintainability, or correctness. Not tracked in the
roadmap (those are features).

## Index lifetime in MCP server

**Problem:** Every tool call that touches the tantivy index
(`search`, `list`, `delete_by_type`) opens it from disk:

```rust
let dir = MmapDirectory::open(&search_dir)?;
let index = Index::open(dir)?;
let reader = index.reader()?;
```

For the CLI this is fine (one command, one open). For the MCP server
(long-running, many calls), this is wasteful — `Index::open()` reads
segment metadata from disk on every request.

**Fix:** Store `tantivy::Index` in `SpaceState`, opened once at
startup. `IndexReader` auto-reloads after commits — no manual refresh
needed. Search/list get a `Searcher` from the reader (cheap). Write
operations get a writer from the index.

**Impact:** Milliseconds per call for small wikis. Noticeable for
10k+ page wikis with frequent queries.

**Blocked by:** Nothing. Pure refactor.

## Partial index rebuild

**Problem:** Any `schema_hash` mismatch triggers a full rebuild. If
only one type's schema changed, we re-index all pages.

**Fix:** Compare per-type hashes (already stored in `state.toml`).
If only some types changed, re-index only pages of those types.
`indexing::rebuild_types(types: &[String])` deletes and re-indexes
pages matching the changed types.

**Impact:** Seconds saved on large wikis when a single schema changes.

**Blocked by:** Nothing. Per-type hashes already computed and stored.

## Schema file content in hash

**Problem:** `schema_hash` is computed from `schema_path` + `aliases`
per type. It doesn't hash the actual schema file content. If a schema
file is modified without changing aliases, the hash doesn't change
and the index isn't rebuilt.

**Fix:** Include a content hash of each schema file in the per-type
hash computation. Or hash the full file content into `schema_hash`.

**Impact:** Correctness — currently a schema change that adds a new
property (without changing aliases) won't trigger a rebuild until
the engine is restarted.

**Blocked by:** Nothing.

## IndexSchema build deduplication with space_builder

**Problem:** `build_space()` reads schema files once and builds both
`SpaceTypeRegistry` and `IndexSchema`. But `IndexSchema::build_from_schemas()`
still exists as a separate code path (used by tests). Two ways to
build the same thing.

**Fix:** Remove `IndexSchema::build_from_schemas()` — all callers
use `build_space()`. Keep `IndexSchema::build()` (hardcoded) for
backward-compat tests only.

**Impact:** Less code to maintain, single path for schema building.

**Blocked by:** Nothing.

## ops module test coverage

**Problem:** `tests/ops.rs` has 21 tests covering the ops API, but
the new schema operations (`schema_list`, `schema_show`, etc.) are
only tested in `tests/schema_integration.rs`. The ops test file
doesn't cover them.

**Fix:** Either add schema ops tests to `tests/ops.rs` or accept
that `tests/schema_integration.rs` is the coverage for those ops.

**Impact:** Test organization clarity.

**Blocked by:** Nothing.

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
  - ~~Chocolatey~~ — too heavy to maintain, drop
- [ ] Document only the supported channels in README and release.md
- [ ] Verify `cargo-binstall` works with current `pkg-url` config

**Impact:** Users can't install easily without `cargo install` today.

**Blocked by:** First stable release (need binaries to distribute).

## User-facing documentation

**Problem:** The README has quick-start snippets but no detailed
guides for installation, platform-specific issues, or integration
beyond MCP config.

**Fix:**

- [ ] Installation guide (cargo install, pre-built binaries, platform
  notes, prerequisites)
- [ ] Windows installation and usage notes (path separators, git
  config, shell differences)
- [ ] IDE integration guides (VS Code, Cursor, Windsurf — beyond the
  MCP config snippets in README, covering workflow examples)
- [ ] CI/CD integration (using llm-wiki in automated pipelines —
  ingest on PR merge, index rebuild in CI, schema validation as
  a pre-commit check)

**Impact:** Adoption barrier — users who aren't Rust developers or
MCP experts can't get started easily.

**Blocked by:** Distribution channels (need installable binaries
before writing installation guides for non-Rust users).

## Implementation documentation gaps

**Problem:** Some implementation areas lack dedicated docs. New
contributors need to read source code to understand the config
system, server lifecycle, and logging.

**Fix:**

- [ ] Architecture overview (module map, data flow diagram, key
  abstractions and their relationships)
- [ ] Config system (two-level resolution, how to add a new config
  key, serde patterns, global-only vs per-wiki keys)
- [ ] Server internals (MCP stdio/SSE transport lifecycle, ACP
  agent session management, shutdown/restart behavior)
- [ ] Logging (rotation config, format options, file vs stderr,
  serve mode vs CLI mode, tracing spans)

**Impact:** Onboarding time for contributors. Currently need to
read `config.rs`, `server.rs`, `mcp/mod.rs` to understand these.

**Blocked by:** Nothing — can be written anytime from existing code.
