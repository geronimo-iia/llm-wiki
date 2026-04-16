# Phase 5 — Lint + Graph

## Context

Phases 1–4 are complete. The wiki can ingest, validate, search, and read
pages. You are now implementing structural auditing and graph generation.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every type, function name, and signature must match the spec exactly.
- Do not add fields, methods, or behaviour not described in the specs.
- Do not modify any file under `docs/`.
- Do not modify Phase 1–4 modules unless fixing a compilation error.
- After each module is complete, run `cargo test` and fix errors before
  moving to the next module.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/commands/lint.md`
- `docs/specifications/commands/graph.md`
- `docs/specifications/commands/cli.md`
- `docs/specifications/llm/backlink-quality.md`
- `docs/specifications/core/source-classification.md`

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/links.rs`

Implement `extract_links(content) -> Vec<String>` as listed under
`### links.rs` in `docs/tasks.md` Phase 5.

Extracts slugs from:
- `sources` frontmatter field
- `concepts` frontmatter field
- Body `[[wikilinks]]`

### 2. `src/lint.rs`

Implement all types and functions listed under `### lint.rs` in
`docs/tasks.md` Phase 5.

Key constraints from `docs/specifications/commands/lint.md`:
- `LINT.md` is written at the **repository root**, not inside `wiki/`
- All 5 sections always present, even when empty
- Empty sections show `_No X found._`
- `LINT.md` must not have frontmatter
- `LINT.md` is excluded from indexing and orphan detection
- Git commit message: `lint: <date> — N orphans, M stubs, K empty sections`

### 3. `src/graph.rs`

Implement all types and functions listed under `### graph.rs` in
`docs/tasks.md` Phase 5.

Key constraints from `docs/specifications/commands/graph.md`:
- Edges from `sources`, `concepts` frontmatter and body `[[links]]`
- Broken references (missing stubs) silently skipped
- Output file gets `status: generated` frontmatter
- Auto-committed if output path is inside wiki root

### 4. `src/lib.rs`

Add module declarations: `links`, `lint`, `graph`.

### 5. `src/cli.rs`

Add Phase 5 commands: `lint`, `lint fix`, `graph`.
See `docs/specifications/commands/cli.md` for exact flags.

### 6. `src/mcp.rs`

Add Phase 5 tools: `wiki_lint`, `wiki_graph`.

### 7. `tests/links.rs`

Write all tests listed under `### tests/links.rs` in `docs/tasks.md`
Phase 5.

### 8. `tests/lint.rs`

Write all tests listed under `### tests/lint.rs` in `docs/tasks.md`
Phase 5.

### 9. `tests/graph.rs`

Write all tests listed under `### tests/graph.rs` in `docs/tasks.md`
Phase 5.

## Exit criteria

Before marking Phase 5 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki lint` writes `LINT.md` at repository root with all 5 sections
- [ ] `wiki lint fix` creates missing stub pages and empty section indexes
- [ ] `wiki graph` outputs Mermaid to stdout
- [ ] `wiki graph --format dot` outputs DOT format
- [ ] `wiki graph --root <slug> --depth 2` outputs correct subgraph
