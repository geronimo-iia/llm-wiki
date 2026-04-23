# Page body templates

## Decision

Body templates for `wiki_content_new` live alongside schemas using a
naming convention: `schemas/<type>.md` next to `schemas/<type>.json`.

## Context

`wiki_content_new` scaffolded frontmatter but left the body empty.
Authors had to write body structure from scratch every time.

## Key decisions

- **Naming convention** — `schemas/<type>.md` next to the JSON Schema.
  No new directory. Custom types get templates for free.
- **Embedded defaults** — `llm-wiki spaces create` ships `.md`
  templates for concept, paper, doc, section, query-result.
- **Fallback chain** — wiki template → embedded → empty body.
- **Watcher ignores `.md` in schemas/** — only `.json` triggers
  index operations.
- **No flag** — templates are always used when available.

## Consequences

- New pages have consistent body structure out of the box
- Wiki owners can customize templates per type
- Custom types get body templates by adding one `.md` file
- No index or schema impact — templates are a content scaffolding
  feature only
