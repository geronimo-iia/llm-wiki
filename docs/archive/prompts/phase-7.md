# Phase 7 — Search Index Incremental Update

You are implementing Phase 7 of llm-wiki (folder projects/llm-wiki), a Rust CLI and MCP server for a
git-backed knowledge base. Phase 6 (multi-wiki + SSE) is complete.


## Context

The full task list is in `docs/tasks/phase-7.md`.
The design is in `docs/dev/search.md`.
The codebase is a Rust workspace. All source files are in `src/`.

## The Problem

`search()` currently rebuilds the tantivy index from scratch on every call.
This is wrong. The correct behavior:

- **First use** — build index if `.wiki/search-index/` does not exist
- **After ingest** — update index incrementally with only changed slugs
- **Explicit rebuild** — `wiki search --rebuild-index` wipes and rebuilds

## What to implement

### `src/search.rs`

Add two new functions alongside the existing `build_index` and `search`:

1. `open_or_build_index(wiki_root, index_dir) -> Result<Index>`
   — open existing index if valid, build from scratch if missing or corrupt

2. `update_index(wiki_root, index_dir, changed_slugs: &[String]) -> Result<()>`
   — for each slug: delete existing document, re-index from current file on disk
   — for slugs that no longer exist on disk: delete from index only
   — commit writer

Change `search()` to call `open_or_build_index` instead of `build_index`.
Change `search_all()` the same way.

### `src/integrate.rs`

All integrate functions must collect the slugs of pages written or deleted,
then call `search::update_index(wiki_root, index_dir, &changed_slugs)` after
the git commit.

Add `index_updated: bool` to `IngestReport`.

### `src/cli.rs`

`wiki search --rebuild-index` must call `build_index` (wipe + rebuild) — behavior
unchanged. `wiki search "<query>"` must call `open_or_build_index`.

## Tests to add in `tests/search.rs`

- `open_or_build_index` with missing index → builds and returns valid index
- `open_or_build_index` with existing index → opens without rebuilding
- `update_index` new page → appears in search
- `update_index` modified page → updated content in search
- `update_index` deleted page → no longer in search
- `update_index` empty slugs → no-op
- Integration: `wiki ingest` → new page searchable immediately without rebuild
- Integration: two consecutive `wiki search` → index files not modified on second call

## Acceptance

```bash
cargo test
wiki ingest paper.json          # index updated incrementally
wiki search "mixture of experts" # fast — no rebuild
wiki search --rebuild-index      # explicit full rebuild, exits 0
```

## Constraints

- No LLM dependency — do not add any AI/LLM crates
- Follow existing code style in `src/search.rs` and `src/integrate.rs`
- Update `CHANGELOG.md` with a Phase 7 entry
- Update `docs/dev/search.md` to add `update_index` and `open_or_build_index`
  function descriptions
