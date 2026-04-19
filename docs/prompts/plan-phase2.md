# Plan Phase 2 Implementation

## Context

Phase 1 is complete. The engine has 15 working tools, hardcoded
field-to-index mapping, and base frontmatter validation (`title`
required, `type` non-empty). The codebase lives in `src/` with
reference material in `code-ref/`.

Phase 2 replaces the hardcoded type registry with a dynamic one
driven by JSON Schema files and `wiki.toml` `[types.*]` entries.

## Read first

Read these in order before planning:

1. `docs/roadmap.md` — Phase 2 deliverables
2. `docs/specifications/model/type-system.md` — full type system spec
3. `docs/specifications/model/wiki-toml.md` — `[types.*]` registry format
4. `docs/specifications/model/types/base.md` — base schema, default fallback
5. `docs/specifications/model/types/concept.md` — concept + query-result
6. `docs/specifications/model/types/source.md` — paper, article, etc.
7. `docs/specifications/model/types/skill.md` — skill with field aliasing
8. `docs/specifications/model/types/doc.md` — doc type
9. `docs/specifications/model/types/section.md` — section type
10. `docs/specifications/engine/index-management.md` — schema change detection, partial rebuild
11. `docs/specifications/engine/ingest-pipeline.md` — validate → alias → index → commit

Then read the implementation docs:
- `docs/implementation/type-registry.md` — SpaceTypeRegistry, validators, caching
- `docs/implementation/tantivy.md` — dynamic schema building, field classification
- `docs/implementation/index-manager.md` — SpaceIndexManager, rebuild, staleness
- `docs/implementation/manager-pattern.md` — detect, refresh, cascade

And the current source files that will be modified:
- `src/type_registry.rs` — currently hardcoded, becomes dynamic
- `src/index_schema.rs` — currently hardcoded fields, becomes derived from registry
- `src/search.rs` — rebuild/update use IndexSchema
- `src/ingest.rs` — validation pipeline
- `src/spaces.rs` — `create()` generates schemas/ and wiki.toml types
- `src/engine.rs` — startup loads type registry, change detection
- `src/config.rs` — WikiConfig needs no `[types.*]` (loaded separately)

## Your Task

Implement Phase 2 step by step. The steps are defined in
`docs/roadmap.md` (Steps 0–10). Each step produces a compilable,
testable increment.

## How to use this prompt

1. Tell the agent which step to implement: "Implement Step N"
2. The agent reads the step from `docs/roadmap.md`, reads the
   referenced implementation docs and current source, then implements
3. After each step: `cargo check`, `cargo test`, commit

To resume after a break, say "Continue from Step N".

## Per-step reading

| Step | What | Key files to read |
|------|------|-------------------|
| 0 | `--version` flag | `src/cli.rs` |
| 1 | Default JSON Schema files | `docs/specifications/model/types/*.md` |
| 2 | Embed schemas in binary | `schemas/*.json` (from step 1) |
| 3 | `wiki.toml` type registry | `src/config.rs`, `docs/specifications/model/wiki-toml.md` |
| 4 | `spaces create` writes schemas | `src/spaces.rs`, `src/default_schemas.rs` |
| 5 | Dynamic `SpaceTypeRegistry` | `src/type_registry.rs`, `docs/implementation/type-registry.md` |
| 6 | Dynamic `IndexSchema` | `src/index_schema.rs`, `docs/implementation/tantivy.md` |
| 7a | Extract `indexing.rs` | `src/search.rs`, `src/engine.rs`, `src/ops.rs` |
| 7b | Alias resolution in `build_document` | `src/indexing.rs`, `src/index_schema.rs`, `docs/implementation/ingest-validation.md` |
| 8 | Schema change detection | `src/engine.rs`, `src/indexing.rs`, `docs/implementation/index-manager.md` |
| 9 | `schema` CLI + MCP tool | `src/cli.rs`, `src/ops.rs`, `src/mcp/tools.rs` |
| 10 | Integration tests | All source files from steps 1–9 |

## Constraints

### Schema storage

The 6 default JSON Schema files live at the engine repo root in
`schemas/` and are committed to git:

```
schemas/
  base.json
  concept.json
  paper.json
  skill.json
  doc.json
  section.json
```

The Rust binary embeds them at compile time via `include_str!()`:

```rust
const BASE_SCHEMA: &str = include_str!("../schemas/base.json");
const CONCEPT_SCHEMA: &str = include_str!("../schemas/concept.json");
// ...
```

On `spaces create`, the engine writes the embedded strings to
`<wiki>/schemas/`. After that, the wiki's copy is independent —
users can modify or add schemas there.

This gives us:
- Schemas visible and diffable in the engine repo's git history
- Referenceable by external tools, linters, or editors
- Forkable by users for custom wikis
- No runtime file dependency — the binary is self-contained

### Build order matters

The type system touches multiple modules. Build bottom-up:

```
schemas/*.json (repo root, embedded via include_str!)
  -> type_registry.rs (load schemas, compile validators, extract aliases/edges)
  -> index_schema.rs (derive tantivy schema from type registry)
  -> search.rs (use dynamic IndexSchema for rebuild/update)
  -> ingest.rs (validate against JSON Schema, apply aliases)
  -> spaces.rs (create() writes embedded schemas to wiki/schemas/)
  -> engine.rs (schema_hash change detection, partial rebuild)
```

### Each step must

- Compile (`cargo check`)
- Have tests (`cargo test`)
- Be committable with a meaningful message

### What Phase 2 implements

- 6 default JSON Schema files in engine repo `schemas/`, embedded
  in the binary via `include_str!()`
- `[types.*]` section in `wiki.toml`
- `schemas/` directory in each wiki repo (written from embedded defaults)
- JSON Schema validation on `wiki_ingest` via `jsonschema` crate
- `x-index-aliases` — resolve field aliases at ingest time
  (e.g. skill's `name` → `title`, `description` → `summary`)
- `llm-wiki spaces create` writes embedded schemas to `<wiki>/schemas/`
  and generates `wiki.toml` with `[types.*]` entries
- `wiki_config list` returns type names + descriptions
- Schema change detection via `schema_hash` in `state.toml`
- Per-type hashes for partial rebuild decisions
- `llm-wiki schema` CLI command and `wiki_schema` MCP tool:
  - `schema list` — list registered types with descriptions
  - `schema show <type>` — print the JSON Schema for a type
  - `schema show <type> --template` — print a frontmatter template
  - `schema add <type> <schema-path>` — register a custom type

### What Phase 2 does NOT implement

- `x-graph-edges` typed edges (Phase 3)
- Skill registry features (Phase 4)
- Hot reload / file watcher (future)
- Custom tokenizer registration (future)

### Backward compatibility

- Pages without a `type` field default to `type: page`, validated
  against `[types.default]`
- Pages with an unregistered type are validated against `[types.default]`
- Wikis with no `schemas/` directory use a built-in base schema
- No frontmatter rewriting — existing files are untouched
- Existing Phase 1 wikis continue to work (no schemas/ = base validation)

### What MUST work at the end of Phase 2

- 6 default schemas in engine repo `schemas/`, embedded in binary
- `llm-wiki spaces create` writes embedded schemas to `<wiki>/schemas/`
  and generates `wiki.toml` with `[types.*]` entries
- `wiki_ingest` validates frontmatter against the type's JSON Schema
- `wiki_ingest` resolves `x-index-aliases` before indexing
  (skill pages with `name`/`description` index as `title`/`summary`)
- `wiki_config list` shows registered types
- `llm-wiki schema list/show/add` and `wiki_schema` MCP tool
- Schema change detection triggers rebuild when `wiki.toml` or
  `schemas/` change
- Custom types addable via `wiki.toml` + schema file
- All existing Phase 1 tests still pass
- New tests for schema validation, aliasing, change detection
- Integration tests: each default schema is valid JSON Schema,
  per-type templates pass their own validation, round-trip
  write → ingest succeeds, alias resolution works, fallback
  to base.json for unrecognized types

## Rules for every step

### Before writing code

1. Read the step description in `docs/roadmap.md`
2. Read the implementation doc(s) listed in the table above
3. Read the current source files that will be modified
4. Read `src/lib.rs` to see what modules exist

### While writing code

- Follow `docs/implementation/rust.md` for style, error handling,
  and testing conventions
- Add the module to `src/lib.rs` if new
- Write unit tests in-module (`#[cfg(test)] mod tests`)
- Use `anyhow::Result` for public functions
- Use `tempfile::tempdir()` for any filesystem tests

### After writing code

1. Run `cargo check` — must pass
2. Run `cargo test` — must pass
3. Run `cargo clippy -- -D warnings` — fix any warnings
4. Commit with the message from the step description

### What NOT to implement

- `x-graph-edges` typed edges (Phase 3)
- Skill registry features (Phase 4)
- Hot reload / file watcher (future)
- Custom tokenizer registration (future)
