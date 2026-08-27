---
title: "Contradiction Type"
summary: "Explicit conflict register — surfaces disagreements between sources rather than silently resolving them."
read_when:
  - Writing contradiction pages
  - Understanding how conflicts between sources are handled
  - Deciding whether to resolve or record a disagreement
status: ready
last_updated: "2026-08-26"
---

# Contradiction Type

Schema: `schemas/contradiction.json` (extends `base.json`)

A contradiction is an explicit record that two or more sources
currently disagree on a claim. It is not a resolution — it is a
refusal to resolve silently. The system surfaces the conflict and names
an owner; the owner decides when and how to resolve it.

Resolved contradictions are never deleted. They form part of the audit
trail: knowing that a conflict existed, who owned it, and when it was
closed is as valuable as the resolution itself.

## Additional Fields

| Field               | Type         | Required | Description                                                        |
| ------------------- | ------------ | -------- | ------------------------------------------------------------------ |
| `severity`          | string       | no       | Impact if left unresolved: `high`, `medium`, or `low`              |
| `accountable_owner` | string       | no       | Person or team responsible for resolution                          |
| `raised_on`         | string       | no       | ISO 8601 date when the contradiction was first recorded            |
| `source_ids`        | list[string] | no       | Slugs of the pages whose claims conflict — flat array, indexable   |

`status` (from `base.json`) carries the contradiction lifecycle:

| Value        | Meaning                                                          |
| ------------ | ---------------------------------------------------------------- |
| `active`     | Contradiction is unresolved and in force                         |
| `draft`      | Candidate contradiction, not yet confirmed                       |
| `generated`  | Machine-detected, pending human review                           |
| `stub`       | Conflict known but not yet fully documented                      |

Use `status: active` for unresolved contradictions and `superseded_by`
(from `base.json`) to link to the resolution page when closed. Do not
delete the contradiction page.

> **Note:** The `status` enum on `base.json` does not include
> `resolved`. Use `superseded_by` pointing to the resolution page
> (a `decision` or `concept` page) to mark closure. This preserves
> the audit trail without requiring a schema enum change.

## Frontmatter is flat by design

The TDS reference model stores `statements[]` as nested objects in
frontmatter. llm-wiki's ingest pipeline does not index nested objects —
they pass through as stored body content only. Keeping `source_ids` as
a flat array makes the conflicting sources queryable:

```
wiki_search --type contradiction --filter source_ids:sources/policy-v2
```

The full statement detail (verbatim claims, per-source effective dates,
locators) belongs in the page body as structured markdown.

## Edge Declarations

| Field           | Relation        | Target types                  |
| --------------- | --------------- | ----------------------------- |
| `source_ids`    | `conflicts-between` | All source types, `doc`, `decision` |
| `superseded_by` | `superseded-by` | `decision`, `concept`         |

## What goes in the body

Structure the body as a `## Statements` section with one entry per
conflicting source. Each entry should include:

- The source slug and its effective date
- The verbatim claim or paraphrase
- Why it conflicts with the other statement(s)

Follow with a `## Resolution` section (empty until resolved) and
optionally a `## Notes` section for context.

```markdown
## Statements

**[sources/policy-v2](sources/policy-v2)** (effective 2025-01-01)
> "Cash settlement is calculated on actual cash value at time of loss."

**[sources/claims-manual-2024](sources/claims-manual-2024)** (effective 2024-03-15)
> "Cash settlement uses replacement cost minus depreciation, not ACV."

These two definitions produce different payout amounts for the same
loss event. The conflict is unresolved pending legal review.

## Resolution

_Unresolved. Owner: claims-team. Raised: 2026-08-26._

## Notes

Affects all claims filed after 2025-01-01 where the insured disputes
the settlement basis.
```

## Relationship to other types

| Type         | Relationship                                                        |
| ------------ | ------------------------------------------------------------------- |
| `decision`   | A decision page may resolve a contradiction — link via `superseded_by` |
| `open_question` | An open question may be `blocked_by` a contradiction slug        |
| Source types | `source_ids` points to the pages whose claims conflict              |

## Agent-layer boundary

The contradiction gate described in the TDS article ("if the topic is
contested, refuse to answer") is an **agent-layer concern**, not an
engine concern. The engine stores contradiction pages, indexes them,
and surfaces them via `wiki_search` and the `unresolved-contradiction`
lint rule. The decision to halt synthesis when a contradiction exists
is the agent's responsibility.

## Template

```yaml
title: "Contradiction: cash settlement basis"
summary: "policy-v2 and claims-manual-2024 define cash settlement differently."
type: contradiction
status: active
last_updated: "2026-08-26"
severity: high
accountable_owner: "claims-team"
raised_on: "2026-08-26"
tags: [claims, settlement, policy]
source_ids:
  - sources/policy-v2
  - sources/claims-manual-2024
```
