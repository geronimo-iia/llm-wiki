---
title: "Schema type gaps — persistent knowledge layer patterns"
summary: "Gap analysis between llm-wiki schema types and the 7-type model from TDS persistent-knowledge-layer article. Covers aliases, contradiction, open_question, decision governance fields, superseded_on, and typed relationships."
status: draft
date: "2026-08-26"
reviewed: "2026-08-26"
---

# Schema Type Gaps — Persistent Knowledge Layer Patterns

## Context

Mapping llm-wiki's 6 schema files against the 7-type model described in
[Designing a Persistent Knowledge Layer That Refuses to Guess](https://towardsdatascience.com/designing-a-persistent-knowledge-layer-that-refuses-to-guess/)
(TDS, 2026) and its demo repo `mcekikj/persistent-knowledge-layer`.

Reference doc: `docs/design-origins/2026-08-26-tds-persistent-knowledge-layer.md`

## Design boundary

Several gaps below involve behavioral guarantees (contradiction gate on query
exit, temporal filtering, entity resolution). These belong in the **agent /
skill layer**, not in the engine. The engine's role is: store typed pages,
index them, surface them via search and lint. The gate is the agent's
responsibility.

Schema changes are necessary but not sufficient for the behavioral properties.
This distinction is called out explicitly for each gap.

## First proposals

Initial schema and specification drafts for the two highest-effort new types:

| Type            | Schema                                               | Specification                                    |
| --------------- | ---------------------------------------------------- | ------------------------------------------------ |
| `decision`      | [types/decision.json](types/decision.json)           | [types/decision.md](types/decision.md)           |
| `contradiction` | [types/contradiction.json](types/contradiction.json) | [types/contradiction.md](types/contradiction.md) |

These are first proposals — not yet merged into `schemas/` or
`docs/specifications/model/types/`. Review against the constraints in
each gap section below before landing.

## Gap 1 — `concept`: missing `aliases` field

Current state: `concept.json` has `title`, `summary`, `confidence`,
`claims[]`, `sources[]`. No alias list.

Gap: Without an explicit `aliases[]` field, entity resolution during
ingestion has no stable list of synonyms to check. The wiki can silently
fragment: "actual cash value", "ACV", and "depreciated value" become three
separate concept pages instead of one with three aliases.

Recommended action: Add field to `concept.json`.

```json
"aliases": {
  "type": "array",
  "items": { "type": "string" },
  "x-keyword": true,
  "description": "Alternative names and synonyms for this concept"
}
```

Scope boundary: The schema change is step 1 — it makes aliases
searchable immediately (a query for "ACV" will hit the concept page). It does
not implement entity resolution. Preventing fragmentation at ingest time
requires a `wiki_ingest` step that loads existing aliases and checks new pages
against them. That is a separate, larger item.

Also missing: `last_validated_at` — a date stamp for when the concept was
last reviewed for accuracy. Not blocking; defer.

Effort: Low. **Can ship independently.**

## Gap 2 — `concept`: `query-result` covers comparison partially

Current state: `query-result` (concept.json) is described as "saved
conclusion — crystallized sessions, comparisons". It reuses all concept fields.

Gap: No structured `subject_a` / `subject_b` fields. A comparison is
navigable as free text but not queryable as a structured object.

Severity: Low — llm-wiki does not currently expose a dedicated comparison
query path. Defer until comparison retrieval is a real use case.

## Gap 3 — `doc`: missing decision governance fields

Current state: `doc.json` covers reference documents. A decision record is
currently stored as a `doc`.

Gap: `doc` has no:
- `effective_date` — when the decision came into force
- `accountable_owner` — who owns resolution if the decision is challenged
- `rationale` — explicit link to the reasoning source

Without these, decisions are indistinguishable from generic reference docs and
governance metadata is lost.

Option analysis:

| Option                                               | Assessment                                                                                                                                                                                                         |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add fields to `base.json`                            | Pollutes all types with decision-specific fields                                                                                                                                                                   |
| Add `decision` type to `doc.json` via `x-wiki-types` | **Wrong.** Registering a new type in `x-wiki-types` without adding the fields to `properties` means those fields pass through as `additionalProperties: true` silently — unvalidated. That is a label, not a type. |
| New `decision.json` schema file                      | **Correct.** One new file, consistent with how `skill.json` was added. Not high friction in this system.                                                                                                           |

Recommended action: New `decision.json` with its own `properties` block.

Minimum fields:
```json
{
  "x-wiki-types": { "decision": "Recorded choice with rationale and governance metadata" },
  "required": ["title", "type"],
  "properties": {
    "effective_date": { "type": "string", "description": "ISO 8601 date when the decision came into force" },
    "accountable_owner": { "type": "string", "description": "Person or team responsible if the decision is challenged" },
    "sources": { "type": "array", "items": { "type": "string" }, "description": "Slugs of source pages that informed this decision" }
  }
}
```

Effort: Low. **Can ship independently.**

## Gap 4 — `contradiction`: no type exists

Current state: No contradiction type. Conflicts between sources are not
surfaced; the system would pick one silently.

Gap: A contradiction register is one of the core safety properties of the
persistent knowledge layer pattern:
- When two current sources disagree, the conflict must be surfaced explicitly.
- Contradictions must name an accountable owner and carry a status.
- Resolved contradictions must not be deleted — they form part of the audit trail.

Recommended action: New `contradiction.json` schema file.

Frontmatter shape — flat fields only:

The TDS demo uses nested `statements[]` objects in frontmatter. llm-wiki's
ingest pipeline extracts scalar and array-of-string fields. Nested objects in
frontmatter are not indexed — they pass through as stored body content only.
If `source_id` must be queryable (e.g., "find all contradictions involving
this source"), it must be a flat array.

```yaml
type: contradiction
status: unresolved        # unresolved | resolved
severity: high            # high | medium | low
accountable_owner: "<string>"
raised_on: "YYYY-MM-DD"
source_ids:               # flat, indexable
  - "sources/doc-a"
  - "sources/doc-b"
```

The full `statements[]` detail (verbatim claims, per-source effective dates)
belongs in the page body as markdown, not in frontmatter.

Scope boundary: The TDS article's "contradiction gate on query exit" is an
agent-layer concern. The engine's role is: store contradiction pages, index
them, and surface them via `wiki_search` and a future `wiki_lint` rule
("unresolved contradictions exist"). The gate — refusing to answer when a
topic is contested — is the agent's responsibility.

Effort: Medium (new schema + lint rule). **Prerequisite for Gap 5.**

## Gap 5 — `open_question`: no type exists

Current state: No open_question type. There is no formal way to record
"this cannot be answered yet".

Gap: A visible unanswered question is safer than one that gets quietly
answered wrong. Key use cases:
- Questions blocked by a contradiction (the answer depends on resolving the
  conflict first).
- Questions where the corpus is silent.

Implementation choice — per-file vs single-file:

The TDS article stores open questions as sections in a single `Open
Questions.md`, not as per-file frontmatter. That is a deliberate choice:
open questions are transient, they get resolved or closed, and a single file
is easier to scan and manage than dozens of stub pages.

Before adding a new type, decide: do open questions need to be first-class
indexed pages (searchable, lintable, linkable from contradiction pages)? If
yes, the per-file approach below is correct. If no, a single `open-questions.md`
with `type: section` and structured body is simpler and avoids polluting the
type registry.

If per-file is chosen, proposed frontmatter:
```yaml
type: open_question
question: "<string>"
what_we_know: "<string>"
blocked_by: "<contradiction-slug | null>"
```

`blocked_by` creates a traversable link to a contradiction page and is the
main reason to prefer per-file over single-file.

Severity: Medium. **Requires Gap 4 to land first.**

## Gap 6 — source types: missing `superseded_on` and `version`

Current state: `base.json` has `superseded_by` (slug of replacement page).
All types inherit it.

Gap:
- `superseded_on` — the date supersession took effect. Without it, temporal
  filtering ("answer as at date X") cannot determine whether a source was in
  force at query time.
- `version` — document version identifier.

Recommended action:

Add `superseded_on` to `base.json`:
```json
"superseded_on": {
  "type": "string",
  "description": "ISO 8601 date when this page was superseded"
}
```

**Do not add `version` to `base.json`.** Version means different things for
different types — a policy document version is not the same as a software spec
version. Adding it to `base.json` gives it no semantics. If a specific type
needs versioning (e.g., `doc`), add it to that schema with a concrete use case.

Scope boundary: `superseded_on` in the schema enables the field to be
stored and searched. Temporal filtering ("was this source in force at date X?")
is a query-layer concern — the agent must apply the filter. The engine does not
enforce temporal validity automatically.

Effort: Low. **Can ship independently.**

## Gap 7 — relationships: untyped wiki-links only

Current state: Relationships between pages are expressed as wiki-links
(`[[page]]`) and the `concepts[]` / `sources[]` fields on concept and paper
pages. The schemas already declare typed edges via `x-graph-edges` for these
fields (`fed-by`, `depends-on`, `superseded-by`, etc.). No standalone
relationship record type exists for ad-hoc typed edges.

Gap: Ad-hoc typed edges not captured by schema fields cannot be expressed
as first-class records. Multi-hop queries need traversable typed edges beyond
what `x-graph-edges` covers.

Severity: Low for current llm-wiki use cases. Becomes relevant if a graph
traversal query path is added. The existing `x-graph-edges` infrastructure
already handles the common cases.

Recommended action: Defer. Design the relationship record structure before
implementing graph traversal. The `x-graph-edges` mechanism is the right
foundation — extend it rather than adding a standalone type.


## Priority order

| Gap                         | Action                                         | Effort | Ships independently |
| --------------------------- | ---------------------------------------------- | ------ | ------------------- |
| 1 — `aliases` on concept    | Add field to `concept.json`                    | Low    | Yes                 |
| 6 — `superseded_on` on base | Add field to `base.json`                       | Low    | Yes                 |
| 3 — `decision` type         | New `decision.json`                            | Low    | Yes                 |
| 4 — `contradiction` type    | New `contradiction.json` + lint rule           | Medium | Yes                 |
| 5 — `open_question` type    | Decide per-file vs single-file; requires Gap 4 | Medium | No                  |
| 2 — comparison structure    | Defer                                          | —      | —                   |
| 7 — typed relationships     | Defer; extend `x-graph-edges` when needed      | High   | —                   |

Gaps 1, 6, and 3 are pure schema additions with no engine changes required.
They can ship in any order. Gap 4 is the prerequisite for Gap 5. Gaps 2 and 7
have no concrete use case yet.
