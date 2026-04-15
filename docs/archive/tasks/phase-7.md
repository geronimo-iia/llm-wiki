# Phase 7 — Search Index — Incremental Update

Goal: the search index is no longer rebuilt on every `wiki search` call.
It is built on first use, updated incrementally after each ingest, and
rebuilt explicitly via `--rebuild-index`. Search results always reflect
the current wiki state without full-rebuild cost on every query.

Depends on: Phase 6 complete.
Design ref: [dev/search.md](../dev/search.md)

Status: **complete**

---

## `search.rs`

- [x] `build_index(wiki_root: &Path, index_dir: &Path) -> Result<Index>`
  — unchanged: walk all `.md` files, parse frontmatter + body, index each
  — called only by `wiki search --rebuild-index`
- [x] `update_index(wiki_root: &Path, index_dir: &Path, changed_slugs: &[String]) -> Result<()>`
  — open existing index
  — for each slug in `changed_slugs`: delete existing document if present,
    re-index from current file on disk
  — for each slug that no longer exists on disk: delete from index
  — commit writer; no-op for empty slug list
- [x] `open_or_build_index(wiki_root: &Path, index_dir: &Path) -> Result<Index>`
  — if `.wiki/search-index/` exists and is valid → open
  — if missing or corrupt → call `build_index`, return result
  — replaces the always-rebuild logic in `search()`
- [x] `search(wiki_root, query, limit)` — calls `open_or_build_index` instead
  of `build_index`; passes `rebuild_index = true` only when flag is set
- [x] `search_all(registry, query, limit)` — uses `open_or_build_index` per wiki

## `integrate.rs`

- [x] `integrate()` collects slugs of all pages written (create/update/append
  and contradictions) into `IngestReport::changed_slugs`
- [x] `IngestReport` gains `index_updated: bool` — true if index was updated
  incrementally, false if index did not exist (built on next search)
- [x] `IngestReport` gains `changed_slugs: Vec<String>` — slugs written during
  the ingest session; used by `ingest.rs` to call `update_index`

## `ingest.rs`

- [x] After git commit: calls `search::update_index(wiki_root, index_dir,
  &report.changed_slugs)` when the index directory already exists

## `git.rs`

No changes. Index update happens after commit, not inside git operations.

## `cli.rs`

- [x] `wiki search --rebuild-index` — calls `build_index` (wipe + rebuild),
  exits 0. Behaviour unchanged.
- [x] `wiki search "<query>"` — calls `open_or_build_index` via `search()`

## Tests

**Test file:** `tests/search.rs`

### Unit tests

- [x] `open_or_build_index` — missing index dir → builds and returns valid index
- [x] `open_or_build_index` — existing index → opens without rebuilding
  (verified by checking mtime of index dir does not change)
- [x] `update_index` — new page added → appears in subsequent search
- [x] `update_index` — existing page modified → updated content appears in search
- [x] `update_index` — page deleted → no longer appears in search
- [x] `update_index` — empty `changed_slugs` → no-op, index mtime unchanged

### Integration tests

- [x] Two consecutive `wiki search` calls → index files not modified on second call
- [x] `cli_search_new_page_reflected_after_update_index` — new page not visible
  before `update_index`, visible after (replaces stale always-rebuild test)

## Changelog

- [x] `CHANGELOG.md` — Phase 7 entry added

## Dev documentation

- [x] `docs/dev/search.md` — Functions section added: `build_index`,
  `open_or_build_index`, `update_index`, `search_index`, `search`, `search_all`
