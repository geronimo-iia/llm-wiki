---
title: "Decision Type"
summary: "Recorded choice with governance metadata — effective date, accountable owner, and rationale links."
read_when:
  - Writing decision pages
  - Storing governance records in the wiki
  - Understanding how decisions differ from docs
status: ready
last_updated: "2026-08-26"
---

# Decision Type

Schema: `schemas/decision.json` (extends `base.json`)

A decision is a recorded choice: a rule, policy, or architectural
commitment that came into force at a specific date and has a named
owner. It is distinct from a `doc` in one key way — a doc is a
reference document whose authority is its content; a decision is a
governance record whose authority is the act of choosing.

The rationale (the reasoning behind the choice, which may live in an
email, a meeting transcript, or a separate source page) is linked via
`sources[]`, not embedded in the frontmatter.

## Additional Fields

| Field               | Type         | Required | Description                                                |
| ------------------- | ------------ | -------- | ---------------------------------------------------------- |
| `effective_date`    | string       | no       | ISO 8601 date when the decision came into force            |
| `accountable_owner` | string       | no       | Person or team responsible if the decision is challenged   |
| `sources`           | list[string] | no       | Slugs of source pages that carry the rationale or evidence |

`status` (from `base.json`) carries the decision lifecycle:

| Value       | Meaning                                               |
| ----------- | ----------------------------------------------------- |
| `active`    | Decision is in force                                  |
| `draft`     | Proposed, not yet ratified                            |
| `stub`      | Placeholder — decision known but not yet documented   |
| `generated` | Machine-generated draft, pending human review         |

When a decision is superseded, set `status: active` on the replacement
and use `superseded_by` (from `base.json`) on the old page to link
forward. Add `superseded_on` (from `base.json`) for the date the old
decision ceased to apply.

## Edge Declarations

| Field           | Relation        | Target types     |
| --------------- | --------------- | ---------------- |
| `sources`       | `informed-by`   | All source types |
| `superseded_by` | `superseded-by` | Any              |

## What goes in the body

The page body carries the substance the frontmatter cannot:

- **Rule text** — the decision stated precisely
- **Rationale** — why this choice was made over alternatives
- **Scope** — what the decision applies to and what it does not
- **Alternatives considered** — options that were rejected and why

Keeping rationale in the body (not frontmatter) means it can be as
long as needed and is full-text searchable.

## Relationship to `doc`

Use `decision` when the page records a choice that was made. Use `doc`
when the page is a reference document (specification, guide, standard)
that describes how something works. A decision page may link to a `doc`
page via `sources[]` if the doc is the authoritative source for the
rule text.

## Template

```yaml
title: "Use tantivy for full-text search"
summary: "tantivy chosen over Meilisearch and Elasticsearch for the search index."
type: decision
status: active
last_updated: "2026-08-26"
effective_date: "2026-01-15"
accountable_owner: "geronimo"
tags: [architecture, search, tantivy]
sources:
  - sources/tantivy-benchmark-2025
  - sources/search-evaluation-notes
```

Body:

```markdown
## Rule

All full-text search in llm-wiki uses tantivy as the index backend.

## Rationale

tantivy is a pure-Rust library with no external process dependency.
Meilisearch and Elasticsearch require a running server, which adds
operational complexity for a single-user tool. tantivy compiles into
the binary directly.

## Scope

Applies to the search index only. Graph traversal uses petgraph.

## Alternatives considered

- **Meilisearch** — excellent relevance, but requires a separate process.
- **Elasticsearch** — mature, but JVM dependency is unacceptable for a
  CLI tool.
```
