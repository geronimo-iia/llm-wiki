# Phase 2 — Search + Context

Goal: `wiki search` and `wiki context` work.
External LLM can retrieve relevant pages as Markdown context.

---

## `search.rs`

- [x] Define tantivy schema: fields `slug` (stored), `title` (text+stored), `tags` (text+stored), `body` (text+stored), `type` (stored)
- [x] `build_index(wiki_root: &Path, index_dir: &Path) -> Result<Index>` — walk all `.md` files, parse frontmatter + body, index each
- [x] Skip `raw/` directory (unprocessed source files)
- [x] `search(index: &Index, query: &str, limit: usize) -> Result<Vec<SearchResult>>` — BM25 ranked
- [x] `SearchResult` — fields: `slug`, `title`, `snippet`, `score`, `page_type`
- [x] Auto-build index on first `wiki search` if `.wiki/search-index/` missing
- [ ] **Fix:** `search()` must NOT rebuild on every call — build-on-first-use only
- [ ] Incremental index update: `update_index(wiki_root, changed_slugs)` — add/update
  changed pages, remove deleted pages without full rebuild
- [x] `rebuild_index(wiki_root, index_dir)` — wipe and rebuild from scratch
- [x] Add `.wiki/search-index/` to `.gitignore`

## `context.rs`

- [x] `context(wiki_root: &Path, question: &str, top_k: usize) -> Result<String>`
  - run `search` against the question
  - load full page content for each result
  - format as Markdown: `# {title}\n{body}\n---\n` block per page
- [x] Default `top_k`: 5
- [x] Pages with `type: contradiction` not filtered out — they are high-value context

## CLI

- [x] `wiki search "<term>"` — print results as table: slug, title, score
- [x] `wiki search "<term>" --top <n>` — limit results
- [x] `wiki search --rebuild-index` — rebuild tantivy index, exit
- [x] `wiki context "<question>"` — print assembled Markdown block to stdout
- [x] `wiki context "<question>" --top <n>` — limit page count

## Tests

**Test file:** `tests/search.rs`

### Unit tests

- [x] `search::build_index` — index created, document count matches `.md` file count
- [x] `search::build_index` — `raw/` files excluded from index
- [x] `search::search` — known term in page title returns that page in top result
- [x] `search::search` — known term in page body returns that page in results
- [x] `search::search` — unknown term returns empty results (no panic, no error)
- [x] `search::search` — result order is by descending score
- [x] `context::context` — output contains page titles of top results
- [x] `context::context` — `top_k: 2` returns at most 2 page blocks
- [x] `context::context` — contradiction page included when relevant

### Integration tests

- [x] Ingest 5 pages via `wiki ingest`, then `wiki search` returns ranked results
- [x] `wiki search --rebuild-index` on fresh clone (no existing index) succeeds
- [x] `wiki search` after adding a new page (without explicit rebuild) reflects new page
- [x] `wiki context` output is valid Markdown (no broken headers or fences)
- [x] `wiki context` with no matching pages returns empty string (no error)

## Changelog

- [x] `CHANGELOG.md` — add Phase 2 section: `wiki search`, `wiki context`, tantivy integration, `.gitignore` update

## README

- [x] CLI reference — add `wiki search`, `wiki context` entries
- [x] Usage example — end-to-end: LLM produces analysis → `wiki ingest` → `wiki context` → LLM synthesizes

## Dev documentation

- [x] `docs/dev/search.md` — tantivy schema fields, index lifecycle, rebuild policy, gitignore rationale
- [x] Document `SearchResult` fields
- [x] Document `context` output format (what an LLM receives)
- [x] Update `docs/dev/architecture.md` — mark Phase 2 modules as implemented
