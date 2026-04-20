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

### Skills (llm-wiki-skills)

- [X] Update `frontmatter` skill with type-specific guidance
- [X] Update `bootstrap` skill to read types from `wiki_config`
- [X] Update `ingest` skill to reference type validation
- [X] Update `write-page` skill to use `wiki_schema show --template`

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
