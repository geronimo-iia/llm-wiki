---
title: "Roadmap"
summary: "Development roadmap for llm-wiki — from focused engine to skill registry."
status: draft
last_updated: "2025-07-17"
---

# Roadmap

Three deliverables, four phases. The engine (`llm-wiki`), the skills
(`llm-wiki-skills`), and the type schemas (`schemas/`) evolve together
but release independently.

## Phase 0 — Specification Rationalization ✓

Completed. Fresh specifications written from the design documents.
All specs reviewed and marked ready.

See [decisions/rationalize-specs.md](decisions/rationalize-specs.md)
for the full record of what was done.

## Phase 1 — Focused Engine ✓

Fresh implementation from the specifications. 260 integration tests,
15 MCP tools, ACP agent, stdio + SSE transport. Single Rust binary,
no runtime dependencies.


### Skills (llm-wiki-skills) ✓

- [x] Create the `llm-wiki-skills` git repository
- [x] Set up Claude Code plugin structure
- [x] Write the 11 initial skills:
  - `setup` — install llm-wiki, create and manage wiki spaces
  - `bootstrap` — session orientation
  - `ingest` — source processing workflow
  - `crystallize` — distil session into wiki pages
  - `research` — search, read, synthesize
  - `lint` — structural audit + fix
  - `graph` — generate and interpret concept graph
  - `frontmatter` — frontmatter authoring reference
  - `skill` — find and activate wiki skills
  - `write-page` — create page of any type
  - `configure-hugo` — configure wiki for Hugo rendering
- [ ] Test with `claude --plugin-dir ./llm-wiki-skills`

### Milestone

Engine binary with 15 tools. Skills repo with 11 skills. Claude Code
plugin installable. `llm-wiki serve` + plugin = working system.

## Phase 2 — Type System

JSON Schema validation per type. Type registry in `wiki.toml`.
`schema.md` eliminated.

Dependencies to re-add:
- `jsonschema = "0.28"` — JSON Schema validation on ingest
- `comrak = "0.28"` — Markdown parsing (if needed for content processing)

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

Embedded in the Rust binary via `include_str!()`. On `spaces create`,
the engine writes the embedded strings to `<wiki>/schemas/`. After
that, the wiki's copy is independent — users can modify or add schemas.

### Step 0: `--version` flag

Modules: `src/cli.rs`
Tests: `llm-wiki --version` prints version
Commit: `cli: add --version flag`

Add `#[command(version)]` to the `Cli` struct.

### Step 1: Default JSON Schema files

Modules: `schemas/*.json` (new, repo root)
Tests: unit tests — each file is valid JSON, parses as JSON Schema
Commit: `schemas: add 6 default JSON Schema files`

Create the 6 schema files at the engine repo root:
- `base.json` — required: `title`, `type`; optional: `summary`,
  `status`, `last_updated`, `tags`, `owner`, `superseded_by`
- `concept.json` — extends base, adds `read_when` (required),
  `tldr`, `sources`, `concepts`, `confidence`, `claims`
- `paper.json` — same additional fields as concept (shared by all
  source types)
- `skill.json` — standalone schema, `name`/`description` instead of
  `title`/`summary`, `x-index-aliases`, skill-specific fields
- `doc.json` — extends base, adds `read_when`, `sources`
- `section.json` — extends base, no additional fields

All schemas use Draft 2020-12 (`$schema`). Include `x-index-aliases`
on `skill.json`. `x-graph-edges` are added in Phase 3.

### Step 2: Embed schemas in binary

Modules: `src/default_schemas.rs` (new)
Tests: unit test — each embedded string is valid JSON
Commit: `schemas: embed defaults via include_str`

New module with `include_str!("../schemas/<name>.json")` constants
and a `fn default_schemas() -> HashMap<&str, &str>` accessor.
Add `pub mod default_schemas;` to `src/lib.rs`.

### Step 3: `wiki.toml` type registry

Modules: `src/config.rs`
Tests: parse `wiki.toml` with `[types.*]` entries, round-trip
Commit: `config: parse [types.*] from wiki.toml`

Add `TypeEntry { schema: String, description: String }` and
`types: HashMap<String, TypeEntry>` to `WikiConfig`. Parse from
`wiki.toml`. Backward compatible — missing `[types.*]` = empty map.

### Step 4: `spaces create` writes schemas + types

Modules: `src/spaces.rs`
Tests: `create()` writes 6 files to `<wiki>/schemas/`, generates
`wiki.toml` with `[types.*]` entries, content matches embedded
Commit: `spaces: write default schemas and type registry on create`

Update `ensure_structure()` to:
1. Write each embedded schema to `<wiki>/schemas/<name>.json`
2. Generate `wiki.toml` with `[wiki]` identity + `[types.*]` entries
   for all 15 default types pointing to their schema files

Existing wikis are not modified — only new wikis get the schemas.

### Step 5: Dynamic `SpaceTypeRegistry`

Modules: `src/type_registry.rs` (rewrite)
Deps: `jsonschema` already in `Cargo.toml`
Tests: build registry from test schemas on disk, build from embedded
defaults when no schemas/ dir, validate frontmatter, alias extraction,
unknown type fallback, wiki.toml override takes precedence
Commit: `type_registry: dynamic registry from schemas/ + wiki.toml overrides`

Replace the hardcoded `TypeRegistry` with `SpaceTypeRegistry`:
- Scan `schemas/*.json` in the wiki repo, read `x-wiki-types` from
  each to discover types
- For each type, compile `jsonschema::Validator`, extract
  `x-index-aliases`
- Read `[types.*]` from `wiki.toml` — overrides take precedence
- Fallback: if no `schemas/` dir, use embedded default schemas
- `validate(frontmatter, type) -> Result<Vec<Warning>>`
- `aliases(type) -> HashMap<String, String>`
- `schema_hash() -> String` (SHA-256 of all inputs)
- `type_hashes() -> HashMap<String, String>` (per-type)
- Multiple types sharing a schema each compile their own validator
  (negligible cost, simpler code)

### Step 5bis: Base schema invariant

Modules: `src/type_registry.rs`
Tests: missing base.json uses embedded fallback, custom base.json
must declare `default` in `x-wiki-types`, custom base.json must
require `title` and `type` fields, incompatible base.json rejected
at build time
Commit: `type_registry: enforce base schema invariant`

The `default` type (from `base.json`) is the fallback for every
unknown type. The registry must guarantee:

1. A `default` type always exists — if no schema declares it, the
   embedded `base.json` is used as fallback
2. A custom `base.json` must declare `default` in `x-wiki-types`
3. A custom `base.json` must require at least `title` and `type`
   (superset of the embedded base is fine, subset is not)
4. If validation of these invariants fails, `build()` returns an
   error with a clear message

### Step 6: Dynamic `IndexSchema`

Modules: `src/index_schema.rs` (rewrite)
Tests: schema built from registry has correct fields, aliases
resolved, unknown fields become text
Commit: `index_schema: derive tantivy schema from type registry`

Replace the hardcoded field list with dynamic derivation:
1. Collect all fields across all type schemas (after alias resolution)
2. Classify by JSON Schema type → tantivy field type
3. Add fixed fields (`slug`, `uri`, `body`, `body_links`)
4. Build tantivy schema

The `IndexSchema` struct gains `aliases` and keeps `fields`.

### Step 7: Validation on ingest

Modules: `src/ingest.rs`
Tests: valid frontmatter passes, invalid rejected, alias resolution
applied before indexing, unknown type uses default schema
Commit: `ingest: JSON Schema validation + alias resolution`

Update the ingest pipeline:
1. Read page `type` → look up in `SpaceTypeRegistry`
2. Fall back to `default` if not found
3. Validate frontmatter against the type's compiled JSON Schema
4. Apply `x-index-aliases` before indexing
5. Index all canonical fields

Strict vs loose controlled by `validation.type_strictness`.

### Step 8: Schema change detection

Modules: `src/engine.rs`, `src/search.rs`
Tests: change a schema file → `has_changed()` returns true,
rebuild triggered, `state.toml` updated with new hashes
Commit: `engine: schema change detection via schema_hash`

Update `EngineManager.build()`:
1. Build `SpaceTypeRegistry` per wiki
2. Compare `schema_hash` with `state.toml`
3. Mismatch → full index rebuild with new schema
4. Store `schema_hash` and per-type hashes in `state.toml`

Update `SpaceState` to hold `SpaceTypeRegistry` instead of bare
`TypeRegistry`.

### Step 9: `llm-wiki schema` CLI + `wiki_schema` MCP tool

Modules: `src/cli.rs`, `src/ops.rs`, `src/mcp/tools.rs`,
`src/mcp/handlers.rs`
Tests: `schema list` returns all types, `schema show concept`
returns JSON Schema, `schema show concept --template` returns
frontmatter template, `schema add` registers custom type
Commit: `schema: CLI command and MCP tool for type introspection`

CLI subcommands:
- `llm-wiki schema list` — types + descriptions from registry
- `llm-wiki schema show <type>` — print JSON Schema
- `llm-wiki schema show <type> --template` — generate YAML
  frontmatter template from required/optional fields
- `llm-wiki schema add <type> <schema-path>` — copy schema to
  `<wiki>/schemas/`, add `[types.<type>]` to `wiki.toml`

MCP tool `wiki_schema` with `action` parameter (`list`, `show`,
`add`) — same logic via `src/ops.rs`.

### Step 10: Integration tests

Modules: `tests/schema_integration.rs` (new)
Tests: full integration test suite
Commit: `tests: schema integration tests`

- Each default schema is valid JSON Schema (Draft 2020-12)
- Each per-type template passes its own schema validation
- `spaces create` writes all 6 schemas, content matches embedded
- Round-trip: write template frontmatter → ingest → no errors
- `x-index-aliases` resolve correctly for skill type
- Unrecognized type falls back to `base.json` validation
- Custom schema via `schema add` validates on ingest
- Schema change triggers rebuild (modify schema, re-ingest)
- `schema list` / `schema show` return correct output
- All existing Phase 1 tests still pass

### Skills (llm-wiki-skills)

- [ ] Update `frontmatter` skill with type-specific guidance
- [ ] Update `bootstrap` skill to read types from `wiki_config`
- [ ] Update `ingest` skill to reference type validation
- [ ] Update `write-page` skill to use `wiki_schema show --template`

### Milestone

Type-specific JSON Schema validation on ingest. Field aliasing for
skill and doc pages. Schema introspection via CLI and MCP. Custom
types addable via `wiki.toml` + schema file.

## Phase 3 — Typed Graph

`x-graph-edges` in type schemas. Typed nodes and labeled edges.
`wiki_graph` filters by relation.

### Engine (llm-wiki)

- [ ] Implement `x-graph-edges` parsing from JSON Schema files
- [ ] At ingest: read edge declarations, index edges with relation labels
- [ ] At graph build: petgraph nodes get `type` label, edges get
  `relation` label
- [ ] `wiki_graph --relation <label>` — filter edges by relation
- [ ] Mermaid and DOT output include relation labels
- [ ] Warn on ingest when edge target has wrong type

### Default edge declarations

| Schema | Field | Relation | Target types |
|--------|-------|----------|-------------|
| `concept.json` | `sources` | `fed-by` | All source types |
| `concept.json` | `concepts` | `depends-on` | `concept` |
| `concept.json` | `superseded_by` | `superseded-by` | Any |
| `paper.json` | `sources` | `cites` | All source types |
| `paper.json` | `concepts` | `informs` | `concept` |
| `paper.json` | `superseded_by` | `superseded-by` | Any |
| `skill.json` | `document_refs` | `documented-by` | `doc` |
| `skill.json` | `superseded_by` | `superseded-by` | Any |
| `doc.json` | `sources` | `informed-by` | All source types |
| `doc.json` | `superseded_by` | `superseded-by` | Any |

Body `[[wiki-links]]` get a generic `links-to` relation.

### Skills (llm-wiki-skills)

- [ ] Update `graph` skill with relation-aware instructions
- [ ] Update `lint` skill to detect type constraint violations

### Milestone

Labeled graph edges. Relation-filtered graph output. Type constraint
warnings on ingest.

## Phase 4 — Skill Registry

The wiki becomes a full skill registry.

### Engine (llm-wiki)

- [ ] Verify `wiki_search --type skill` works end-to-end with
  `x-index-aliases`
- [ ] Verify `wiki_list --type skill` returns skill-specific metadata
- [ ] Verify `wiki_graph` renders skill edges correctly
- [ ] Cross-wiki skill discovery: `wiki_search --type skill --all`

### Skills (llm-wiki-skills)

- [ ] Finalize `skill` skill — find, read, activate wiki skills
- [ ] Document the skill authoring workflow
- [ ] Add example wiki skills to the README

### Milestone

Wiki as skill registry. Agents discover skills via search, read them
via `wiki_content_read`, activate them by injecting the body into
context.

## Future

Ideas that don't fit in the four phases:

- `wiki_diff` — changes between two commits for a page
- `wiki_history` — git log for a specific page
- `wiki_search` facets — type/status/tag distributions alongside results
- `wiki_export` — static site, PDF, or EPUB
- Cross-wiki links — `wiki://<name>/<slug>` resolved in graph and search
- Webhook on ingest — notify external systems
- `wiki_watch` — filesystem watcher that auto-ingests on save
- Skill composition — `extends` field for wiki skills
- Confidence propagation — compute concept confidence from source graph
- Persistent graph index — avoid rebuilding petgraph on every call

## Related: llm-wiki-hugo-cms

A separate project that renders a wiki as a Hugo site. The wiki is the
CMS, Hugo is the renderer. See
[decisions/three-repositories.md](decisions/three-repositories.md) for
why it's a separate repo.
