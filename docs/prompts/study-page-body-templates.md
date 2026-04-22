# Study: Page body templates via wiki_content_new

Extend `wiki_content_new` to scaffold a full page body (not just
frontmatter) based on the page type. Speeds up page creation for
both LLMs and humans.

## Problem

`wiki_content_new` creates a page with scaffolded frontmatter but an
empty body. `wiki_schema show --template` returns frontmatter only.
The author must write the body structure from scratch every time.

For common types (concept, paper, section), the body follows a
predictable pattern. Scaffolding it saves time and ensures
consistency.

## Proposed behavior

`wiki_content_new` gains a `--template` flag (or templates are
always included):

```
llm-wiki content new concepts/new-concept --type concept
```

Creates:

```markdown
---
title: "New Concept"
type: concept
status: draft
last_updated: "2025-07-22"
read_when:
  - ""
summary: ""
tags: []
---

## Overview



## Key ideas



## Related concepts



## Open questions

```

### Template per type

| Type | Body sections |
|------|--------------|
| `concept` | Overview, Key ideas, Related concepts, Open questions |
| `paper` | Summary, Key claims, Methodology, Limitations |
| `article` | Summary, Key points, Context |
| `query-result` | Summary, Decisions, Findings, Open questions |
| `section` | (list of child pages — auto-generated or placeholder) |
| `doc` | Overview, Details, References |
| `skill` | (body is the workflow — no template, too varied) |

### Where templates live

Two options:

1. **Embedded in the engine** — hardcoded per built-in type, custom
   types get frontmatter only.
2. **In schema files** — add `x-body-template` extension to JSON
   Schema, alongside `x-wiki-types` and `x-graph-edges`.

Option 2 is more flexible — custom types can define their own body
templates. But it adds complexity to the schema format.

Recommendation: start with option 1 (embedded), add option 2 later
if custom types need body templates.

## Interaction with existing features

- `wiki_schema show --template` — returns frontmatter only (unchanged)
- `wiki_content_new` — now also scaffolds body
- Ingest skill — uses `content_new` then overwrites body with
  synthesized content (template is a starting point)
- Write-page / content skill — template gives the LLM a structure
  to fill in

## Open questions

- Should templates always be included, or opt-in with `--template`?
- Should the LLM be able to customize templates per wiki (option 2)?
- Should section templates auto-list child pages?

## Tasks

- [ ] Spec: update `docs/specifications/tools/content-operations.md`
- [ ] `src/ops/content.rs` — add body template generation per type
- [ ] Embedded templates for built-in types
- [ ] Tests
- [ ] Update skills (content, ingest)
- [ ] Decision record, changelog, roadmap
