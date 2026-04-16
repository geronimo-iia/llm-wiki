# Phase 3 — Frontmatter Validation + Type Taxonomy

## Context

Phases 1 and 2 are complete. `frontmatter.rs` exists but has no
validation logic yet. You are now adding validation and writing
`src/instructions.md`.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every type, function name, and signature must match the spec exactly.
- Do not add fields, methods, or behaviour not described in the specs.
- Do not modify any file under `docs/`.
- Do not modify Phase 1–2 modules unless fixing a compilation error.
- After each change, run `cargo test` and fix errors before continuing.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/core/page-content.md`
- `docs/specifications/core/frontmatter-authoring.md`
- `docs/specifications/core/source-classification.md`
- `docs/specifications/commands/configuration.md` — `[validation]` section
- `docs/specifications/pipelines/ingest.md` §2 — validation table

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/frontmatter.rs` — validation additions

Add to the existing `frontmatter.rs`:

- Built-in type list (exact list in
  `docs/specifications/core/source-classification.md` §1 +
  `docs/specifications/core/frontmatter-authoring.md` §2)
- `validate_frontmatter(fm, schema) -> Result<Vec<Warning>>` as listed
  under `### frontmatter.rs — validation` in `docs/tasks.md` Phase 3

`strict` vs `loose` behaviour is driven by `ValidationConfig` from
`config.rs`. Do not hardcode either mode.

### 2. `src/ingest.rs` — wire validation

Call `validate_frontmatter` on every `.md` file during ingest.
Respect `validation.type_strictness` from the resolved config.
Include warnings in `IngestReport.warnings`.

### 3. `src/instructions.md`

Write all workflow sections listed under `### src/instructions.md` in
`docs/tasks.md` Phase 3.

The `## frontmatter` section content is specified in
`docs/specifications/core/frontmatter-authoring.md` §8.
The other workflow sections are specified in
`docs/specifications/commands/instruct.md` §2.
The session orientation and linking policy preambles are specified in
`docs/specifications/llm/session-bootstrap.md` §5 and
`docs/specifications/llm/backlink-quality.md` §5.

Write condensed, token-efficient versions — no rationale paragraphs.

### 4. `tests/frontmatter.rs` — Phase 3 additions

Add the validation tests listed under
`### tests/frontmatter.rs — Phase 3 additions` in `docs/tasks.md`.

## Exit criteria

Before marking Phase 3 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki ingest` warns on missing `read_when`
- [ ] `wiki ingest` warns on `source-summary` type
- [ ] `wiki ingest` rejects unknown type when `type_strictness = "strict"`
- [ ] `wiki instruct frontmatter` prints the frontmatter guide
