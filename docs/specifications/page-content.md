---
title: "Page Content"
summary: "Anatomy of a wiki page — frontmatter schema, required fields, per-type conventions, and what the engine validates on ingest."
read_when:
  - Understanding the structure of a wiki page
  - Implementing frontmatter validation in markdown.rs
  - Writing the frontmatter authoring guide for LLMs
  - Deciding what fields are required vs optional
status: active
last_updated: "2025-07-15"
---

# Page Content

A wiki page is a Markdown file with YAML frontmatter. The author (human or
LLM) writes the complete file — frontmatter and body. The engine validates
the frontmatter on ingest but does not assemble or modify it (except
`last_updated` and defaults for missing required fields).

---

## 1. Anatomy of a Wiki Page

```
concepts/mixture-of-experts.md
─────────────────────────────────────────────────
---                                 ← frontmatter open
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
tldr: "MoE reduces compute 8x at pre-training scale."
read_when:
  - "Reasoning about MoE architecture"
status: active
last_updated: "2025-07-15"
type: concept
tags: [transformers, scaling]
sources: [sources/switch-transformer-2021]
concepts: [concepts/scaling-laws]
confidence: high
claims:
  - text: "Sparse MoE reduces effective compute 8x"
    confidence: high
    section: "Results"
---                                 ← frontmatter close
                                    ← blank line (required)
## Overview                         ← body starts here

MoE routes tokens to sparse expert subnetworks…
```

The file is always: `frontmatter block` + `blank line` + `body`.

---

## 2. Frontmatter Schema

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Display name |
| `summary` | string | One-line scope description |
| `status` | string | `active`, `draft`, `stub`, or `generated` |
| `last_updated` | string | ISO 8601 date |
| `type` | string | Page type from unified taxonomy (see [frontmatter-authoring.md](frontmatter-authoring.md) § 2) |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `tldr` | string | One-sentence key takeaway |
| `read_when` | list[string] | Retrieval conditions for LLM search |
| `tags` | list[string] | Search and cross-reference terms |
| `sources` | list[string] | Slugs of source pages |
| `concepts` | list[string] | Slugs of related concept pages |
| `confidence` | string | `high`, `medium`, or `low` |
| `claims` | list[object] | Structured claims: `{ text, confidence, section }` |

---

## 3. Engine Behavior on Ingest

### What the engine validates

| Check | On failure |
|-------|------------|
| Valid YAML frontmatter block present | Error — file rejected |
| `title` present and non-empty | Error — file rejected |
| `type` recognized | Warning — defaults to `page` |
| `status` recognized | Warning — defaults to `active` |


### What the engine sets automatically

| Field | Rule |
|-------|------|
| `last_updated` | Always set to today on ingest |
| `status` | Set to `active` if absent |
| `type` | Inferred from slug prefix if absent (`concepts/` → `concept`, etc.) |

### What the engine never touches

Everything else. The file the author writes is the file that lands in the
wiki. The engine does not merge, rewrite, or reformat frontmatter or body.

---

## 4. Files Without Frontmatter

When `wiki ingest` processes a file with no frontmatter block, minimal
frontmatter is generated:

| Field | Value |
|-------|-------|
| `title` | First H1 heading in body, or filename stem if no H1 |
| `summary` | `""` (empty) |
| `status` | `active` |
| `last_updated` | Today's ISO 8601 date |
| `type` | Inferred from slug prefix, or `page` as default |
| `tags` | `[]` |

The body is preserved exactly as found — no reformatting.

---

## 5. Update Responsibility

When updating an existing page, the author (human or LLM) is responsible
for the complete file. There are no automatic merge rules.

**Accumulation contract:** list fields (`tags`, `read_when`, `sources`,
`concepts`, `claims`) accumulate knowledge from multiple ingests. When
updating a page, the author must preserve existing values and add new ones.
Silently dropping values added by prior ingests breaks search and
provenance tracking.

The instruct workflow enforces this by requiring the LLM to `wiki_read`
the existing page before writing an update.

---

## 6. Parsing Rules

`integrate` splits an existing `.md` file into frontmatter and body:

- First line must be `---`
- Frontmatter ends at the next `---` line
- Everything after the closing `---` (including the blank line) is the body
- No frontmatter block → generate minimal frontmatter (see § 4)

---

## 7. File Encoding and Line Endings

All wiki pages are UTF-8, LF line endings. `integrate` normalises CRLF to LF
on write and rejects non-UTF-8 body content with a validation error.

---

## 8. What Was Removed

The previous version of this document described:
- `Enrichment` JSON → frontmatter merge rules (UNION, APPEND, SET, PRESERVE)
- `QueryResult` JSON → engine-generated frontmatter from JSON fields
- "The wiki owns frontmatter" principle

These are replaced by the simpler model: the author writes the complete file,
the engine validates and commits. See [ingest.md](ingest.md) for the full
ingest specification.
