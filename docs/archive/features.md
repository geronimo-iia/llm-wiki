# Features

Complete feature reference for llm-wiki. Organized by capability area.

---

## Ingest

### Direct file ingest
Ingest a single Markdown file into the wiki. Frontmatter preserved if present;
minimal frontmatter generated from H1 heading or filename if absent.
```
wiki ingest path/to/file.md [--prefix <slug>] [--update] [--dry-run]
```

### Direct folder ingest
Ingest a folder recursively. Markdown files become pages; non-Markdown files
become co-located bundle assets. Single git commit for the entire folder.
```
wiki ingest path/to/folder/ [--prefix <slug>] [--update] [--dry-run]
```

### Enrichment ingest
Apply LLM-produced enrichment JSON on top of existing pages. Merges claims,
concepts, tags, confidence, and source references into frontmatter. Writes
contradiction pages. Body never touched.
```
wiki ingest path/ --analysis enrichment.json
wiki ingest --analysis-only enrichment.json
```

### Dry run
Show what would be written without committing anything.
```
wiki ingest path/ --dry-run
```

### Bundle promotion
When a flat page gains its first co-located asset, it is automatically
promoted from `{slug}.md` to `{slug}/index.md`. Slug unchanged.

---

## Search and Retrieval

### Full-text search
BM25 search across all pages. Returns ranked results with slug, title, score.
```
wiki search "<term>" [--top <n>] [--wiki <name>]
wiki search --all "<term>"   # across all registered wikis
```

### Context lookup
Find the most relevant pages for a question. Returns ranked references
(slug, URI, path, title, score) — never page bodies.
```
wiki context "<question>" [--top-k <n>] [--wiki <name>]
```

### Page read
Fetch the full content of a single page by slug. Resolves both flat files
and bundles transparently.
```
wiki read <slug> [--body-only] [--wiki <name>]
```

### Index rebuild
Rebuild the tantivy search index from scratch.
```
wiki search --rebuild-index
```

---

## Knowledge Quality

### Lint
Structural audit: orphan pages, missing stubs, active contradictions, orphan
asset references. Writes `LINT.md` and commits it.
```
wiki lint [--wiki <name>]
```

### Contradiction list
List contradiction pages, optionally filtered by status.
```
wiki contradict [--status active|resolved|under-analysis] [--wiki <name>]
```

### Concept graph
Emit the concept graph as DOT or Mermaid. Nodes are pages; edges are
wikilinks and `related_concepts` frontmatter fields.
```
wiki graph [--format dot|mermaid] [--wiki <name>]
```

### Page list
List all pages, optionally filtered by type.
```
wiki list [--type concept|source|contradiction|query] [--wiki <name>]
```

### Diff
Show what the last ingest changed.
```
wiki diff [--wiki <name>]
```

---

## MCP Server

### Stdio transport
Start the MCP server on stdio. Default for Claude Code and local agents.
```
wiki serve
```

### SSE transport
Start the MCP server on SSE for remote multi-client access.
```
wiki serve --sse :<port>
```

### MCP tools
`wiki_ingest`, `wiki_ingest_analysis`, `wiki_context`, `wiki_read`,
`wiki_search`, `wiki_lint`, `wiki_list`, `wiki_instruct`

### MCP resources
All pages accessible at `wiki://<name>/<slug>`. Bundle assets accessible
at `wiki://<name>/<slug>/<filename>`. Resource update notifications on ingest.

### MCP prompts
`ingest_source`, `research_question`, `lint_and_enrich`, `analyse_contradiction`

---

## ACP Agent

### ACP transport
Start as a native Zed / VS Code agent. Session-oriented, streaming.
Instructions injected at session start.
```
wiki serve --acp [--wiki <name>]
```

### Workflow dispatch
Prompts are dispatched to workflows based on content or explicit `meta.workflow`:
`ingest`, `research`, `lint`, `enrichment`.

### Streaming
Every workflow step streams as a tool call or message event — visible in the
IDE in real time.

---

## Instructions

### Full guide
```
wiki instruct
```

### Topic-specific
```
wiki instruct doc-authoring    # frontmatter schema, read_when discipline
wiki instruct enrichment       # enrichment.json contract
wiki instruct ingest           # ingest workflow
wiki instruct research         # research workflow
wiki instruct lint             # lint workflow
wiki instruct contradiction    # contradiction enrichment
```

---

## Multi-Wiki

### Registry
Multiple wikis registered in `~/.wiki/config.toml`. Each has a name, path,
and optional git remote.

### Wiki targeting
All CLI commands and MCP tools accept `--wiki <name>`. Default wiki used
when omitted.
```
wiki --wiki research search "MoE"
wiki --wiki work ingest analysis.json
```

### Cross-wiki search
Fan out search across all registered wikis, merge results by relevance.
```
wiki search --all "<term>"
```

### Wiki init
Initialize a new wiki repository with default directory structure.
```
wiki init <path>
wiki init --register   # also add to ~/.wiki/config.toml
```

---

## Repository Layout

### Flat page
`{slug}.md` — page with no assets.

### Bundle page
`{slug}/index.md` + co-located assets — page with assets.

### Fixed categories
`concepts/`, `sources/`, `contradictions/`, `queries/` — created by `wiki init`,
enforced by enrichment-only ingest.

### User-defined categories
`skills/`, `guides/`, etc. — created on demand by direct ingest with `--prefix`.

### Shared assets
`assets/diagrams/`, `assets/configs/`, `assets/scripts/`, `assets/data/` —
for assets referenced by multiple pages.

### Raw sources
`raw/` — original source files, never indexed, never modified.

---

## Page Frontmatter

Every wiki page carries YAML frontmatter:

| Field | Required | Description |
|-------|----------|-------------|
| `title` | yes | Display name |
| `summary` | yes | One-line scope description |
| `read_when` | yes | Conditions under which this page is relevant |
| `status` | yes | `active`, `draft`, `deprecated`, or `stub` |
| `last_updated` | yes | ISO date |
| `type` | yes | `concept`, `source-summary`, `query-result`, or `contradiction` |
| `tags` | no | Search and cross-reference tags |
| `sources` | no | Wiki-managed — slugs of source pages |
| `confidence` | no | `high`, `medium`, or `low` |
| `contradictions` | no | Wiki-managed — slugs of contradiction pages |
| `tldr` | no | One-sentence summary |
| `claims` | no | Structured claims from enrichment |
