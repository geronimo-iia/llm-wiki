# Phase 4 — Search + Read + Index

## Context

Phases 1–3 are complete. The wiki can ingest and validate pages.
You are now implementing full-text search, page reading, and index
management.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every type, function name, and signature must match the spec exactly.
- Do not add fields, methods, or behaviour not described in the specs.
- Do not modify any file under `docs/`.
- Do not modify Phase 1–3 modules unless fixing a compilation error.
- After each module is complete, run `cargo test` and fix errors before
  moving to the next module.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/commands/search.md`
- `docs/specifications/commands/read.md`
- `docs/specifications/commands/list.md`
- `docs/specifications/commands/index.md`
- `docs/specifications/commands/cli.md`
- `docs/specifications/core/repository-layout.md` — slug resolution

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/search.rs`

Implement all types and functions listed under `### search.rs` in
`docs/tasks.md` Phase 4.

Key constraints:
- Index stored at `~/.wiki/indexes/<name>/search-index/`
- `state.toml` written alongside the index on every rebuild
- `state.toml` fields: `built`, `pages`, `sections`, `commit`
- Staleness = `state.toml` commit != `git HEAD`
- All frontmatter fields indexed in tantivy, not just title/body/tags
- `PageRef` has no `path` field — machine-local paths are excluded

### 2. `src/lib.rs`

Add module declaration: `search`.

### 3. `src/cli.rs`

Add Phase 4 commands: `search`, `read`, `list`, `index rebuild`,
`index status`. See `docs/specifications/commands/cli.md` for exact
flags.

### 4. `src/mcp.rs`

Add Phase 4 tools: `wiki_search`, `wiki_read`, `wiki_list`,
`wiki_index_rebuild`, `wiki_index_status`. Tool signatures are in the
respective spec files.

### 5. `tests/search.rs`

Write all tests listed under `### tests/search.rs` in `docs/tasks.md`
Phase 4. Use `tempfile::tempdir()` for index storage.

## Exit criteria

Before marking Phase 4 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki search "foo"` returns ranked `Vec<PageRef>` with `wiki://` URIs
- [ ] `wiki search "foo" --no-excerpt` returns refs without excerpts
- [ ] `wiki read wiki://test/concepts/foo` returns full page content
- [ ] `wiki read wiki://test/concepts/foo --no-frontmatter` strips frontmatter
- [ ] `wiki list --type concept` returns paginated concept pages
- [ ] `wiki index status` shows stale/fresh correctly
- [ ] `wiki index rebuild` rebuilds and writes `state.toml`
