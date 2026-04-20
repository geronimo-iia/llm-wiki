---
title: "Interlude — Pre-Phase 3 Improvements"
summary: "Engineering improvements to tackle between Phase 2 and Phase 3."
status: ready
last_updated: "2025-07-18"
---

# Interlude — Pre-Phase 3 Improvements

Improvements to address before starting Phase 3 (Typed Graph). Ordered
by priority — correctness first, then simplification, then quality.

## 1. Schema file content in hash

**Priority:** Must fix — blocks Phase 3.

**Problem:** `schema_hash` is computed from `schema_path` + `aliases`
per type (in-memory metadata). It does not hash the actual schema file
content. If a schema file is modified without changing aliases (e.g.
adding `x-graph-edges` in Phase 3), the hash doesn't change and the
index isn't rebuilt.

**Current code:** `type_registry::compute_hashes(&types)` hashes
`RegisteredType.schema_path` and `RegisteredType.aliases` — both are
metadata extracted at build time, not file content. Uses
`std::collections::hash_map::DefaultHasher` which is not stable
across Rust versions or binary rebuilds.

**Design:**

Two changes:

**A. Fix `compute_hashes` to include file content and use SHA-256.**

Add `sha2` crate to `Cargo.toml`. Replace `DefaultHasher` with
SHA-256 throughout hash computation. This makes hashes stable across
binary rebuilds and Rust version upgrades.

The existing `compute_hashes(&types)` in `type_registry.rs` runs at
build time. Fix it to include `rt.content_hash` (a SHA-256 hex string
of the schema file bytes, computed once at parse time and stored in
`RegisteredType`).

`SpaceTypeRegistry::schema_hash()` then reflects the actual file
content at the time the registry was built. It's written to
`state.toml` after rebuild.

**B. Add `compute_disk_hashes(repo_root)` for staleness checks.**

A standalone function that reads schema files from disk and computes
hashes without building a full registry:

```rust
pub fn compute_disk_hashes(repo_root: &Path) -> Result<(String, HashMap<String, String>)>
```

Algorithm:
1. Scan `schemas/*.json` (sorted by filename)
2. For each file: read bytes, compute SHA-256 of content
3. Read `x-wiki-types` from each file to map type_name to file
   content hash
4. Read `wiki.toml` `[types.*]` overrides: for each override, hash
   the referenced schema file content
5. Per-type hash = SHA-256 of the schema file content that serves
   that type
6. Global hash = SHA-256 of all per-type hashes combined (sorted by
   type name)

For embedded defaults (no `schemas/` dir): hash the embedded strings.

This function is called:
- At startup: `compute_disk_hashes` vs `state.toml` to check stale
- By `index_status`: same comparison

After rebuild, `state.toml` is written with
`registry.schema_hash()` (from build time) — which equals
`compute_disk_hashes()` since nothing changed between build and write.

**Where it lives:** `src/type_registry.rs` — alongside the existing
`compute_hashes`.

**Dependencies:**

```toml
sha2 = "0.10"
```

Used for all hash computation (content hashes, per-type hashes,
global hash). Replaces `DefaultHasher` entirely in hash-related code.

**Impact on existing code:**

- `Cargo.toml` — add `sha2 = "0.10"`
- `RegisteredType` — add `content_hash: String` field (SHA-256 hex
  of schema file bytes, computed at parse time)
- `compile_schema` in `type_registry.rs` — accept content hash
  parameter (or compute from content string passed in)
- `compute_hashes` — replace `DefaultHasher` with SHA-256, include
  `rt.content_hash` in the per-type hash
- `space_builder.rs` — compute SHA-256 of file content when parsing
  each schema file, pass into `RegisteredType`
- `discover_from_dir` in `type_registry.rs` — same: compute content
  hash, pass to `RegisteredType`
- `discover_from_embedded` in `type_registry.rs` — hash embedded
  string content
- `indexing::index_status` — use `compute_disk_hashes(repo_root)`
  instead of receiving `current_schema_hash` parameter
- `engine.rs` — startup uses `compute_disk_hashes` for staleness
  check instead of `type_registry.schema_hash()`
- `ops/index.rs` — `index_status` no longer passes `schema_hash`
  parameter
- `SpaceTypeRegistry::schema_hash()` — stays, now reflects content
- All tests that pass `schema_hash` to `index_status` — update
  (function signature changes)
- `tests/indexing.rs` — `status_stale_on_schema_hash_mismatch` test
  now works by modifying a file on disk instead of passing a fake hash

**Migration from DefaultHasher:**

Existing `state.toml` files have hashes computed with `DefaultHasher`.
After this change, the new SHA-256 hashes won't match. This is treated
as stale, triggering a full rebuild on first run. Correct and expected.

**Tests:**

- Modify a schema file on disk: `compute_disk_hashes` returns
  different global hash
- Add a property to a schema (no alias change): hash changes
- Add `x-graph-edges` to a schema: hash changes
- Two wikis with identical schemas: same hash
- Embedded fallback: stable hash across calls
- `index_status` reports stale after schema file modification
- `rebuild_index` writes correct hash to `state.toml`
- Round-trip: rebuild then no modification then not stale
- Hash is deterministic (same input, same output)
- Hash is stable across process restarts (SHA-256, not DefaultHasher)

## 2. Remove hardcoded `IndexSchema::build()`

**Priority:** Simplifies Phase 3 work.

**Problem:** `IndexSchema::build(tokenizer)` is a hardcoded constructor
with a fixed field set (title, summary, type, status, tags). It does
not reflect actual schema files and will drift further as Phase 3 adds
fields via `x-graph-edges`. Three test files still use it.

**What stays:**

- `IndexSchema::build_from_schemas(repo_root, tokenizer)` — standalone
  constructor that reads schema files and classifies fields. This is
  the unit-testable entry point for `IndexSchema` in isolation (no
  registry needed). Used by 14 tests in `tests/index_schema.rs`.
- `space_builder::build_space()` — production path that builds both
  `SpaceTypeRegistry` + `IndexSchema` in one pass.

These two are not duplicates: `build_from_schemas` tests index field
classification without coupling to the registry; `build_space` is the
full production build.

**Fix:** Remove `IndexSchema::build()`. Migrate its 3 test callers
(`tests/indexing.rs`, `tests/search.rs`, `tests/graph.rs`) to use the
`IndexSchema` returned by `build_space_from_embedded()`.

**Why before Phase 3:** Phase 3 adds new field classifications. The
hardcoded `build()` would silently miss them, causing test/production
divergence.

**Scope:** `src/index_schema.rs`, `tests/indexing.rs`,
`tests/search.rs`, `tests/graph.rs`.

## 3. Index lifetime in MCP server

**Priority:** Performance — noticeable on large wikis.

**Problem:** Every tool call that touches the tantivy index opens it
from disk (`Index::open()`, `reader()`, `searcher()`). For the MCP
server (long-running, many calls), this is wasteful.

**Fix:** Store `tantivy::Index` in `SpaceState`, opened once at
startup. `IndexReader` auto-reloads after commits. Search/list get a
`Searcher` from the reader (cheap). Write operations get a writer
from the index.

**Scope:** `src/engine.rs` (`SpaceState`), `src/indexing.rs`,
`src/search.rs`.

## 4. Partial index rebuild

**Priority:** Optimization — not blocking.

**Problem:** Any `schema_hash` mismatch triggers a full rebuild. If
only one type's schema changed, all pages are re-indexed.

**Fix:** Compare per-type hashes (already stored in `state.toml`).
If only some types changed, re-index only pages of those types via
`indexing::rebuild_types(types: &[String])`.

**Scope:** `src/indexing.rs`, `src/engine.rs`.

## 5. ops module test coverage

**Priority:** Quality — do whenever.

**Problem:** The new schema operations (`schema_list`, `schema_show`,
etc.) are tested in `tests/schema_integration.rs` but not in
`tests/ops.rs`. The ops test file covers spaces, config, content,
search, list, ingest, index, graph — but not schema.

**Fix:** Add schema ops tests to `tests/ops.rs`, or accept the
current split (ops.rs tests the original ops, schema_integration.rs
tests the new ones).

**Scope:** `tests/ops.rs`.

## 6. Wiki logs

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
