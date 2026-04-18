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

## Phase 1 — Focused Engine

Fresh implementation from the specifications. The current codebase
(`src/`, `tests/`) moves to `code-ref/` as reference material. New
`src/` built against the specs, pulling implementation patterns and
complete modules from `code-ref/` where they still apply.

### Codebase reset

- [ ] Move `src/` and `tests/` to `code-ref/`
- [ ] Start fresh `src/` from specifications
- [ ] Reusable as-is from `code-ref/`: tantivy index operations, git2
  wrappers, comrak parsing, MCP/ACP server wiring
- [ ] Needs rewriting: tool names, frontmatter validation, ingest
  pipeline, CLI command structure

### Engine (llm-wiki)

- [ ] Remove `llm-wiki instruct` from the binary
- [ ] Remove `llm-wiki lint` CLI command (moves to skill)
- [ ] Implement the 15 MCP/ACP tools:
  - Spaces: `wiki_spaces_create`, `wiki_spaces_list`,
    `wiki_spaces_remove`, `wiki_spaces_set_default`
  - Config: `wiki_config`
  - Content: `wiki_content_read`, `wiki_content_write`,
    `wiki_content_new`, `wiki_content_commit`
  - Search & index: `wiki_search`, `wiki_list`, `wiki_ingest`,
    `wiki_graph`, `wiki_index_rebuild`, `wiki_index_status`
- [ ] Add `--type` filter to `wiki_search`
- [ ] Add `--format text|json` to search, list, spaces list, config list,
  index status, index rebuild, ingest
- [ ] `wiki_content_read` surfaces supersession notice when
  `superseded_by` is set
- [ ] Fold `wiki_index_check` into `wiki_index_status`
- [ ] `llm-wiki content write` CLI command (stdin or `--file`)

### Skills (llm-wiki-skills)

- [ ] Create the `llm-wiki-skills` git repository
- [ ] Set up Claude Code plugin structure
- [ ] Write the 8 initial skills:
  - `bootstrap` — session orientation
  - `ingest` — source processing workflow
  - `crystallize` — distil session into wiki pages
  - `research` — search, read, synthesize
  - `lint` — structural audit + fix
  - `graph` — generate and interpret concept graph
  - `frontmatter` — frontmatter authoring reference
  - `skill` — find and activate wiki skills
- [ ] Test with `claude --plugin-dir ./llm-wiki-skills`

### Milestone

Engine binary with 15 tools. Skills repo with 8 skills. Claude Code
plugin installable. `llm-wiki serve` + plugin = working system.

## Phase 2 — Type System

JSON Schema validation per type. Type registry in `wiki.toml`.
`schema.md` eliminated.

### Engine (llm-wiki)

- [ ] Add `[types.*]` section to `wiki.toml`
- [ ] Add `schemas/` directory to wiki repo layout
- [ ] Ship default JSON Schema files:
  - `base.json` — required: `title`, `type`
  - `concept.json` — extends base, adds `read_when`, `sources`,
    `concepts`, `confidence`, `claims`
  - `paper.json` — extends base, adds `read_when`, `sources`,
    `concepts`, `confidence`, `claims`
  - `skill.json` — standalone, uses `x-index-aliases`
  - `doc.json` — extends base, adds `read_when`, `sources`
  - `section.json` — extends base
- [ ] Implement JSON Schema validation on `wiki_ingest`
- [ ] Implement `x-index-aliases` — resolve field aliases at ingest
- [ ] `llm-wiki spaces create` generates default `wiki.toml` with
  `[types.*]` entries and `schemas/` directory
- [ ] `wiki_config list` returns type names + descriptions
- [ ] Schema change detection via `schema_hash` in `state.toml`
- [ ] Per-type hashes for partial rebuild

### Skills (llm-wiki-skills)

- [ ] Update `frontmatter` skill with type-specific guidance
- [ ] Update `bootstrap` skill to read types from `wiki_config`
- [ ] Update `ingest` skill to reference type validation

### Milestone

Type-specific JSON Schema validation on ingest. Field aliasing for
skill and doc pages. Custom types addable via `wiki.toml` + schema file.

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
