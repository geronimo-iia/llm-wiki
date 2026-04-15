# Phase 1 — Core Write Loop

Goal: `wiki ingest <file|->` works end-to-end.
Pages appear on disk with correct frontmatter, committed to git.

---

## `markdown.rs`

- [x] `parse_frontmatter(content: &str) -> Result<(PageFrontmatter, &str)>` — split YAML block from body
- [x] `write_page(path: &Path, frontmatter: &PageFrontmatter, body: &str) -> Result<()>` — emit `---\n<yaml>\n---\n<body>`
- [x] Generate frontmatter from `SuggestedPage` fields (`slug`, `title`, `type`, `tags`, `tldr`, `read_when`)
- [x] `status` defaults to `active` on create; `last_updated` set to today

## `integrate.rs`

- [x] `integrate(analysis: Analysis, wiki_root: &Path) -> Result<IngestReport>`
- [x] `action: create` — write `{slug}.md`; return error if file already exists
- [x] `action: update` — replace body, merge frontmatter fields (append tags, update `last_updated`)
- [x] `action: append` — add new Markdown section to end of existing body
- [x] Ensure parent directory exists before writing (`concepts/`, `sources/`, `queries/`)
- [x] Write `contradictions/*.md` for each entry in `contradictions[]` if non-empty; skip silently if empty
- [x] Validate `slug` — reject path traversal (`../`), enforce `concepts/|sources/|queries/|contradictions/` prefix
- [x] `IngestReport` — counts: pages created, pages updated, pages appended, contradiction files written

## `git.rs`

- [x] `init_if_needed(root: &Path) -> Result<()>` — `git init` if no `.git` present
- [x] `stage_all(root: &Path) -> Result<()>` — stage all modified/new files under wiki root
- [x] `commit(root: &Path, message: &str) -> Result<()>` — commit with message
- [x] Commit message format: `"ingest: <title> — +N pages"` (using `IngestReport` counts)

## `ingest.rs`

- [x] Resolve input: file path or `-` (stdin)
- [x] Deserialize `analysis.json` — clear error on malformed JSON
- [x] Validate `doc_type` against enum; reject unknown values with list of valid options
- [x] Validate `action` on each `suggested_page` — reject unknown values
- [x] Validate `slug` — no path traversal, known prefix
- [x] `action: create` on existing slug → error listing the conflicting path
- [x] `action: update|append` on missing slug → error suggesting `create`
- [x] Call `integrate`, then `git::commit`
- [x] Print `IngestReport` summary to stdout on success

## CLI

- [x] `wiki ingest <file>` — read from file path
- [x] `wiki ingest -` — read from stdin
- [x] Exit code 1 on validation error, 0 on success

## Tests

**Test file:** `tests/ingest.rs`

### Unit tests

- [x] `markdown::parse_frontmatter` — valid YAML block → correct struct fields
- [x] `markdown::parse_frontmatter` — missing required field → error naming the field
- [x] `markdown::parse_frontmatter` — no frontmatter block → error
- [x] `markdown::write_page` — output starts with `---`, contains all frontmatter fields
- [x] `markdown::write_page` + `parse_frontmatter` round-trip — fields identical after round-trip
- [x] `integrate` — `action: create` writes file at `{wiki_root}/{slug}.md`
- [x] `integrate` — `action: create` on existing slug → error
- [x] `integrate` — `action: update` replaces body, preserves other frontmatter
- [x] `integrate` — `action: append` adds section, original body intact
- [x] `integrate` — `contradictions[]` non-empty → contradiction files written
- [x] `integrate` — `contradictions[]` empty → no files in `contradictions/`
- [x] `integrate` — slug with `../` → rejected
- [x] `ingest` — unknown `doc_type` → error with valid values listed
- [x] `ingest` — invalid JSON → error with line/column hint

### Integration tests

- [x] `wiki ingest file.json` — correct `.md` on disk, `git log` shows one commit
- [x] `wiki ingest -` from stdin — same result
- [x] Two consecutive `wiki ingest` with `action: create` on same slug → second fails, first commit preserved
- [x] `wiki ingest` with `action: append` — body grows, frontmatter `last_updated` changes
- [x] Path traversal in slug → rejected, no files written, no commit

## Changelog

- [x] `CHANGELOG.md` — add Phase 1 section: `wiki ingest`, `analysis.json` contract, `action` semantics

## README

- [x] **Install** section — `cargo install llm-wiki`, minimum Rust version
- [x] **Quick start** — `wiki init <path>`, then `wiki ingest analysis.json`
- [x] **CLI reference** table — Phase 1 commands with one-line descriptions
- [x] **`analysis.json` contract** — minimal example, link to design doc

## Dev documentation

- [x] `docs/dev/ingest.md` — pipeline walkthrough: deserialize → validate → integrate → commit
- [x] Document `action` semantics (`create`/`update`/`append`) with examples
- [x] Document `IngestReport` fields
- [x] Document validation rules (slug format, `doc_type` enum, `action` enum)
- [x] Update `docs/dev/architecture.md` — mark Phase 1 modules as implemented
