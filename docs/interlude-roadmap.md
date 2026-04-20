---
title: "Interlude — Pre-Phase 3 Improvements"
summary: "Engineering improvements to tackle between Phase 2 and Phase 3."
status: in-progress
last_updated: "2025-07-19"
---

# Interlude — Pre-Phase 3 Improvements

Improvements to address before starting Phase 3 (Typed Graph). Ordered
by priority — correctness first, then simplification, then quality.

## 1. Schema file content in hash ✅

Done. `compute_hashes` now includes SHA-256 of schema file content.
`compute_disk_hashes(repo_root)` reads files from disk for staleness
checks without building a full registry. `DefaultHasher` replaced by
SHA-256 throughout.

## 2. Remove hardcoded `IndexSchema::build()` ✅

Done. Removed the hardcoded constructor. Tests migrated to
`build_space_from_embedded`. Fixed non-deterministic field ordering
in `parse_from_embedded` (HashMap iteration → sorted).

## 3. Introduce `SpaceIndexManager` ✅

Done. Free functions in `src/indexing.rs` converted to methods on
`SpaceIndexManager` struct (`rebuild`, `update`, `status`, `last_commit`,
`delete_by_type`). Renamed `SpaceState` → `SpaceContext` — the per-wiki
aggregate that holds paths, registry, schema, and `index_manager`.

See `docs/decisions/space-context.md` for the design rationale.

## 4. Index lifetime in MCP server ✅

Done. `SpaceIndexManager` holds `Index` + `IndexReader` in memory.
`EngineManager::build()` calls `open()` at startup. `searcher()` returns
a cheap arc-clone of the current segment set.

`search.rs` and `graph.rs` are now pure functions over `&Searcher` —
no I/O, no recovery logic, no index-opening. `ops/` gets the searcher
from `space.index_manager.searcher()` and passes it through.

Also completed the §3 cleanup: deleted `src/indexing.rs`,
`tests/indexing.rs`, all deprecated wrappers, and `RecoveryContext`.
Recovery is handled internally by `SpaceIndexManager::open()`.

## 5. Partial index rebuild

**Priority:** Optimization — not blocking.

**Problem:** Any `schema_hash` mismatch triggers a full rebuild. If
only one type's schema changed (e.g. added a field to `concept.json`),
all pages are re-indexed unnecessarily.

**Mechanism:** `state.toml` already stores per-type hashes in `[types]`.
`SpaceTypeRegistry::type_hashes()` returns the current per-type hashes.
Comparing them reveals exactly which types changed.

**Task list:**

1. Add `pub fn changed_types(&self, repo_root: &Path) -> Result<Vec<String>>`
   to `SpaceIndexManager`. Reads `state.toml.types`, compares against
   `compute_disk_hashes(repo_root)`. Returns type names whose hash
   differs (added or modified). Removed types also returned.

2. Add `pub fn rebuild_types(&self, types: &[String], wiki_root: &Path,
   repo_root: &Path, is: &IndexSchema, registry: &SpaceTypeRegistry)
   -> Result<IndexReport>`. For each type in the list:
   - `delete_by_type(is, type_name)` to remove old documents
   - Walk `wiki/`, parse each `.md`, if `page.page_type()` matches
     one of the changed types → `add_document()`
   - `writer.commit()`
   - Update `state.toml` (new schema_hash, new type_hashes, new commit)

3. Add `pub fn staleness_kind(&self, repo_root: &Path) -> Result<StalenessKind>`
   where `StalenessKind` is:
   ```rust
   pub enum StalenessKind {
       Current,
       CommitChanged,
       TypesChanged(Vec<String>),
       FullRebuildNeeded,  // schema_hash changed + types added/removed
   }
   ```
   Logic:
   - If commit differs → `CommitChanged` (handled by `update()`)
   - If only some type hashes differ → `TypesChanged(changed)`
   - If global schema_hash differs but all individual types match
     → shouldn't happen, but fallback to `FullRebuildNeeded`

4. Update `EngineManager::build()` staleness handling:
   ```
   match index_manager.staleness_kind(&repo_root)? {
       Current => {},
       CommitChanged => index_manager.update(...),
       TypesChanged(types) => index_manager.rebuild_types(&types, ...),
       FullRebuildNeeded => index_manager.rebuild(...),
   }
   ```

5. Tests:
   - Modify one type schema → only that type's pages re-indexed
   - Add a new type → pages of new type indexed, others untouched
   - Remove a type → pages of that type deleted from index
   - Commit change (no schema change) → incremental update, not rebuild

**Scope:** `src/index_manager.rs`, `src/engine.rs`,
`tests/index_manager.rs`.

## 6. ops module test coverage

**Priority:** Quality — do whenever.

**Problem:** The new schema operations (`schema_list`, `schema_show`,
etc.) are tested in `tests/schema_integration.rs` but not in
`tests/ops.rs`. The ops test file covers spaces, config, content,
search, list, ingest, index, graph — but not schema.

**Fix:** Add schema ops tests to `tests/ops.rs`, or accept the
current split (ops.rs tests the original ops, schema_integration.rs
tests the new ones).

**Scope:** `tests/ops.rs`.

## 7. Wiki logs

**Priority:** Operational — independent of everything.

**Problem:** `llm-wiki serve` writes logs to `~/.llm-wiki/logs/` via
`tracing-appender`, but there's no CLI command to inspect, tail, or
manage logs. No log level control at runtime.

**Fix:**
- `llm-wiki logs tail` — stream recent log entries
- `llm-wiki logs clear` — rotate/delete old logs
- Runtime log level via env var or config
- Document log format and rotation in user-facing docs

**Scope:** `src/cli.rs`, new `src/ops/logs.rs`, `docs/`.
