---
title: "Design Evolution — Analysis as Enrichment"
summary: "How the accumulated design decisions since design.md shift the role of analysis.json from primary page-creation mechanism to frontmatter enrichment layer, and what that implies for the contract, workflow, and module architecture."
read_when:
  - Understanding why analysis.json changed from the primary interface to an enrichment layer
  - Revising the analysis.json contract or the ingest pipeline
  - Reconciling design.md with ingest.md, asset-ingest.md, and repository-layout.md
status: draft
last_updated: "2025-07-15"
---

# Design Evolution — Analysis as Enrichment

`design.md` was written with one central assumption: `analysis.json` is the
primary interface. The LLM reads a source, produces analysis, the wiki stores
the resulting pages. Everything flows from that.

Four subsequent design decisions have collectively inverted this assumption.
This document traces the evolution, states the new model explicitly, and
identifies what changes in the contract and implementation.

---

## 1. The Four Decisions That Changed the Model

### 1.1 Direct ingest as default (ingest.md)

`wiki ingest <path>` — file or folder, no LLM required — became the primary
entry point. Analysis JSON became an optional `--analysis` flag. The implication:
**pages exist before the LLM sees them**.

### 1.2 Co-located assets (repository-layout.md, asset-ingest.md)

Non-Markdown files stay beside their page in a bundle folder. The wiki is not
assembled from LLM output — it mirrors the source structure. The implication:
**the wiki is a structured copy of existing content, not LLM-generated content**.

### 1.3 Context returns references not content (context-retrieval.md)

`wiki context` returns slugs, URIs, and paths. The LLM fetches what it needs.
The implication: **the LLM reads pages from the wiki, it does not write them**.

### 1.4 Analysis as enrichment (this document)

Following from 1.1–1.3: the LLM's job is to read existing pages and annotate
them — add claims, concepts, confidence to frontmatter. It does not author
page bodies. The body is already there from direct ingest.

---

## 2. The New Model

```
Direct ingest (primary)          Analysis enrichment (optional, LLM)
─────────────────────────        ──────────────────────────────────────
wiki ingest <path>               wiki search → LLM reads pages
  write pages as-is                LLM produces enrichment.json
  co-locate assets                 wiki ingest --analysis enrichment.json
  git commit                         merge claims/concepts into frontmatter
                                     git commit
```

The wiki is a structured, versioned copy of existing content. The LLM enriches
it with semantic metadata. The two operations are independent and composable:
you can ingest without enriching, enrich without re-ingesting, or do both.

---

## 3. What Changes in analysis.json

### 3.1 `suggested_pages` — body removed, action simplified

**Before**: the LLM writes the full page body. The wiki creates the page from
LLM output.

**After**: the page already exists. The LLM annotates it. `body` is removed.
`action` is always `enrich` (a new value replacing `create`/`update`/`append`).

```json
// Before
{
  "suggested_pages": [
    {
      "slug": "concepts/mixture-of-experts",
      "title": "Mixture of Experts",
      "type": "concept",
      "action": "create",
      "tldr": "...",
      "body": "## Overview\n\nMoE routes tokens...",
      "tags": ["transformers"],
      "read_when": ["Reasoning about MoE architecture"]
    }
  ]
}

// After
{
  "enrichments": [
    {
      "slug": "concepts/mixture-of-experts",
      "claims": [
        { "text": "sparse MoE reduces effective compute 8x", "confidence": "high", "section": "Results" }
      ],
      "concepts": ["scaling-laws", "transformer"],
      "tags": ["transformers", "scaling"],
      "read_when": ["Reasoning about MoE architecture"],
      "confidence": "high",
      "sources": ["sources/switch-transformer-2021"]
    }
  ]
}
```

The `enrichments` array replaces `suggested_pages`. Each entry targets an
existing page by slug and merges metadata into its frontmatter. The body is
never touched.

### 3.2 `doc_type`, `title`, `language`, `key_quotes`, `data_gaps` — removed

These fields described the source document being analyzed. With direct ingest,
the source document is already in the wiki as a page. These fields are redundant
— the page itself carries `title`, `type`, `sources`, etc.

The only remaining top-level fields are:

```json
{
  "source": "sources/switch-transformer-2021",   // which wiki page was analyzed
  "enrichments": [...],                           // frontmatter additions per page
  "query_results": [...]                          // LLM-authored pages
}
```

### 3.3 New `enrichment` action in integrate.rs

`integrate_analysis` gains a new path: for each `enrichment` entry, read the
existing page's frontmatter, merge the new fields (union tags, union read_when,
union sources, set confidence, append claims), write back. Body is never read
or written.

```rust
pub struct Enrichment {
    pub slug:       String,
    pub claims:     Vec<Claim>,
    pub concepts:   Vec<String>,
    pub tags:       Vec<String>,
    pub read_when:  Vec<String>,
    pub confidence: Option<Confidence>,
    pub sources:    Vec<String>,
}
```

---

## 4. What Changes in the Workflow

### Before

```
1. Obtain source (PDF, URL, Markdown)
2. LLM reads source → produces analysis.json (pages + contradictions)
3. wiki ingest analysis.json → creates pages, writes contradictions
4. wiki context → LLM synthesizes answers
5. wiki lint → LLM enriches contradictions → re-ingest
```

### After

```
1. wiki ingest <path>              → pages exist, assets co-located
2. wiki search → LLM reads pages  → produces enrichment.json
3. wiki ingest --analysis enrichment.json → merges metadata into frontmatter
4. wiki search → LLM synthesizes answers using enriched pages
5. wiki lint → LLM fills stubs, links orphans
```

Step 2 and 3 are optional. A wiki of direct-ingested pages is already useful
for search and context retrieval without any LLM enrichment.

---

## 5. What Changes in the Prompts

The `ingest_source` MCP prompt changes significantly:

**Before**: "Read the source, produce analysis.json with page bodies."

**After**: "The page is already in the wiki. Call `wiki_read` to read it. Produce
enrichment.json with claims, concepts, confidence, and contradictions. Call
`wiki_ingest --analysis` to merge."

The `research_question` and `lint_and_enrich` prompts are unchanged.

---

## 6. What Stays the Same

- Git as backend — unchanged
- No LLM calls inside the wiki binary — unchanged
- Tantivy full-text search — unchanged
- MCP server and resource exposure — unchanged
- Multi-wiki registry — unchanged
- `wiki lint` structural audit — unchanged
- `wiki search` returning `Vec<PageRef>` — unchanged

---

## 7. The Remaining Use Case for Body-Writing

There is one case where the LLM still writes a page body: **query results**.

When an LLM synthesizes an answer from wiki context, that answer can be saved
as a `query-result` page. This page has no pre-existing file — it is created
from LLM output. The `suggested_pages` mechanism with `action: create` survives
for this case only.

```json
{
  "source": "query:moe-scaling-efficiency",
  "enrichments": [],
  "query_results": [
    {
      "slug": "queries/moe-scaling-efficiency-2025",
      "title": "MoE scaling efficiency — synthesis",
      "tldr": "...",
      "body": "## Summary\n\n...",
      "tags": ["moe", "scaling"],
      "read_when": ["Reviewing MoE scaling tradeoffs"],
      "sources": ["concepts/mixture-of-experts"]
    }
  ]
}
```

`query_results` replaces `suggested_pages` for this case — same fields, but
semantically distinct: these are LLM-authored pages, not enrichments of existing
pages.

---

## 8. Impact on design.md

The following sections of `design.md` are superseded by this document and the
four referenced docs:

| Section | Superseded by |
|---------|---------------|
| "The `analysis.json` Contract" | This document § 3 |
| "The Workflow" | This document § 4 |
| "Repository Layout" | [repository-layout.md](repository-layout.md) |
| CLI `wiki ingest <file\|->` | [ingest.md](ingest.md) |
| CLI `wiki context` | [context-retrieval.md](context-retrieval.md) |
| MCP `wiki_ingest` tool | [ingest.md § 7](ingest.md) |
| MCP `wiki_context` tool | [context-retrieval.md § 5](context-retrieval.md) |
| `ingest_source` prompt | This document § 5 |

The core insight of `design.md` — contradictions as knowledge, git as backend,
no LLM inside the binary — remains fully valid and unchanged.

---

## 9. Revised enrichment.json Schema

```json
{
  "source": "sources/switch-transformer-2021",
  "enrichments": [
    {
      "slug": "concepts/mixture-of-experts",
      "claims": [
        { "text": "sparse MoE reduces effective compute 8x", "confidence": "high", "section": "Results" }
      ],
      "concepts": ["scaling-laws", "transformer"],
      "tags": ["transformers", "scaling", "compute-efficiency"],
      "read_when": ["Reasoning about MoE architecture or scaling efficiency"],
      "confidence": "high",
      "sources": ["sources/switch-transformer-2021"]
    }
  ],
  "query_results": [
    {
      "slug": "queries/moe-scaling-efficiency-2025",
      "title": "MoE scaling efficiency — synthesis",
      "tldr": "MoE compute gains are phase-dependent.",
      "body": "## Summary\n\n...",
      "tags": ["moe", "scaling"],
      "read_when": ["Reviewing MoE scaling tradeoffs"],
      "sources": ["concepts/mixture-of-experts"]
    }
  ]
}
```

Two top-level arrays, each with a distinct purpose:
- `enrichments` — metadata additions to existing pages (no body)
- `query_results` — LLM-authored pages (body present, always `create`)

---

## 10. Rust Struct Changes

```rust
// Replaces SuggestedPage for the enrichment case
pub struct Enrichment {
    pub slug:       String,
    pub claims:     Vec<Claim>,
    pub concepts:   Vec<String>,
    pub tags:       Vec<String>,
    pub read_when:  Vec<String>,
    pub confidence: Option<Confidence>,
    pub sources:    Vec<String>,
}

// Replaces SuggestedPage for the query-result case
pub struct QueryResult {
    pub slug:      String,
    pub title:     String,
    pub tldr:      String,
    pub body:      String,
    pub tags:      Vec<String>,
    pub read_when: Vec<String>,
    pub sources:   Vec<String>,
}

// Analysis struct — simplified
pub struct Analysis {
    pub source:        String,
    pub enrichments:   Vec<Enrichment>,
    pub query_results: Vec<QueryResult>,
    pub assets:        Vec<Asset>,
}
```

`SuggestedPage`, `Action`, `DocType`, `PageType`, `Contradiction` are removed
from `analysis.rs`. `integrate.rs` gains `integrate_enrichment` and
`integrate_query_result` replacing the old `Action`-based dispatch.
