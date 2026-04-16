# Phase 2 — Core Write Loop: Ingest + Page Creation

## Context

Phase 1 is complete. `config.rs`, `spaces.rs`, `git.rs`, `cli.rs` are
implemented and tested. You are now implementing Phase 2.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every type, function name, and signature must match the spec exactly.
- Do not add fields, methods, or behaviour not described in the specs.
- Do not modify any file under `docs/`.
- Do not modify Phase 1 modules unless fixing a compilation error.
- After each module is complete, run `cargo test` and fix errors before
  moving to the next module.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/core/page-content.md`
- `docs/specifications/core/repository-layout.md`
- `docs/specifications/core/frontmatter-authoring.md`
- `docs/specifications/pipelines/ingest.md`
- `docs/specifications/pipelines/asset-ingest.md`
- `docs/specifications/commands/page-creation.md`
- `docs/specifications/commands/cli.md`

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/frontmatter.rs`

Implement `PageFrontmatter` and all functions listed under
`### frontmatter.rs` in `docs/tasks.md` Phase 2.

The full field list for `PageFrontmatter` is in
`docs/specifications/core/page-content.md` §2.

### 2. `src/markdown.rs`

Implement all functions listed under `### markdown.rs` in `docs/tasks.md`
Phase 2. Slug resolution rules are in
`docs/specifications/core/repository-layout.md` §Slug Resolution.

### 3. `src/ingest.rs`

Implement `IngestOptions`, `IngestReport`, and `ingest()` as listed under
`### ingest.rs` in `docs/tasks.md` Phase 2.

The validation rules are in `docs/specifications/pipelines/ingest.md` §2.
Asset detection rules are in
`docs/specifications/pipelines/asset-ingest.md` §2.

### 4. `src/lib.rs`

Add module declarations: `frontmatter`, `markdown`, `ingest`.

### 5. `src/cli.rs`

Add Phase 2 commands: `ingest`, `new page`, `new section`.
See `docs/specifications/commands/cli.md` for exact flags.

### 6. `src/mcp.rs`

Add Phase 2 tools: `wiki_write`, `wiki_ingest`, `wiki_new_page`,
`wiki_new_section`. Tool signatures are in
`docs/specifications/pipelines/ingest.md` §7 and
`docs/specifications/commands/page-creation.md` §6.

### 7. `tests/frontmatter.rs`

Write all tests listed under `### tests/frontmatter.rs` in
`docs/tasks.md` Phase 2.

### 8. `tests/markdown.rs`

Write all tests listed under `### tests/markdown.rs` in
`docs/tasks.md` Phase 2.

### 9. `tests/ingest.rs`

Write all tests listed under `### tests/ingest.rs` in
`docs/tasks.md` Phase 2.

## Exit criteria

Before marking Phase 2 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki new page wiki://test/concepts/foo` creates a scaffolded page
      and commits
- [ ] `wiki ingest wiki/concepts/foo.md` validates, commits, and indexes
- [ ] `wiki ingest wiki/` ingests all pages recursively
- [ ] `--dry-run` shows what would happen without committing
