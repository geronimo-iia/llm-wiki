---
title: "Source Types"
summary: "paper, article, documentation, and more — what each source claims."
read_when:
  - Writing source pages
  - Choosing the right source type
status: ready
last_updated: "2025-07-17"
---

# Source Types

Schema: `schemas/paper.json` (extends `base.json`)

All source types share the same schema. The `type` field carries the
distinction. Classify by the source material's nature, not its topic.

| Type | Source nature |
|------|-------------|
| `paper` | Academic — research papers, preprints |
| `article` | Editorial — blog posts, news, essays |
| `documentation` | Reference — product docs, API references |
| `clipping` | Web capture — browser clips, bookmarks |
| `transcript` | Spoken — meeting transcripts, podcasts |
| `note` | Informal — freeform drafts, quick captures |
| `data` | Structured — CSV, JSON, datasets |
| `book-chapter` | Published — book excerpts |
| `thread` | Discussion — forum threads, social media |

## Additional Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `read_when` | list[string] | no | `[]` | Retrieval conditions |
| `tldr` | string | no | none | One-sentence key takeaway |
| `sources` | list[string] | no | `[]` | Slugs of other source pages (e.g. papers cited by a survey) |
| `concepts` | list[string] | no | `[]` | Slugs of concept pages this source informs |
| `confidence` | float 0.0–1.0 | no | `0.5` | Certainty of page content. See [base.md](base.md). Legacy strings `high`/`medium`/`low` are read as `0.9`/`0.5`/`0.2`. |
| `claims` | list[claim] | no | `[]` | Structured claims (see [concept.md](concept.md) for claim format) |

## Edge Declarations

| Field | Relation | Target types |
|-------|----------|-------------|
| `sources` | `cites` | All source types |
| `concepts` | `informs` | `concept` |
| `superseded_by` | `superseded-by` | Any |

## Template

```yaml
title: "Switch Transformer (2021)"
summary: "Fedus et al. on scaling MoE to trillion parameters."
tldr: "Switch routing achieves 4x speedup over dense baselines."
read_when:
  - "Looking for MoE benchmark results"
status: active
type: paper
last_updated: "2025-07-17"
tags: [mixture-of-experts, switch-transformer, scaling]
concepts: [concepts/mixture-of-experts, concepts/scaling-laws]
confidence: 0.9
claims:
  - text: "Switch routing achieves 4x speedup"
    confidence: high
    source: sources/switch-transformer-2021
    section: "Abstract"
```
