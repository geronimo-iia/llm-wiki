---
title: "Features"
summary: "Complete feature list for llm-wiki, organized by capability area."
read_when:
  - Getting a full picture of what llm-wiki supports
  - Checking whether a specific capability is planned or implemented
  - Onboarding a new contributor
status: active
last_updated: "2025-07-15"
---

# Features

Complete feature list organized by capability area. Implementation status is
tracked per-feature in the individual specification docs.

---

## Wiki Management

- Initialize a new wiki with default directory structure and git repo (`wiki init`)
- Register wiki automatically in `~/.wiki/config.toml` on init
- List all registered wikis (`wiki registry list`)
- Remove a wiki from the registry, optionally deleting local files (`wiki registry remove`)
- Set the default wiki (`wiki registry set-default`)
- Multi-wiki support — one process manages all registered wikis
- Per-wiki config at `.wiki/config.toml`, global config at `~/.wiki/config.toml`
- Two-level config resolution: CLI flag → per-wiki → global → built-in default
- `wiki config get/set/list` for reading and writing config

---

## Page and Section Creation

- Create a flat page with scaffolded frontmatter (`wiki new page <slug>`)
- Create a bundle page with `index.md` and folder (`wiki new page <slug> --bundle`)
- Create a section with `index.md` (`wiki new section <slug>`)
- Auto-create missing parent sections when creating a page
- Configurable default page mode: `flat` or `bundle`

---

## Ingest

- File ingest — Markdown with frontmatter, authored by human or LLM (`wiki ingest <file>`)
- Folder ingest — recursive, assets co-located (`wiki ingest <folder>`)
- Engine validates frontmatter, places files, commits, indexes
- LLM writes complete Markdown files — no JSON intermediary
- Update mode — overwrite existing page (`--update`)
- Frontmatter preserved on ingest; minimal frontmatter generated if absent
- Bundle promotion — flat page auto-promoted to bundle when first asset co-located
- Co-located assets — non-Markdown files stay beside their page
- Shared assets — assets referenced by multiple pages go to `assets/`
- Dry run mode — show what would be written without committing (`--dry-run`)
- All ingests produce a git commit

---

## Search

- Full-text BM25 search via tantivy (`wiki search "<query>"`)
- Excerpts included by default, omitted with `--no-excerpt`
- Section pages excluded by default, included with `--include-sections`
- Configurable default `--top-k`
- Cross-wiki search across all registered wikis (`--all`)
- Unified `PageRef` return type: slug, `wiki://` URI, title, score, excerpt
- All frontmatter fields indexed — any field is filterable

---

## Read

- Fetch full Markdown content of a page by slug or `wiki://` URI (`wiki read`)
- Short URI form for default wiki: `wiki://<slug>`
- Strip frontmatter from output (`--no-frontmatter`)
- Configurable default `no_frontmatter`

---

## Index Management

- Explicit index rebuild from committed Markdown (`wiki index rebuild`)
- Index status inspection — built date, page count, staleness (`wiki index status`)
- Staleness detection via `index-status.toml` committed to git (compares git HEAD)
- Auto-rebuild on stale index before search/list (configurable, default off)
- `index-status.toml` committed on every rebuild; `.wiki/search-index/` gitignored

---

## List

- Paginated enumeration of wiki pages (`wiki list`)
- Filter by `type` and `status` frontmatter fields
- Offset-based pagination backed by tantivy index
- Configurable default page size

---

## Lint

- Structural audit: orphan pages, missing stubs, empty sections (`wiki lint`)
- `LINT.md` written and committed on every lint run
- `LINT.md` has no frontmatter — excluded from indexing and orphan detection
- Auto-fix missing stubs: create scaffold pages (`wiki lint fix`)
- Auto-fix empty sections: create `index.md` (`wiki lint fix`)
- `--only` flag to run a single fix
- Configurable auto-fix defaults per check

---

## Graph

- Concept graph from frontmatter links and body `[[links]]` (`wiki graph`)
- Mermaid output (default) or DOT
- Full graph or subgraph from a root node with depth limit
- Filter by page type
- Output to stdout or file; auto-commit if file is inside wiki root
- Output file gets minimal frontmatter with `status: generated`
- Configurable defaults: format, depth, type filter, output path

---

## Serve

- MCP server on stdio — always active (`wiki serve`)
- MCP server on SSE — opt-in, multi-client (`wiki serve --sse`)
- ACP agent on stdio — opt-in, streaming, session-oriented (`wiki serve --acp`)
- SSE and ACP can run simultaneously alongside stdio
- All registered wikis mounted at startup — no `--wiki` flag on serve
- MCP resources namespaced by wiki name: `wiki://<name>/<slug>`
- MCP resource update notifications on every ingest
- Configurable defaults: `sse`, `sse_port`, `acp`

---

## MCP Tools

| Tool | Description |
|------|-------------|
| `wiki_ingest` | Ingest a Markdown file or folder into the wiki |
| `wiki_search` | Full-text search, returns `Vec<PageRef>` |
| `wiki_read` | Read full content of a page by slug or URI |
| `wiki_new_page` | Create a new page with scaffolded frontmatter |
| `wiki_new_section` | Create a new section with `index.md` |
| `wiki_list` | Paginated page listing with filters |
| `wiki_lint` | Structural audit, returns `LintReport` |
| `wiki_graph` | Generate concept graph, returns `GraphReport` |
| `wiki_index_rebuild` | Rebuild tantivy index |
| `wiki_index_status` | Inspect index health |
| `wiki_config` | Get or set config values |
| `wiki_registry_list` | List registered wikis |
| `wiki_registry_remove` | Remove a wiki from the registry |
| `wiki_registry_set_default` | Set the default wiki |
| `wiki_init` | Initialize a new wiki |

---

## Crystallize

- Instruct workflow for distilling chat sessions into wiki pages
- LLM writes complete Markdown file, ingests via `wiki ingest`
- Guides the LLM on what to extract (decisions, findings, open questions)
- Prefers updating existing hub pages over creating new orphans
- Suggested body structure: Summary, Decisions, Findings, Open Questions
- Slash command: `/llm-wiki:crystallize`

---

## Session Bootstrap

- Three-layer bootstrap: instructions → schema.md → hub page orientation
- `schema.md` injected alongside instructions at MCP/ACP session start
- Every instruct workflow begins with an orientation step (search + read hub pages)
- Crystallize feeds back into bootstrap — each session enriches the next

---

## Backlink Quality

- Linking policy: add links only when a reader would genuinely benefit
- Graph density is not the goal — prefer fewer meaningful links
- Lint detects missing connection candidates (significant term overlap, no mutual links)
- `MissingConnection` in `LintReport` with overlapping terms

---

## Source Classification

- Source types folded into the `type` field: `paper`, `article`, `documentation`, `clipping`, `transcript`, `note`, `data`, `book-chapter`, `thread`
- No separate `classification` field — `type` is the single axis
- Custom types defined in `schema.md`, validated by engine on ingest
- `--type paper` filters directly in `wiki search` and `wiki list`
- Lint flags source pages with missing or deprecated `source-summary` type

---

## Instructions

- Print embedded workflow instructions (`wiki instruct`)
- Per-workflow instructions: `help`, `ingest`, `research`, `lint`, `crystallize`, `frontmatter`
- Session orientation preamble: search + read hub pages before any workflow
- Linking policy preamble: quality test for all link additions
- Frontmatter authoring guide: per-field, per-type reference for LLM-produced values
- Instructions injected at MCP server start and ACP `initialize`
- Binary is the single source of truth — plugin files delegate to `wiki instruct`

---

## Repository Layout

- Flat page: `{slug}.md`
- Bundle page: `{slug}/index.md` + co-located assets
- Four fixed categories: `concepts/`, `sources/`, `queries/`, `raw/`
- User-defined sections created on demand via direct ingest or `wiki new section`
- Shared assets: `assets/diagrams/`, `assets/configs/`, `assets/scripts/`, `assets/data/`
- `LINT.md` at wiki root — committed by `wiki lint`
- `.wiki/index-status.toml` — committed by `wiki index rebuild`
- `.wiki/search-index/` — gitignored, rebuilt locally

---

## Page Frontmatter

Every wiki page carries YAML frontmatter. The author (human or LLM) writes
frontmatter directly in the Markdown file. The engine validates on ingest.
See [frontmatter-authoring.md](specifications/frontmatter-authoring.md).

| Field | Required | Description |
|-------|----------|-------------|
| `title` | yes | Display name |
| `summary` | yes | One-line scope description |
| `read_when` | yes | Conditions under which this page is relevant |
| `status` | yes | `active`, `draft`, `stub`, or `generated` |
| `last_updated` | yes | ISO date |
| `type` | yes | `concept`, `paper`, `article`, `documentation`, `clipping`, `transcript`, `note`, `data`, `book-chapter`, `thread`, `query-result`, `section`, or custom |
| `tags` | no | Search and cross-reference tags |
| `sources` | no | Slugs of source pages |
| `confidence` | no | `high`, `medium`, or `low` |
| `claims` | no | Structured claims from enrichment |
