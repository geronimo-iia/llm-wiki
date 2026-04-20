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

## 3. Introduce `SpaceIndexManager`

**Priority:** Structural — prerequisite for index lifetime (§4).

**Problem:** Index operations are free functions in `src/indexing.rs`
that take `index_path`, `wiki_name`, `repo_root`, `wiki_root`, `is`,
and `registry` as parameters on every call. There's no single owner
of per-wiki index state. The parameters are threaded through from
`engine.rs` → `ops/` → `indexing.rs` on every operation.

**Design:**

A `SpaceIndexManager` struct that owns the per-wiki index context.
The logic lives in the methods directly — not a wrapper over free
functions.

```rust
pub struct SpaceIndexManager {
    wiki_name: String,
    index_path: PathBuf,
}

impl SpaceIndexManager {
    pub fn new(wiki_name: &str, index_path: PathBuf) -> Self;

    pub fn rebuild(
        &self,
        wiki_root: &Path,
        repo_root: &Path,
        is: &IndexSchema,
        registry: &SpaceTypeRegistry,
    ) -> Result<IndexReport>;

    pub fn update(
        &self,
        wiki_root: &Path,
        repo_root: &Path,
        is: &IndexSchema,
        wiki_name: &str,
        registry: &SpaceTypeRegistry,
    ) -> Result<UpdateReport>;

    pub fn status(&self, repo_root: &Path) -> Result<IndexStatus>;

    pub fn last_commit(&self) -> Option<String>;

    pub fn delete_by_type(&self, is: &IndexSchema, type_name: &str) -> Result<()>;

    pub fn open_or_recover(
        &self,
        is: &IndexSchema,
        recovery: Option<&RecoveryContext<'_>>,
    ) -> Result<Index>;
}
```

The document-building helpers (`build_document`, `yaml_to_text`,
`index_value`, etc.) stay as private functions in the same module —
they're implementation details, not part of the public API.

Data types (`IndexReport`, `UpdateReport`, `IndexStatus`, `IndexState`,
`RecoveryContext`) stay public in the same module.

`SpaceState` holds the manager:

```rust
pub struct SpaceState {
    pub name: String,
    pub wiki_root: PathBuf,
    pub repo_root: PathBuf,
    pub type_registry: SpaceTypeRegistry,
    pub index_schema: IndexSchema,
    pub index_manager: SpaceIndexManager,
}
```

**Migration strategy (incremental, never breaks tests):**

1. Copy `src/indexing.rs` → `src/index_manager.rs`
2. Add `pub mod index_manager` to `lib.rs` (both modules coexist)
3. In `index_manager.rs`: introduce the `SpaceIndexManager` struct,
   convert free functions to methods. Keep the public free functions
   as `#[deprecated]` thin wrappers that delegate to the struct
   (so existing callers still compile).
4. Migrate callers one at a time:
   a. `src/engine.rs` — use `SpaceIndexManager` in `SpaceState`
   b. `src/ops/index.rs` — call `space.index_manager.rebuild()` etc.
   c. `src/ops/search.rs` — use `space.index_manager.index_path()`
   d. `src/search.rs` — unchanged (still takes `index_path`)
   e. `src/graph.rs` — unchanged (still takes `index_path`)
   f. Tests — migrate imports from `indexing::*` to `index_manager::*`
5. Once no caller uses `indexing::*`, remove `src/indexing.rs` and
   the deprecated wrappers from `index_manager.rs`

Each step compiles and all tests pass.

**What this is NOT:**

- Not a wrapper/delegation layer that lives permanently alongside
  `indexing.rs` — the deprecated wrappers are temporary scaffolding
- Not a big-bang rename — callers migrate incrementally

**Why before §4:** Section 4 adds `Index` + `IndexReader` fields.
  They belong inside the manager that already owns `index_path` and
  the rebuild/update logic. Without the struct, those fields land in
  `SpaceState` — mixing concerns.

**Scope:** new `src/index_manager.rs`, then incremental migration of
`src/engine.rs`, `src/ops/index.rs`, `src/ops/search.rs`, `src/lib.rs`,
all tests. Finally remove `src/indexing.rs`.

## 4. Index lifetime in MCP server

**Priority:** Performance — noticeable on large wikis.

**Problem:** Every tool call that touches the tantivy index opens it
from disk (`MmapDirectory::open` + `Index::open` + `reader()`).
For the MCP server (long-running, many calls per session), this adds
measurable latency on large wikis.

**Design:**

Add `Index` + `IndexReader` fields to `SpaceIndexManager` (from §3).
The manager opens the index once at startup and exposes `searcher()`:

```rust
pub struct SpaceIndexManager {
    wiki_name: String,
    index_path: PathBuf,
    tantivy_index: Option<Index>,
    index_reader: Option<IndexReader>,
}

impl SpaceIndexManager {
    /// Get a searcher (cheap — arc clone of current segment set)
    pub fn searcher(&self) -> Result<Searcher>;

    /// Get a writer for mutations (rebuild, update, delete_by_type)
    fn writer(&self) -> Result<IndexWriter>;

    // rebuild, update, status, etc. now use self.writer() internally
}
```

The reader uses `ReloadPolicy::OnCommitWithDelay` so it automatically
picks up new segments after `writer.commit()`.

The ops layer obtains the searcher from the manager and passes it to
the lower-level modules. `search.rs` and `graph.rs` become pure
functions that accept `&Searcher` — they never open an index.

**Changes:**

- `src/index_manager.rs` — add `tantivy_index`, `index_reader` fields.
  Add `searcher()` method. `rebuild()` and `update()` use internal
  `writer()` instead of opening from disk. Open index at startup.

- `src/search.rs` — change `search()` and `list()` to accept
  `&Searcher` + `&IndexSchema`. Remove all index-opening code.

- `src/graph.rs` — change `build_graph()` to accept `&Searcher` +
  `&IndexSchema`. Remove direct index opening.

- `src/ops/search.rs` — call `space.index_manager.searcher()`, pass
  to `search::search()` / `search::list()`.

- `src/ops/graph.rs` — same: pass searcher to `graph::build_graph()`.

**Lifecycle:**

```
startup:
  index_manager.rebuild() if needed (uses internal writer)
  index_manager.open() → store Index + IndexReader

tool call (search/list/graph):
  ops: space.index_manager.searcher() → pass to search/graph

tool call (ingest/rebuild):
  index_manager.update() / rebuild() → internal writer → commit
  (reader auto-reloads on next searcher() call)
```

**Edge cases:**

- Index doesn't exist at startup: fields are `None`, `searcher()`
  returns error "index not available"
- Index rebuilt externally (CLI while server runs): reader won't see
  it until server restart
- Concurrent reads: `IndexReader::searcher()` is thread-safe (arc)
- Concurrent writes: `RwLock<Engine>` serializes writes

**Scope:** `src/index_manager.rs`, `src/search.rs`, `src/graph.rs`,
`src/ops/search.rs`, `src/ops/graph.rs`.

## 5. Partial index rebuild

**Priority:** Optimization — not blocking.

**Problem:** Any `schema_hash` mismatch triggers a full rebuild. If
only one type's schema changed, all pages are re-indexed.

**Fix:** Compare per-type hashes (already stored in `state.toml`).
If only some types changed, re-index only pages of those types via
`index_manager.rebuild_types(types: &[String])`.

**Scope:** `src/index_manager.rs`.

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
