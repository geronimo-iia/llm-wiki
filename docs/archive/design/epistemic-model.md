---
title: "Epistemic Model"
summary: "Why the four default wiki categories exist — the epistemic roles of concepts, sources, queries, and raw, and why separating them matters."
read_when:
  - Understanding why the wiki has these specific default directories
  - Deciding which category a new page belongs in
  - Explaining the design philosophy to a new contributor or LLM
status: active
last_updated: "2025-07-15"
---

# Epistemic Model

The four default wiki categories are not arbitrary. Each has a distinct
epistemic role. Mixing them collapses distinctions that matter for knowledge
quality and provenance tracking.

## Origin

The category structure is grounded in Karpathy's LLM Wiki concept (April 2026
gist). Karpathy's core ideas:

- Process sources at **ingest time**, not query time — build a persistent wiki
- The LLM reads each source and integrates it, updating existing pages
- Save valuable Q&A as **query-result** pages
- Run **lint passes** to audit for orphans and obsolete content

What this project adds beyond Karpathy:

- The explicit **epistemic layer model** below — Karpathy describes the
  workflow but does not name the layers this way
- `raw/` as a dedicated directory — implied by Karpathy's workflow but not
  formalized as a category

---

## The Layers

```
raw/       → what we received        (unprocessed input)
sources/   → what each source claims (provenance)
concepts/  → what we know            (synthesized knowledge)
queries/   → what we concluded       (reasoning output)
```

Each layer answers a different question. None can substitute for another.

---

## `raw/`

**What we received — unprocessed input.**

Original source files: PDFs, transcripts, Markdown files, HTML exports.
Never modified after ingestion. Excluded from tantivy indexing, orphan
detection, and graph traversal.

**Why it exists separately:**

The wiki is self-contained. You can always re-analyze a source from scratch
without fetching it again. `raw/` is an archive, not a knowledge base. Keeping
it separate prevents raw content from polluting search results and concept
pages.

---

## `sources/`

**What each source claims — provenance.**

One page per source document. Records what a specific paper, blog post, or
transcript claims, with what confidence, and where the gaps are.

**Why it exists separately from `concepts/`:**

The same concept can be claimed by many sources with different confidence
levels, different methodologies, and different scopes. Provenance tracking
requires knowing *which source made which claim*. If source summaries are
merged directly into concept pages, that provenance is lost.

```
sources/switch-transformer-2021.md  → "sparse MoE reduces compute 8x (high confidence)"
sources/moe-survey-2023.md          → "MoE gains diminish beyond 100B params (medium confidence)"
```

Without `sources/`, you cannot ask "which sources support this claim?".

---

## `concepts/`

**What we know — synthesized knowledge.**

One page per concept, continuously enriched across multiple sources. The
canonical answer to "what do we know about X?". Pages accumulate claims,
tags, confidence levels, and source links over time.

**Why it exists separately from `sources/`:**

A concept page represents the *current state of knowledge* about a topic,
synthesized from all sources. A source page represents what *one document*
said. These are different things:

- `concepts/mixture-of-experts.md` — everything we know about MoE, from all sources
- `sources/switch-transformer-2021.md` — what this one paper said about MoE

Concept pages are the primary retrieval target for `wiki search`. They are
what an LLM reads to answer a question. Source pages are what an LLM reads
to check provenance.

---

## `queries/`

**What we concluded — reasoning output.**

Saved Q&A results. When an LLM synthesizes an answer from wiki context,
that synthesis is itself knowledge worth preserving — especially when it
draws on multiple concept pages.

**Why it exists separately from `concepts/`:**

A query result is not a concept. It is a *conclusion* drawn from concepts
at a specific point in time, for a specific question. Keeping them separate
prevents conflating:

- "What does the wiki know about MoE?" (concept page)
- "What is the answer to: does MoE scale efficiently?" (query result)

Query results also carry their source slugs — which concept and source pages
were used to produce the answer. This makes them auditable and re-derivable.

---

## Why Separation Matters

The failure mode of naive RAG is collapsing these layers:

| Collapsed | Problem |
|-----------|---------|
| `sources/` merged into `concepts/` | Cannot ask "which source claims this?" — provenance lost |
| `queries/` merged into `concepts/` | Conclusions presented as facts — reasoning not auditable |
| `raw/` indexed alongside pages | Unprocessed content pollutes search — noise in retrieval |

Each separation is a deliberate choice to preserve a distinction that matters
for knowledge quality.

---

## Relationship to `validate_slug`

The fixed category prefixes enforced by `validate_slug_analysis` in
`integrate.rs` are exactly these four directories (minus `raw/`, which is
never a slug prefix). Analysis-only ingest is restricted to these categories
because the enrichment contract (`enrichments[]`, `query_results[]`) maps
directly onto them.

Direct ingest (`wiki ingest <path> --prefix <name>`) relaxes this — user-defined
prefixes like `skills/` or `guides/` are valid because they represent
structured content that does not fit the epistemic model but is still worth
indexing and searching. See [repository-layout.md](repository-layout.md) for
the full distinction between fixed categories and user-defined prefixes.
