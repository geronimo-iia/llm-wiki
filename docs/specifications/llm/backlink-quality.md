---
title: "Backlink Quality"
summary: "Linking policy for wiki pages — when to add a link, when not to, and how lint evaluates connection quality."
read_when:
  - Writing or reviewing instruct workflows that add links between pages
  - Understanding the linking philosophy
  - Implementing missing-connection detection in lint
status: draft
last_updated: "2025-07-15"
---

# Backlink Quality

Links between wiki pages are the knowledge graph's edges. Their quality
determines whether the graph is navigable or noisy. This document defines
the linking policy enforced through instruct workflows and evaluated by lint.

---

## 1. The Principle

Add a link when a reader of page A would genuinely benefit from knowing about
page B — not because they share a keyword, but because one informs the other.

Graph density is not the goal. A sparse graph with meaningful edges is more
valuable than a dense graph with noise.

---

## 2. The Test

Before adding a `[[link]]` or a slug to `sources`/`concepts` frontmatter,
apply this test:

> Would a reader of this page, in the course of normal reading, benefit from
> navigating to the linked page? Does the linked page add context, evidence,
> a counterpoint, or a prerequisite that makes this page more useful?

If yes → add the link.
If the connection is only surface-level (shared keyword, same broad domain,
tangential mention) → omit it.

---

## 3. Link Types and When to Use Them

| Link type | Mechanism | When to use |
|-----------|-----------|-------------|
| Frontmatter `sources` | Slug array in YAML | This page's claims originate from that source |
| Frontmatter `concepts` | Slug array in YAML | This page directly discusses or depends on that concept |
| Body `[[wikilink]]` | Inline Markdown link | A specific passage references or builds on another page |

### Frontmatter links

Frontmatter links are structural — they define the knowledge graph edges that
`wiki graph` and `wiki lint` traverse. They should be precise:

- `sources` — only pages that actually contributed claims to this page
- `concepts` — only concepts this page directly discusses, not every concept
  mentioned in passing

### Body links

Body links are contextual — they help the reader navigate while reading. They
should be natural:

- Link on first meaningful mention, not every mention
- Link where the reader would want to drill deeper
- Do not link common terms that happen to have wiki pages

---

## 4. Anti-Patterns

| Anti-pattern | Why it's harmful |
|--------------|------------------|
| Linking every shared keyword | Noise — reader cannot distinguish meaningful from trivial connections |
| Linking for graph density | Orphan count drops but navigation quality drops too |
| Reciprocal links by default | Page A linking to B does not mean B should link to A — evaluate each direction independently |
| Linking to unrelated pages in the same section | Proximity is not relevance |

---

## 5. Instruct Integration

The linking policy is referenced in the ingest and crystallize instruct
workflows. Addition to `src/instructions.md` preamble:

```markdown
## Linking policy

When adding links between pages — in frontmatter (`sources`, `concepts`) or
body (`[[wikilinks]]`) — apply this test: would a reader of this page benefit
from navigating to the linked page? If the connection is only surface-level
(shared keyword, same broad domain), omit the link. Prefer fewer meaningful
links over many weak ones.
```

---

## 6. Lint Integration

`wiki lint` detects orphan pages, missing stubs, empty sections, missing
connection candidates, and untyped sources. See
[lint.md](../commands/lint.md) for the full `LintReport` definition.
