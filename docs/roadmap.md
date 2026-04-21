---
title: "Roadmap"
summary: "Development roadmap for llm-wiki — from focused engine to skill registry."
status: ready
last_updated: "2025-07-20"
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

Fresh implementation from the specifications. 354 integration tests,
16 MCP tools, ACP agent, stdio + SSE transport. Single Rust binary,
no runtime dependencies.

- tantivy 0.25 for full-text search
- `WikiEngine` / `EngineState` architecture with `mount_wiki` per space
- Interior mutability in `SpaceIndexManager` (`RwLock<IndexInner>`)
- Graceful shutdown via `watch` channel + `AtomicBool`
- `_slug_ord` u64 FAST field for sorted list pagination
- Reusable ACP workflow steps (`step_search`, `step_read`, `step_report_results`)
- Per-wiki `IndexSchema` in cross-wiki search

### Skills (llm-wiki-skills) ✓

- [x] Create the `llm-wiki-skills` git repository
- [x] Set up Claude Code plugin structure
- [x] Write the 11 initial skills
- [ ] Test with `claude --plugin-dir ./llm-wiki-skills`

### Milestone ✓

Engine binary with 16 tools. Skills repo with 11 skills. Claude Code
plugin installable. `llm-wiki serve` + plugin = working system.

## Phase 2 — Type System ✓

JSON Schema validation per type. Type registry discovered from
`schemas/*.json` via `x-wiki-types`. `schema.md` eliminated.

- [x] JSON Schema validation on ingest (`jsonschema` crate)
- [x] Schema discovery from `schemas/*.json` via `x-wiki-types`
- [x] `wiki.toml` `[types.*]` overrides
- [x] Field aliasing via `x-index-aliases`
- [x] `wiki_schema` tool (list/show/add/remove/validate)
- [x] Frontmatter template generation (`--template`)
- [x] Schema change detection (per-type hashes, shared `hash_type_entries`)
- [x] Embedded default schemas via `include_str!()`
- [x] Base schema invariant enforcement

### Skills (llm-wiki-skills) ✓

- [x] Update `frontmatter` skill with type-specific guidance
- [x] Update `bootstrap` skill to read types from `wiki_config`
- [x] Update `ingest` skill to reference type validation
- [x] Update `write-page` skill to use `wiki_schema show --template`

### Milestone ✓

Type-specific JSON Schema validation on ingest. Field aliasing for
skill and doc pages. Schema introspection via CLI and MCP. Custom
types addable via `wiki.toml` + schema file.

## Phase 3 — Typed Graph ✓

`x-graph-edges` in type schemas. Typed nodes and labeled edges.
`wiki_graph` filters by relation.

### Skills (llm-wiki-skills)

- [ ] Update `graph` skill with relation-aware instructions
- [ ] Update `lint` skill to detect type constraint violations

### Milestone ✓

Labeled graph edges from frontmatter fields. Relation-filtered graph
output. Type constraint warnings on ingest.

## Phase 4 — Skill Registry

The wiki becomes a full skill registry.

### Engine (llm-wiki)

- [ ] Verify `wiki_search --type skill` works end-to-end with
  `x-index-aliases`
- [ ] Verify `wiki_list --type skill` returns skill-specific metadata
- [ ] Verify `wiki_graph` renders skill edges correctly
- [ ] Cross-wiki skill discovery: `wiki_search --type skill --cross-wiki`

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
- Hot reload — add/remove wikis without restart
- Custom tokenizer registration
- ACP workflows beyond `research` (ingest, explore, summarize)

## Related: llm-wiki-hugo-cms

A separate project that renders a wiki as a Hugo site. The wiki is the
CMS, Hugo is the renderer. See
[decisions/three-repositories.md](decisions/three-repositories.md) for
why it's a separate repo.
