---
title: "Concept Type"
summary: "concept and query-result — synthesized knowledge and saved conclusions."
read_when:
  - Writing concept or query-result pages
  - Understanding knowledge type fields
status: ready
last_updated: "2025-07-17"
---

# Concept Type

Schema: `schemas/concept.json` (extends `base.json`)

Two types share this schema:

| Type           | Description                                         |
| -------------- | --------------------------------------------------- |
| `concept`      | Synthesized knowledge — one concept per page        |
| `query-result` | Saved conclusion — crystallized session, comparison |

## Additional Fields

| Field        | Type         | Required | Default | Description                                     |
| ------------ | ------------ | -------- | ------- | ----------------------------------------------- |
| `read_when`  | list[string] | yes      |         | Retrieval conditions (situations, not keywords) |
| `tldr`       | string       | no       | none    | One-sentence key takeaway                       |
| `sources`    | list[string] | no       | `[]`    | Slugs of source pages that contributed claims   |
| `concepts`   | list[string] | no       | `[]`    | Slugs of concept pages this page depends on     |
| `confidence` | float 0.0–1.0 | no      | `0.5`   | Certainty of page content. See [base.md](base.md). Legacy strings `high`/`medium`/`low` are read as `0.9`/`0.5`/`0.2`. |
| `claims`     | list[claim]  | no       | `[]`    | Structured claims extracted from sources        |

## Claims

A claim is a factual statement extracted from a source, with optional
confidence and location:

| Field        | Type   | Required | Description                                   |
| ------------ | ------ | -------- | --------------------------------------------- |
| `text`       | string | yes      | The claim as a factual statement              |
| `confidence` | string | no       | `high`, `medium`, `low` — claim-level confidence, distinct from page-level `confidence` |
| `source`     | string | no       | Slug of the source page                       |
| `section`    | string | no       | Section in the source where the claim appears |

```yaml
claims:
  - text: "Sparse MoE reduces effective compute 8x"
    confidence: high
    source: sources/switch-transformer-2021
    section: "Results"
```

> `claims[].confidence` is a string enum (`high`/`medium`/`low`) scoped to the claim object. It is distinct from the page-level `confidence` float field.

## Edge Declarations

| Field           | Relation        | Target types     |
| --------------- | --------------- | ---------------- |
| `sources`       | `fed-by`        | All source types |
| `concepts`      | `depends-on`    | `concept`        |
| `superseded_by` | `superseded-by` | Any              |

## Template

```yaml
title: "Mixture of Experts"
summary: "Sparse routing of tokens to expert subnetworks."
tldr: "MoE reduces compute 8x at pre-training scale."
read_when:
  - "Reasoning about MoE architecture tradeoffs"
status: active
type: concept
last_updated: "2025-07-17"
tags: [mixture-of-experts, scaling, transformers]
sources: [sources/switch-transformer-2021]
concepts: [concepts/scaling-laws]
confidence: 0.9
claims:
  - text: "Sparse MoE reduces effective compute 8x"
    confidence: high
    section: "Results"
```
