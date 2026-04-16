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

- Initialize a new wiki with default directory structure and git repo (`llm-wiki init`)
- Register wiki automatically in `~/.llm-wiki/config.toml` on init
- List all registered wikis (`llm-wiki spaces list`)
- Remove a wiki from the spaces, optionally deleting local files (`llm-wiki spaces remove`)
- Set the default wiki (`llm-wiki spaces set-default`)
- Multi-wiki support — one process manages all registered wikis
- Per-wiki config at `wiki.toml` in repo root, global config at `~/.llm-wiki/config.toml`
- Two-level config resolution: CLI flag → per-wiki → global → built-in default
- `llm-wiki config get/set/list` for reading and writing config

---

## Page and Section Creation

- Create a flat page with scaffolded frontmatter (`llm-wiki new page <slug>`)
- Create a bundle page with `index.md` and folder (`llm-wiki new page <slug> --bundle`)
- Create a section with `index.md` (`llm-wiki new section <slug>`)
- Auto-create missing parent sections when creating a page
- Configurable default page mode: `flat` or `bundle`

---

## Ingest

- Validate and index files already in the wiki tree (`llm-wiki ingest <path>`)
- File ingest — single Markdown file
- Folder ingest — recursive, all `.md` files and co-located assets
- Engine validates frontmatter, indexes; commits when `ingest.auto_commit` is `true`
- LLM writes directly into the wiki tree via `wiki_write` MCP tool
- Frontmatter preserved on ingest; minimal frontmatter generated if absent
- Dry run mode — validate only, no disk writes (`--dry-run`)
- Explicit commit via `llm-wiki commit` when `auto_commit` is `false`

---

## Search

- Full-text BM25 search via tantivy (`llm-wiki search "<query>"`)
- Excerpts included by default, omitted with `--no-excerpt`
- Section pages excluded by default, included with `--include-sections`
- Configurable default `--top-k`
- Cross-wiki search across all registered wikis (`--all`)
- Unified `PageRef` return type: slug, `wiki://` URI, title, score, excerpt
- All frontmatter fields indexed — any field is filterable

---

## Read

- Fetch full Markdown content of a page by slug or `wiki://` URI (`llm-wiki read`)
- Short URI form for default wiki: `wiki://<slug>`
- Strip frontmatter from output (`--no-frontmatter`)
- Configurable default `no_frontmatter`

---

## Index Management

- Explicit index rebuild from committed Markdown (`llm-wiki index rebuild`)
- Index status inspection — built date, page count, staleness (`llm-wiki index status`)
- Indexes stored in `~/.llm-wiki/indexes/<name>/` — outside the wiki repo
- Staleness detection: compare indexed commit hash in `state.toml` against `git HEAD`
- Auto-rebuild on stale index before search/list (configurable, default off)

---

## List

- Paginated enumeration of wiki pages (`llm-wiki list`)
- Filter by `type` and `status` frontmatter fields
- Offset-based pagination backed by tantivy index
- Configurable default page size

---

## Lint

- Structural audit: orphan pages, missing stubs, empty sections (`llm-wiki lint`)
- `LINT.md` written on every lint run
- `LINT.md` has no frontmatter — excluded from indexing and orphan detection
- Auto-fix missing stubs: create scaffold pages (`llm-wiki lint fix`)
- Auto-fix empty sections: create `index.md` (`llm-wiki lint fix`)
- `--only` flag to run a single fix
- Configurable auto-fix defaults per check

---

## Graph

- Concept graph from frontmatter links and body `[[links]]` (`llm-wiki graph`)
- Mermaid output (default) or DOT
- Full graph or subgraph from a root node with depth limit
- Filter by page type
- Output to stdout or file
- Output file gets minimal frontmatter with `status: generated`
- Configurable defaults: format, depth, type filter, output path

---

## Serve

- MCP server on stdio — always active (`llm-wiki serve`)
- MCP server on SSE — opt-in, multi-client (`llm-wiki serve --sse`)
- ACP agent on stdio — opt-in, streaming, session-oriented (`llm-wiki serve --acp`)
- SSE and ACP can run simultaneously alongside stdio
- All registered wikis mounted at startup — no `--wiki` flag on serve
- MCP resources namespaced by wiki name: `wiki://<name>/<slug>`
- MCP resource update notifications on every ingest
- Configurable defaults: `sse`, `sse_port`, `acp`

---

## MCP Tools

| Tool | Description |
|------|-------------|
| `wiki_write` | Write a file into the wiki tree |
| `wiki_ingest` | Validate and index files in the wiki tree |
| `wiki_search` | Full-text search, returns `Vec<PageRef>` |
| `wiki_read` | Read full content of a page by slug or URI |
| `wiki_new_page` | Create a new page with scaffolded frontmatter |
| `wiki_new_section` | Create a new section with `index.md` |
| `wiki_list` | Paginated page listing with filters |
| `wiki_lint` | Structural audit, returns `LintReport` |
| `wiki_graph` | Generate concept graph, returns `GraphReport` |
| `wiki_index_rebuild` | Rebuild tantivy index |
| `wiki_index_status` | Inspect index health |
| `wiki_index_check` | Read-only integrity check on the search index |
| `wiki_config` | Get or set config values |
| `wiki_spaces_list` | List registered llm-wiki spaces |
| `wiki_spaces_remove` | Remove a wiki space |
| `wiki_spaces_set_default` | Set the default wiki space |
| `wiki_init` | Initialize a new wiki |
| `wiki_commit` | Commit pending changes to git |

---

## Crystallize

- Instruct workflow for distilling chat sessions into wiki pages
- LLM writes complete Markdown file, ingests via `llm-wiki ingest`
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
- `--type paper` filters directly in `llm-wiki search` and `llm-wiki list`
- Lint flags source pages with missing or deprecated `source-summary` type
---

## Instructions

- Print embedded workflow instructions (`llm-wiki instruct`)
- Per-workflow instructions: `help`, `ingest`, `research`, `lint`, `crystallize`, `frontmatter`
- Session orientation preamble: search + read hub pages before any workflow
- Linking policy preamble: quality test for all link additions
- Frontmatter authoring guide: per-field, per-type reference for LLM-produced values
- Instructions injected at MCP server start and ACP `initialize`
- Binary is the single source of truth — plugin files delegate to `llm-wiki instruct`

---

## Repository Layout

- Flat page: `{slug}.md`
- Bundle page: `{slug}/index.md` + co-located assets
- Folder structure defined by `schema.md` — no engine-enforced categories
- Default `schema.md` suggests `concepts/`, `sources/`, `queries/` as conventions
- Epistemic distinctions carried by `type` field, not by folder
- User-defined sections created on demand via `llm-wiki new section`
- `LINT.md` at repository root — written by `llm-wiki lint`
- Indexes stored in `~/.llm-wiki/indexes/<name>/` — outside the repo

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
