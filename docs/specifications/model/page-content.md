---
title: "Page Content"
summary: "Anatomy of a wiki page — flat vs bundle, slug resolution, body conventions, section index pages."
read_when:
  - Understanding the structure of a wiki page
  - Understanding how slugs map to disk paths
  - Writing page body content
status: ready
last_updated: "2025-07-17"
---

# Page Content

A wiki page is a Markdown file with YAML frontmatter. The author (human
or LLM) writes the complete file directly in the wiki tree. The engine
validates frontmatter on ingest but does not modify body content.

For field definitions, see [types/base.md](types/base.md) and the
individual type docs under [types/](types/).

## Anatomy of a Page

```
concepts/mixture-of-experts.md
─────────────────────────────────────────────────
---                                 ← frontmatter open
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
type: concept
status: active
...
---                                 ← frontmatter close
                                    ← blank line (required)
## Overview                         ← body starts here

MoE routes tokens to sparse expert subnetworks…
```

The file is always: `frontmatter block` + `blank line` + `body`.

## Flat Page vs Bundle

**Flat page** — a single `.md` file, no co-located assets:

```
concepts/scaling-laws.md
```

**Bundle page** — a folder containing `index.md` and assets beside it:

```
concepts/mixture-of-experts/
├── index.md
├── moe-routing.png
└── vllm-config.yaml
```

Assets are referenced with short relative paths:

```markdown
![MoE routing](./moe-routing.png)
See [vllm-config.yaml](./vllm-config.yaml)
```

Assets always belong to one page — there is no shared asset folder.

## Slug Resolution

A slug is a path without extension, relative to `wiki/`. Resolution
checks two forms in order:

```
slug: concepts/mixture-of-experts

1. concepts/mixture-of-experts.md        → flat file
2. concepts/mixture-of-experts/index.md  → bundle
```

The author always uses the same slug regardless of which form is on
disk.

## Body Conventions

CommonMark + GFM (GitHub Flavored Markdown). Parsed by comrak.

### Wiki Links

`[[slug]]` links to another wiki page:

```markdown
See [[concepts/scaling-laws]] for background.
```

Wiki links create graph edges (generic `links-to` relation). See
[graph.md](../engine/graph.md).

**Extraction rules:**

- Pattern: `[[` + content + `]]`
- Content is trimmed of leading/trailing whitespace
- Empty content is ignored
- No escaping — `\[[` is not supported
- No link-with-title — `[[slug|title]]` is not supported
- Nested brackets are not handled — first `]]` closes the link
- Deduplicated per page — each slug appears once in the link list
- Extracted from body text only (frontmatter slug-list fields like
  `sources` and `concepts` are handled separately)

### Headings

Start body content at `##` level. `#` is reserved for the page title
(derived from frontmatter `title`).

## Section Index Pages

A section is a directory with an `index.md` that groups related pages.
Section pages use `type: section`.

```
wiki/concepts/
├── index.md                    ← section index (type: section)
├── scaling-laws.md
└── mixture-of-experts.md
```

Section index pages are excluded from search results by default
(`--include-sections` to include them). They serve as navigation, not
knowledge.
