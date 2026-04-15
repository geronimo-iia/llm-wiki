---
title: "Frontmatter Authoring Guide"
summary: "LLM-facing reference for writing frontmatter in wiki pages — unified type taxonomy, per-type conventions, and common mistakes."
read_when:
  - Writing wiki pages as an LLM
  - Understanding what frontmatter values to set per page type
  - Writing or reviewing the frontmatter section of src/instructions.md
  - Deciding which type to assign to a new page
status: draft
last_updated: "2025-07-15"
---

# Frontmatter Authoring Guide

The LLM writes complete Markdown files — frontmatter and body. This document
is the reference for how to write frontmatter correctly. It is embedded in
`src/instructions.md` as `## frontmatter` so the LLM has it in context at
every session start.

---

## 1. Required Fields

Every wiki page must have these fields:

```yaml
---
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
status: active
last_updated: "2025-07-15"
type: concept
---
```

| Field | Rule |
|-------|------|
| `title` | Always set. Concise, specific, unambiguous |
| `summary` | Always set. One sentence: what this page is about |
| `status` | Always `active` for new pages. Use `draft` for incomplete work |
| `last_updated` | Today's ISO 8601 date. Engine overwrites this on ingest anyway |
| `type` | Page type from the unified taxonomy (see § 2) |

---

## 2. Page Type Taxonomy

The `type` field carries both the epistemic role and the source nature in a
single field. No separate `classification` field.

### Knowledge types

Pages the wiki synthesizes and maintains.

| Type | Epistemic role | Slug prefix | Description |
|------|---------------|-------------|-------------|
| `concept` | What we know | `concepts/` | Synthesized knowledge, one concept per page |
| `query-result` | What we concluded | `queries/` | Saved Q&A, crystallized sessions, comparisons |
| `section` | Navigation | any | Section index page grouping related pages |

### Source types

Pages that record what a specific source claims. One page per source document.

| Type | Source nature | Slug prefix | Description |
|------|-------------|-------------|-------------|
| `paper` | Academic | `sources/` | Research papers, preprints, academic material |
| `article` | Editorial | `sources/` | Blog posts, news, long-form essays, opinion pieces |
| `documentation` | Reference | `sources/` | Product docs, API references, technical specifications |
| `clipping` | Web capture | `sources/` | Web saves, browser clips, bookmarks with content |
| `transcript` | Spoken | `sources/` | Meeting transcripts, podcast transcripts, interviews |
| `note` | Informal | `sources/` | Freeform drafts, quick captures, personal notes |
| `data` | Structured | `sources/` | CSV, JSON, structured datasets, spreadsheets |
| `book-chapter` | Published | `sources/` | Excerpts or chapters from books |
| `thread` | Discussion | `sources/` | Forum threads, social media threads, discussion archives |

### Choosing a type

- **Is this synthesized knowledge?** → `concept`
- **Is this a conclusion or decision from a session?** → `query-result`
- **Is this a summary of a specific source?** → pick the source type that
  matches the material's nature (`paper`, `article`, `documentation`, etc.)
- **Is this a section index?** → `section`

Classify by the source material's nature, not its topic. A blog post about
academic research is `article`, not `paper`. A PDF of API docs is
`documentation`, not `paper`.

### Custom types

Wiki owners can add domain-specific types in `schema.md`:

```yaml
types:
  - patent
  - legal-filing
  - specification
  - meeting-notes
```

Custom types are valid for any slug prefix. The engine validates against the
combined list (built-in + custom).

---

## 3. Recommended Fields

Include these when relevant:

```yaml
tldr: "MoE reduces compute 8x at pre-training scale."
read_when:
  - "Reasoning about MoE architecture tradeoffs"
  - "Comparing sparse vs dense model scaling"
tags: [mixture-of-experts, scaling, transformers]
sources: [sources/switch-transformer-2021]
concepts: [concepts/scaling-laws]
confidence: high
```

---

## 4. Field-by-Field Guide

### `title`

| Type | Convention | Example |
|------|-----------|---------|
| `concept` | The concept name | "Mixture of Experts" |
| `paper` | "Paper Title (Year)" | "Switch Transformer (2021)" |
| `article` | "Article Title" | "Why MoE Models Are the Future" |
| `documentation` | "Product — Topic" | "vLLM — MoE Configuration" |
| `transcript` | "Event — Topic (Date)" | "Team Standup — MoE Decision (2025-07-15)" |
| `note` | Descriptive title | "Quick Notes on Routing Strategies" |
| `clipping` | Original title or descriptive | "Karpathy on LLM Wikis" |
| `book-chapter` | "Book Title — Chapter" | "Designing ML Systems — Ch. 8" |
| `thread` | "Forum — Topic" | "HN — MoE Scaling Discussion" |
| `data` | Descriptive title | "MoE Benchmark Results 2024" |
| `query-result` | "Topic — Aspect" | "MoE Routing — Design Decision" |
| `section` | The section name | "Scaling Research" |

Do not include the page type in the title ("Paper: Switch Transformer" ✗).

### `summary`

One sentence describing the page's scope. Answers: "what is this page about?"

### `tldr`

One sentence capturing the key takeaway. Answers: "what's the bottom line?"

`summary` describes scope. `tldr` states the conclusion. They may overlap
for simple pages.

### `read_when`

Conditions under which an LLM should retrieve this page. Write them as
situations, not keywords:

```yaml
read_when:
  - "Reasoning about MoE architecture tradeoffs"
  - "Comparing sparse vs dense model scaling"
```

Not: `["MoE", "scaling", "routing"]` — that's what `tags` is for.

Aim for 2–5 entries.

### `tags`

Flat list of search terms. Lowercase, hyphenated:

```yaml
tags: [mixture-of-experts, scaling, transformers, routing]
```

Tags are for search recall (wide net). `read_when` is for precision.

### `sources`

Slugs of source pages that contributed claims to this page:

```yaml
sources:
  - sources/switch-transformer-2021
  - sources/moe-survey-2023
```

Only sources that actually contributed. Not every source that mentions the
topic.

### `concepts`

Slugs of concept pages this page directly discusses or depends on:

```yaml
concepts:
  - concepts/scaling-laws
  - concepts/sparse-attention
```

Apply the [backlink quality test](backlink-quality.md): would a reader
benefit from navigating there?

### `confidence`

| Value | When to use |
|-------|-------------|
| `high` | Multiple corroborating sources, well-established |
| `medium` | Single source, or sources with caveats |
| `low` | Preliminary, speculative, or contradicted |

Default to `medium`. Only set `high` or `low` with clear reason.

### `claims`

Structured claims extracted from sources:

```yaml
claims:
  - text: "Sparse MoE reduces effective compute 8x"
    confidence: high
    section: "Results"
```

Write claims as factual statements, not opinions.

---

## 5. Per-Type Templates

### Concept

```yaml
---
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
tldr: "MoE reduces compute 8x at pre-training scale."
read_when:
  - "Reasoning about MoE architecture tradeoffs"
status: active
last_updated: "2025-07-15"
type: concept
tags: [mixture-of-experts, scaling, transformers]
sources: [sources/switch-transformer-2021, sources/moe-survey-2023]
concepts: [concepts/scaling-laws]
confidence: high
claims:
  - text: "Sparse MoE reduces effective compute 8x"
    confidence: high
    section: "Results"
---
```

### Paper (source)

```yaml
---
title: "Switch Transformer (2021)"
summary: "Fedus et al. on scaling MoE to trillion parameters."
tldr: "Switch routing achieves 4x speedup over dense baselines."
read_when:
  - "Looking for MoE benchmark results"
status: active
last_updated: "2025-07-15"
type: paper
tags: [mixture-of-experts, switch-transformer, scaling]
concepts: [concepts/mixture-of-experts, concepts/scaling-laws]
confidence: high
claims:
  - text: "Switch routing achieves 4x speedup"
    confidence: high
    section: "Abstract"
---
```

### Article (source)

```yaml
---
title: "Why MoE Models Are the Future"
summary: "Industry perspective on MoE adoption trends."
tldr: "MoE is becoming the default architecture for large-scale inference."
read_when:
  - "Understanding MoE industry adoption"
status: active
last_updated: "2025-07-15"
type: article
tags: [mixture-of-experts, industry, inference]
concepts: [concepts/mixture-of-experts]
confidence: medium
---
```

### Documentation (source)

```yaml
---
title: "vLLM — MoE Configuration"
summary: "Official vLLM docs on configuring MoE model serving."
read_when:
  - "Setting up MoE inference with vLLM"
status: active
last_updated: "2025-07-15"
type: documentation
tags: [vllm, moe, inference, configuration]
concepts: [concepts/mixture-of-experts]
confidence: high
---
```

### Query result

```yaml
---
title: "MoE Routing — Design Decision"
summary: "Expert-choice routing selected for inference pipeline."
tldr: "Expert-choice gives best quality/efficiency tradeoff above 10B."
read_when:
  - "Reviewing MoE routing decisions"
status: active
last_updated: "2025-07-15"
type: query-result
tags: [moe, routing, inference]
sources: [sources/switch-transformer-2021]
concepts: [concepts/mixture-of-experts]
confidence: medium
---
```

---

## 6. Updating Existing Pages

When updating a page, you must:

1. **Read the current page first** — `wiki_read(<slug>)`
2. **Preserve existing list values** — do not drop `tags`, `read_when`,
   `sources`, `concepts`, or `claims` added by prior ingests
3. **Add new values** to lists, do not replace them
4. **Update scalar fields** (`summary`, `tldr`, `confidence`) only when
   you have a clear reason
5. **Write the complete file** and ingest with `--update`

This is the accumulation contract. The engine does not enforce it — you do.

---

## 7. Common Mistakes

| Mistake | Fix |
|---------|-----|
| Missing `title` | Engine rejects the file. Always include it |
| Using `source-summary` as type | Pick the specific source type: `paper`, `article`, `documentation`, etc. |
| Writing `read_when` as keywords | Write retrieval situations, not keywords |
| Listing every related source in `sources` | Only sources that contributed claims |
| Dropping existing tags/sources on update | Read the page first, preserve existing values |
| Setting `confidence: high` without evidence | Default to `medium` |
| Classifying by topic instead of source nature | A blog post about research is `article`, not `paper` |

---

## 8. Instruct Integration

This guide is embedded in `src/instructions.md` as `## frontmatter` — a
condensed version optimized for token efficiency: type taxonomy table,
per-type templates, update rules, common mistakes. No rationale paragraphs.

---

## 9. Implementation Status

| Feature | Status |
|---------|--------|
| `## frontmatter` in `src/instructions.md` | **not implemented** |
| Frontmatter validation on ingest | **not implemented** |
| Type taxonomy validation (built-in + custom) | **not implemented** |
| Warning on missing recommended fields | **not implemented** |
