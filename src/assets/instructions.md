# llm-wiki Instructions

## Session orientation

At the start of any workflow, orient yourself to the wiki's current state:

1. Read `schema.md` if not already in context (injected automatically via MCP).
2. Search for the topic: `wiki_search("<topic>")`.
3. Read the most relevant hub page or section index before making changes.

The wiki is the persistent context across sessions. Start from what it knows.

## Linking policy

When adding links — frontmatter (`sources`, `concepts`) or body (`[[wikilinks]]`) — apply this test: would a reader of this page benefit from navigating to the linked page? If the connection is only surface-level (shared keyword, same broad domain), omit the link. Prefer fewer meaningful links over many weak ones.

## help

Available tools:

- `wiki_search(query)` — search the wiki, returns ranked results
- `wiki_read(slug)` — read a page by slug
- `wiki_list(type?, status?)` — list pages with optional filters
- `wiki_write(path, content)` — write a file into the wiki tree
- `wiki_ingest(path, dry_run?)` — validate, commit, and index
- `wiki_new_page(uri, bundle?)` — create a scaffolded page
- `wiki_new_section(uri)` — create a section index
- `wiki_lint()` — run structural lint checks
- `wiki_graph(root?, depth?, format?)` — render the knowledge graph
- `wiki_config(action, key?, value?)` — get/set configuration
- `wiki_index_rebuild()` — rebuild the search index
- `wiki_index_status()` — check if the index is stale
- `wiki_index_check()` — run read-only integrity check

Workflows: `new`, `ingest`, `research`, `lint`, `crystallize`.

## new

Create a page or section in the wiki.

1. Orient: `wiki_search("<topic>")` to check if the page already exists.
2. Create: `wiki_new_page("wiki://<slug>")` or `wiki_new_section("wiki://<slug>")`.
3. Edit: `wiki_read(<slug>)` the scaffold, write full content via `wiki_write`.
4. Commit: `wiki_ingest(<path>)`.

Use `--bundle` for pages with co-located assets.

## ingest

Validate and commit files already in the wiki tree.

1. Orient: `wiki_search("<topic>")` to find related existing pages.
2. Read existing page if updating: `wiki_read(<slug>)`.
3. Write the complete file: `wiki_write(<path>, <content>)`.
   - Preserve existing list values (`tags`, `read_when`, `sources`, `concepts`, `claims`).
   - Set all required frontmatter fields (title, summary, read_when, status, type, last_updated).
   - Choose the correct `type` from the taxonomy (see `## frontmatter`).
4. Commit: `wiki_ingest(<path>)`.
5. Review warnings in the `IngestReport`.

Accumulation contract: read before writing. Never drop existing list values.

## research

Search the wiki and synthesize an answer.

1. Search: `wiki_search("<question>")`.
2. Read top results: `wiki_read(<slug>)` for each relevant hit.
3. Synthesize: answer the question using wiki knowledge.
4. Cite sources: reference wiki pages by slug.

If the answer reveals a gap, suggest creating a new page via the `new` workflow.

## lint

Run structural checks and fix issues.

1. Run: `wiki_lint()` to get the `LintReport`.
2. Review each section: orphans, missing stubs, empty sections, missing connections, untyped sources.
3. Fix: address findings — add links to orphans, create stubs, set proper types.
4. Commit fixes: `wiki_ingest(<path>)` for each changed file.

Use `wiki lint fix` to auto-create missing stubs and empty section indexes.

## crystallize

Distil a session's insights into wiki pages.

1. Orient: `wiki_search("<topic>")` to find the existing home for this knowledge.
2. Read: `wiki_read(<candidate hub page>)` to understand current state.
3. Decide: update an existing page or create a new one.
4. Write: complete file with full frontmatter. Preserve existing values if updating.
5. Link: add `sources` and `concepts` slugs. Apply the linking policy test.
6. Commit: `wiki_ingest(<path>)`.

Crystallize after every substantive session. The wiki is the accumulator.

## index troubleshooting

If search returns no results or unexpected errors:

1. `wiki_index_check()` — diagnose: is the index openable? queryable? schema current?
2. `wiki_index_status()` — check staleness: has the git HEAD moved since last rebuild?
3. `wiki_index_rebuild()` — fix: full rebuild from wiki markdown

The index auto-recovers from corruption when `index.auto_recovery` is enabled
(default). Stale indexes are rebuilt automatically when `index.auto_rebuild`
is enabled. If both are disabled, use the tools above to diagnose and fix.

## frontmatter

### Required fields

```yaml
---
title: "Page Title"
summary: "One-sentence scope description."
read_when:
  - "Retrieval condition 1"
  - "Retrieval condition 2"
status: active
last_updated: "2025-07-15"
type: concept
---
```

### Type taxonomy

Knowledge types:

| Type | Role |
|------|------|
| `concept` | Synthesized knowledge, one concept per page |
| `query-result` | Saved Q&A, crystallized sessions |
| `section` | Section index page |

Source types (one page per source document):

| Type | Nature |
|------|--------|
| `paper` | Academic |
| `article` | Editorial (blogs, news, essays) |
| `documentation` | Reference (API docs, specs) |
| `clipping` | Web capture |
| `transcript` | Spoken (meetings, podcasts) |
| `note` | Informal drafts |
| `data` | Structured datasets |
| `book-chapter` | Published excerpts |
| `thread` | Discussion archives |

Classify by source nature, not topic. A blog about research → `article`, not `paper`.

### Per-type templates

Concept:
```yaml
type: concept
tags: [topic-a, topic-b]
sources: [sources/source-slug]
concepts: [concepts/related-slug]
confidence: high
```

Source (paper, article, etc.):
```yaml
type: paper
tags: [topic]
concepts: [concepts/related-slug]
confidence: high
claims:
  - text: "Factual claim from source"
    confidence: high
    section: "Results"
```

Query result:
```yaml
type: query-result
tags: [topic]
sources: [sources/source-slug]
concepts: [concepts/related-slug]
confidence: medium
```

### Update rules

1. `wiki_read(<slug>)` before writing.
2. Preserve existing list values — never drop tags, sources, concepts, claims.
3. Add new values to lists.
4. Update scalars (summary, tldr, confidence) only with clear reason.

### Common mistakes

| Mistake | Fix |
|---------|-----|
| Missing `title` | Engine rejects. Always include |
| `source-summary` type | Use specific type: `paper`, `article`, etc. |
| Missing `read_when` | Always include 2–5 retrieval conditions |
| Dropping existing values on update | Read first, preserve existing |
| `confidence: high` without evidence | Default to `medium` |
| Classifying by topic not source nature | Blog about research → `article` |
