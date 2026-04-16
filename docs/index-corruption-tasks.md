# Index Corruption Tasks

Implement corruption detection, auto-recovery, and schema versioning for
the tantivy search index.

Reference:
- [Index integrity spec](specifications/core/index-integrity.md)
- [Index corruption analysis](index-corruption.md)
- [Index commands spec](specifications/commands/index.md)

Current state: only staleness detection (git commit comparison). No
corruption detection, no schema versioning, no auto-recovery.

---

## Phase 1 — Resilience (recover from broken state)

### Task I1 — Resilient state.toml parsing

**Goal:** A malformed or missing `state.toml` should not error — it should
be treated as "needs rebuild."

#### Analysis

`index_status` calls `toml::from_str` on `state.toml`. If the file is
partially written (crash during rebuild) or manually corrupted, this
returns an error that propagates to the caller. The caller gets a raw
error instead of a recoverable "stale" status.

#### Code changes

- `src/search.rs` — in `index_status`, catch `toml::from_str` errors
  and return `stale: true` with `built: None, pages: 0, sections: 0`
  instead of propagating the error.

#### Tests

- `tests/search.rs` — new test: `index_status_returns_stale_on_malformed_state_toml`
  — write garbage to `state.toml`, assert `status.stale == true` and no error.

#### Exit criteria

- Malformed `state.toml` → `stale: true`, no error.
- Missing `state.toml` → `stale: true` (already works).
- `cargo test` passes.

---

### Task I2 — Try-open with auto-recovery

**Goal:** If the tantivy index is corrupt, auto-rebuild instead of
returning an opaque error.

#### Analysis

`search()` and `list()` call `Index::open(dir)`. If the mmap files are
truncated or corrupted, this fails with a tantivy error. Currently the
error propagates as-is.

The fix: wrap `Index::open` in a try-open → rebuild → retry pattern.
Gate behind a config flag so the user can disable it.

#### Config

Add `index.auto_recovery` to global config (default: `true`):

```toml
[index]
auto_rebuild = false    # existing: rebuild stale index before search/list
auto_recovery = true    # new: rebuild corrupt index on open failure
```

`auto_recovery` is separate from `auto_rebuild`:
- `auto_rebuild` gates rebuilding when the index is stale (commit mismatch)
- `auto_recovery` gates rebuilding when the index is corrupt (open failure)

Both are global-only (not per-wiki). `auto_recovery = false` means
corrupt index errors propagate to the caller.

#### Code changes

- `src/config.rs` — add `auto_recovery: bool` (default `true`) to
  `IndexConfig`. Add to `set_global_config_value`. Add to global-only
  rejection list in `set_wiki_config_value`.
- `src/main.rs` — add to `get_config_value`.
- `src/mcp/tools.rs` — add to `get_value`.
- `src/search.rs` — extract index opening into a helper:
  ```rust
  fn open_index_or_recover(
      search_dir: &Path,
      wiki_root: &Path,
      index_path: &Path,
      wiki_name: &str,
      repo_root: &Path,
      auto_recovery: bool,
  ) -> Result<Index> {
      let dir = MmapDirectory::open(search_dir)?;
      match Index::open(dir) {
          Ok(idx) => Ok(idx),
          Err(e) if auto_recovery => {
              tracing::warn!(
                  wiki = %wiki_name,
                  error = %e,
                  "index corrupt, rebuilding",
              );
              rebuild_index(wiki_root, index_path, wiki_name, repo_root)?;
              let dir = MmapDirectory::open(search_dir)?;
              Index::open(dir).context("index still corrupt after rebuild")
          }
          Err(e) => Err(e.into()),
      }
  }
  ```
- `src/search.rs` — update `search()` and `list()` to accept
  `auto_recovery: bool` and use the helper. Update all callers.

#### Tests

- `tests/search.rs` — new tests:
  - `search_recovers_from_corrupt_index` — corrupt the index files,
    search with `auto_recovery = true`, assert results returned.
  - `search_errors_on_corrupt_index_without_recovery` — corrupt the
    index, search with `auto_recovery = false`, assert error.

#### Exit criteria

- Corrupt index + `auto_recovery = true` → auto-rebuild + successful query.
- Corrupt index + `auto_recovery = false` → error propagated.
- `wiki config set index.auto_recovery false --global` works.
- `wiki config set index.auto_recovery false --wiki research` → error.
- `cargo test` passes.

---

### Task I3 — Server-side retry for MCP search/list

**Goal:** MCP tool handlers retry once on index error before failing.

#### Analysis

Currently `handle_search` and `handle_list` in `mcp/tools.rs` call
`search::search` / `search::list` directly. If the index is corrupt,
the error is returned to the MCP client.

With I2 implemented, the auto-recovery happens inside `search()` and
`list()` when `auto_recovery = true`. This task ensures the MCP handlers
pass the config flag through.

#### Code changes

- `src/mcp/tools.rs` — in `handle_search` and `handle_list`, pass
  `resolved.index.auto_recovery` to the search/list calls (after I2
  changes the signatures).

#### Tests

- Covered by I2 tests (the recovery logic is in search.rs).

#### Exit criteria

- MCP `wiki_search` auto-recovers from corrupt index.
- MCP `wiki_list` auto-recovers from corrupt index.
- `cargo test` passes.

---

## Phase 2 — Schema versioning

### Task I4 — Schema version in state.toml

**Goal:** Detect schema changes between versions and trigger rebuild.

#### Analysis

If `build_schema()` changes (e.g. adding a field), the existing index
becomes incompatible. Currently there's no detection — queries may
silently return wrong results or fail.

Schema version is a constant in the code (`CURRENT_SCHEMA_VERSION`),
bumped manually when `build_schema()` changes. Written to `state.toml`
on rebuild, checked in `index_status`.

Schema mismatch sets `stale: true` in `index_status`. This triggers:
- `auto_rebuild = true` → rebuild before search/list (staleness path)
- `auto_recovery = true` → rebuild on open failure (recovery path, if
  the schema change also makes `Index::open` fail)
- Both false → stale warning, may produce wrong results

No new fields on `IndexStatus`, no new caller logic. The existing
rebuild/recovery paths handle everything.

#### Code changes

- `src/search.rs` — add `const CURRENT_SCHEMA_VERSION: u32 = 1`.
- `src/search.rs` — add `schema_version: u32` to `IndexState` with
  `#[serde(default)]` (so pre-versioning state.toml files parse as 0).
- `src/search.rs` — in `rebuild_index`, write `CURRENT_SCHEMA_VERSION`
  to `IndexState`.
- `src/search.rs` — in `index_status`, after parsing `state.toml`,
  treat `state.schema_version != CURRENT_SCHEMA_VERSION` as `stale: true`.

#### Tests

- `tests/search.rs` — new test:
  `index_status_returns_stale_on_schema_version_mismatch` — build index,
  manually edit `state.toml` to set `schema_version = 0`, assert
  `stale: true`.

#### Exit criteria

- Schema version mismatch → `stale: true`.
- `wiki index status` shows stale after schema change.
- Pre-versioning `state.toml` (no `schema_version` field) → version 0
  → mismatch → stale.
- `cargo test` passes.

---

## Phase 3 — Diagnostics

### Task I5 — `wiki index check` subcommand

**Goal:** Dedicated integrity check without modifying anything.

#### Code changes

- `src/cli.rs` — add `IndexAction::Check` variant.
- `src/search.rs` — add `index_check()` that:
  1. Checks `state.toml` exists and parses
  2. Checks schema version matches
  3. Tries `Index::open`
  4. Runs a test `AllQuery` with limit 1
  5. Returns a structured `IndexCheckReport`
- `src/main.rs` — wire up `Commands::Index { action: Check }`.
- `src/mcp/tools.rs` — add `wiki_index_check` tool.
- `src/mcp/tools.rs` — add tool to `tool_list()`.

#### Report

```rust
pub struct IndexCheckReport {
    pub wiki: String,
    pub openable: bool,
    pub queryable: bool,
    pub schema_version: Option<u32>,
    pub schema_current: bool,
    pub state_valid: bool,
    pub stale: bool,
}
```

#### Tests

- `tests/search.rs` — new tests:
  - `index_check_reports_healthy_index` — build index, check, assert all ok.
  - `index_check_reports_corrupt_index` — corrupt files, check, assert
    `openable: false`.

#### Exit criteria

- `wiki index check` prints structured health report.
- MCP `wiki_index_check` returns the same report as JSON.
- `cargo test` passes.

---

## Execution order

| Order | Task | Phase | Effort | Dependencies |
|-------|------|-------|--------|-------------|
| 1 | I1 — Resilient state.toml | Resilience | Tiny | None |
| 2 | I2 — Try-open auto-recovery | Resilience | Medium | I1 |
| 3 | I3 — MCP retry | Resilience | Small | I2 |
| 4 | I4 — Schema version | Versioning | Small | None |
| 5 | I5 — index check command | Diagnostics | Medium | I4 |

Phase 1 (I1-I3) is the priority — it prevents the most common failure
mode (corrupt index blocking all search/list operations).
