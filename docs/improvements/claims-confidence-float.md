---
title: "claims[].confidence as float"
summary: "Align claims[].confidence with page-level confidence — change from string enum to float 0.0–1.0."
status: proposed
last_updated: "2026-04-27"
depends_on: confidence
---

# claims[].confidence as float

## Problem

Page-level `confidence` is now a float `0.0–1.0`. The `claims[].confidence`
sub-field inside `concept` and `paper` schemas is still a string enum
(`high`, `medium`, `low`). This inconsistency means:

- LLMs must use two different mental models for the same word in the same file.
- Claim-level certainty cannot be used in numeric comparisons or aggregations.
- The frontmatter skill's guidance table (`high` / `medium` / `low`) applies
  at the page level but not at the claim level, creating a confusing split.

No existing pages use `claims[].confidence` yet, so there is no migration cost.

## Goal

Uniform schema: `confidence` is always a float `0.0–1.0` wherever it appears.

## Solution

Change `claims[].confidence` in both `schemas/concept.json` and
`schemas/paper.json` from:

```json
"confidence": {
  "type": "string",
  "enum": ["high", "medium", "low"]
}
```

to:

```json
"confidence": {
  "type": "number",
  "minimum": 0.0,
  "maximum": 1.0,
  "description": "Certainty of this claim. 0.0=speculative, 1.0=verified."
}
```

No `default` is set — claim-level confidence remains optional (absence means
no certainty signal was recorded, which is distinct from `0.5` neutral).

**Conventional values** (same scale as page-level):
- `0.9` — well-corroborated, multiple sources agree
- `0.5` — single source, or source has caveats
- `0.2` — preliminary, speculative, or contradicted by other claims

There is no legacy string mapping needed for `claims[].confidence` because
no pages exist yet.

## Impact

### Schemas (JSON)
- `schemas/concept.json` — `claims[].confidence`: string enum → number
- `schemas/paper.json` — `claims[].confidence`: string enum → number

### Source code
No Rust changes required. `claims` is an opaque array stored as text in the
tantivy index; `claims[].confidence` is never read by the engine directly.
The `frontmatter::confidence()` getter operates on page-level only and is
unaffected.

The schema validator in `ingest` will now reject `claims[].confidence: "high"`.
Since no pages exist yet, this is safe.

### Tests
- `tests/default_schemas.rs` — two existing tests pass `"confidence": "high"`
  inside a `claims` item. Both must be updated to use a float (e.g. `0.9`).

### llm-wiki-skills
Four files reference `claims[].confidence` as a string enum and must be updated:

| File | Change needed |
|------|--------------|
| `skills/frontmatter/SKILL.md` | `### confidence` section: add float scale table; update `claims` example from `confidence: high` to `confidence: 0.9`; update anti-pattern row `confidence: high` → `confidence: 0.9` |
| `skills/frontmatter/SKILL.md` | `### claims` example block: `confidence: high` → `confidence: 0.9` |
| `skills/ingest/SKILL.md` | Any claim example using `confidence: high` |
| `skills/frontmatter/references/type-taxonomy.md` | Prose mentioning `confidence` on claim objects |

### Specification docs
- `docs/specifications/model/types/concept.md` — Claims table: change
  `confidence` row from `string / high,medium,low` to `float 0.0–1.0`;
  update claim example to `confidence: 0.9`; remove the note distinguishing
  claim vs page confidence types (they are now the same type).
- `docs/specifications/model/types/source.md` — same (references concept.md
  for claim format, but template example also uses `confidence: high`).

## Branch & PR — `llm-wiki`

Implemented as part of `feat/confidence-search-ranking`. No separate branch
needed — schema and test changes ship in the same PR as imp-1.

## Branch & PR — `llm-wiki-skills`

Skills changes are pure Markdown and have no engine dependency — they can
be developed in parallel with the engine branch and merged independently.

```bash
# in llm-wiki-skills repo
git checkout -b feat/claims-confidence-float
```

When done:

```bash
git push -u origin feat/claims-confidence-float
gh pr create \
  --repo geronimo-iia/llm-wiki-skills \
  --milestone "v0.4.0" \
  --title "fix: align claims[].confidence to float 0.0–1.0" \
  --body "$(cat <<'EOF'
Aligns skill documentation with the schema change in llm-wiki imp-1b:
`claims[].confidence` is now a float `0.0–1.0`, not a string enum.

- `skills/frontmatter/SKILL.md`: rewrite confidence table to float scale;
  update claims example and anti-pattern row
- `skills/ingest/SKILL.md`: update any claim examples using string confidence
- `skills/frontmatter/references/type-taxonomy.md`: update confidence prose

Companion to llm-wiki feat/confidence-search-ranking (imp-1b).
Closes geronimo-iia/llm-wiki-skills#1 (imp-1b)
EOF
)"
```

> Merge timing: can merge before or after the engine PR — no runtime
> dependency. Merging before is fine since the skills just document what the
> schema will enforce.

## Tasks

### Engine — `llm-wiki` (branch: `feat/confidence-search-ranking`)

#### Schemas (JSON)
- [ ] `schemas/concept.json`: change `claims[].confidence` from `type: string, enum: [...]` to `type: number, minimum: 0.0, maximum: 1.0`.
- [ ] `schemas/paper.json`: same change.

#### Tests
- [ ] `tests/default_schemas.rs` `concept_accepts_full_template`: change `"confidence": "high"` inside claims item to `"confidence": 0.9`.
- [ ] `tests/default_schemas.rs` `paper_accepts_source_template`: same.
- [ ] Add test: `concept_rejects_string_confidence_in_claim` — assert `claims[].confidence: "high"` is now invalid.
- [ ] Add test: `paper_rejects_string_confidence_in_claim` — same.

#### Specification docs
- [ ] `docs/specifications/model/types/concept.md`: update Claims table row and example; remove the `claims[].confidence` distinction note (types are now uniform).
- [ ] `docs/specifications/model/types/source.md`: update template example `confidence: high` in claims → `confidence: 0.9`.

### Skills — `llm-wiki-skills` (branch: `feat/claims-confidence-float`)

- [ ] `skills/frontmatter/SKILL.md`: rewrite `### confidence` table from string enum to float scale; update anti-pattern row.
- [ ] `skills/frontmatter/SKILL.md`: update `### claims` example: `confidence: high` → `confidence: 0.9`.
- [ ] `skills/ingest/SKILL.md`: update any claim examples using string confidence.
- [ ] `skills/frontmatter/references/type-taxonomy.md`: update confidence prose.
</content>
</invoke>