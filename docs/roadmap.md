---
title: "Roadmap"
summary: "Development roadmap for llm-wiki — from focused engine to skill registry."
status: ready
last_updated: "2025-07-18"
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

### Step 7a: Extract `indexing.rs` module

Modules: `src/indexing.rs` (new), `src/search.rs` (shrink),
`src/engine.rs`, `src/ops.rs`, `src/lib.rs`
Tests: all existing search/index tests still pass, no behavior change
Commit: `refactor: extract indexing.rs from search.rs`

Mechanical refactor — split `search.rs` into read and write:

**`indexing.rs`** (write path):
- `build_document()` — build tantivy doc from frontmatter
- `rebuild_index()` — full rebuild from wiki tree
- `update_index()` — incremental update from git diff
- `collect_changed_files()` — git diff helper
- `save_state()` / `last_indexed_commit()` — state.toml management
- `IndexReport`, `UpdateReport`, `IndexState` — write-side types

**`search.rs`** (read path, stays):
- `search()` / `search_all()` — BM25 query
- `list()` — paginated listing
- `index_status()` — health check
- `open_index()` — open with recovery
- Read-side types

No behavior change. All callers updated to import from the new module.

### Step 7b: Alias resolution in `build_document`

Modules: `src/indexing.rs`, `src/index_schema.rs`, `src/engine.rs`
Tests: skill page indexed as title/summary, concept fields indexed,
unrecognized fields in body text, existing tests pass
Commit: `ingest: dynamic field indexing with alias resolution`

Update the indexing pipeline:
1. `build_document` gains a `&SpaceTypeRegistry` parameter
2. Dynamic field iteration replaces hardcoded field extraction
3. Alias resolution: `name` → `title`, `description` → `summary`
4. `IndexSchema` gains `keyword_fields: HashSet<String>` to
   distinguish text vs keyword indexing for array values
5. Unrecognized fields appended to body text
6. `rebuild_index` and `update_index` pass registry through
7. `Engine` propagates registry to index operations

### Step 8: Schema change detection

Modules: `src/space_builder.rs` (new), `src/indexing.rs`,
`src/engine.rs`, `src/ops.rs`, `src/type_registry.rs`,
`src/index_schema.rs`
Tests: `build_space` returns consistent registry + index schema,
new wiki has `schema_hash` in `state.toml`, modify schema file →
stale, rebuild updates hash, old `state.toml` with `schema_version`
→ stale, two wikis with different schemas, embedded fallback
Commit: `engine: schema change detection via schema_hash`

Shared builder:
1. New `space_builder.rs` with `build_space(repo_root, tokenizer)`
   that reads schema files once, builds both `SpaceTypeRegistry`
   and `IndexSchema`, discards raw JSON
2. `SpaceTypeRegistry::from_parts()` internal constructor
3. `IndexSchema::build_from_schemas()` becomes private — callers
   use `build_space()`

Per-wiki registry:
4. `SpaceState` gains `type_registry`, `schema` renamed to
   `index_schema`
5. `Engine.type_registry` removed
6. `EngineManager::build()` calls `build_space()` per wiki

Schema hash:
7. `IndexState`: replace `schema_version: u32` with
   `schema_hash: String`, add `types: HashMap<String, String>`
8. Remove `CURRENT_SCHEMA_VERSION`
9. `rebuild_index` writes `schema_hash` + `type_hashes`
10. `index_status` compares `schema_hash` (needs current hash)
11. Staleness = `commit != HEAD || schema_hash != current`

Callers:
12. `ops.rs`: `&engine.type_registry` → `&space.type_registry`
13. `RecoveryContext`: registry from `space.type_registry`

Cleanup:
14. Rewrite `docs/implementation/schema-change-detection.md` as a
    permanent implementation reference (remove migration language,
    "changes needed" section, "current vs target" framing)

### Step 9: `llm-wiki schema` CLI + `wiki_schema` MCP tool

Modules: `src/cli.rs`, `src/ops.rs`, `src/mcp/tools.rs`,
`src/mcp/handlers.rs`
Spec: `docs/specifications/tools/schema-management.md`
Tests: `schema list` returns all types with `--format`,
`schema show` returns JSON Schema, `schema show --template` returns
frontmatter template, `schema add` registers custom type and
validates, `schema remove` removes from index with `--dry-run`,
`schema validate` catches invalid schemas and field conflicts
Commit: `schema: CLI command and MCP tool for type management`

CLI subcommands:
- `llm-wiki schema list [--format text|json]`
- `llm-wiki schema show <type> [--format text|json]`
- `llm-wiki schema show <type> --template`
- `llm-wiki schema add <type> <schema-path>`
- `llm-wiki schema remove <type> [--delete] [--dry-run]`
- `llm-wiki schema validate [<type>]`

MCP tool `wiki_schema` with `action` parameter (`list`, `show`,
`add`, `remove`, `validate`) — same logic via `src/ops.rs`.

All operations target a wiki (`--wiki` or default).

Open question: did acp integration need update?


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

### Step 10bi: Integration tests

ops test integration split

### Step 11: Tantitvy Index with embbeded type documentation

Doc: 'docs/implementation/index-schema-building.md'

- Compute Tantitvy Index mapping with embedded type
- Documente how those embeded type are mapped

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

Did `graph.type` — documented but not yet implemented in `set_global_config_value`. ?


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
- Partial Rebuild - Per-type hashes are stored in `state.toml` but not compared yet. Any
`schema_hash` mismatch triggers a full rebuild.
- Hot reload / file watcher (future)
- Custom tokenizer registration (future)
- implement wiki logs 

## Related: llm-wiki-hugo-cms

A separate project that renders a wiki as a Hugo site. The wiki is the
CMS, Hugo is the renderer. See
[decisions/three-repositories.md](decisions/three-repositories.md) for
why it's a separate repo.
