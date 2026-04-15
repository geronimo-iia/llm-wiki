# Contradiction Pages — Lifecycle and Surfacing

Contradictions are **first-class knowledge nodes** in `llm-wiki`. They are never
deleted — only enriched. A resolved contradiction carries the explanation of *why*
two sources disagreed; that explanation is the knowledge.

---

## Phase 1: Written at ingest

When an external LLM calls `wiki context` before producing its analysis, it may
detect that a new claim contradicts an existing wiki page. It records this in the
`contradictions[]` array of `analysis.json`.

`wiki ingest` writes each contradiction as `contradictions/<slug>.md` with a fixed
frontmatter schema (see below). The slug is derived by slugifying the title.

```yaml
---
title: "MoE scaling efficiency: contradictory views"
type: contradiction
claim_a: "sparse MoE reduces effective compute 8x at same quality"
source_a: "sources/switch-transformer-2021"
claim_b: "MoE gains diminish sharply beyond 100B parameters"
source_b: "sources/moe-survey-2023"
dimension: context        # context | time | scale | methodology | open-dispute
epistemic_value: >
  Compute/quality tradeoff in MoE is phase-dependent — non-obvious from either
  paper alone.
status: active            # active | resolved | under-analysis
tags: []
created: 2026-04-13
updated: 2026-04-13
---

## Claim A

sparse MoE reduces effective compute 8x at same quality

## Claim B

MoE gains diminish sharply beyond 100B parameters

## Analysis

Compute/quality tradeoff in MoE is phase-dependent — non-obvious from either
paper alone.
```

---

## Phase 3: Surfaced at lint

`wiki lint` walks `contradictions/` and collects pages whose `status` is
`active` or `under-analysis`. These appear in the **Active Contradictions** table
in `LINT.md`.

`wiki contradict` lists all contradiction pages interactively, with an optional
`--status` filter:

```bash
wiki contradict                        # all contradictions
wiki contradict --status active        # only unresolved
wiki contradict --status resolved      # only resolved
wiki contradict --status under-analysis
```

---

## Enrichment workflow

1. Run `wiki lint` → check `LINT.md` for active contradictions.
2. For each active contradiction, run `wiki context "<title>"` to retrieve both
   source pages and the contradiction page itself.
3. Analyse the dimension (`context | time | scale | methodology | open-dispute`)
   and write a resolution explanation.
4. Produce an `analysis.json` with the enriched contradiction:
   ```json
   {
     "contradictions": [{
       "title": "MoE scaling efficiency: contradictory views",
       "claim_a": "...", "source_a": "sources/switch-transformer-2021",
       "claim_b": "...", "source_b": "sources/moe-survey-2023",
       "dimension": "context",
       "epistemic_value": "...",
       "status": "resolved",
       "resolution": "Both true: claim_a holds for pre-training FLOPs; claim_b applies to fine-tuning regime."
     }]
   }
   ```
5. `wiki ingest` updates the contradiction page in-place; the git diff shows
   exactly what changed.

---

## Status transitions

```
active ──────────▶ under-analysis ──────────▶ resolved
  │                                               │
  └───────────────────────────────────────────────┘
            (external LLM enriches and re-ingests)
```

Resolved contradictions stay in `contradictions/` forever.
`git log contradictions/moe-scaling-efficiency.md` traces the full history of how
understanding of that tension evolved.

---

## `ContradictionSummary` (runtime struct)

Used by `wiki lint` and `wiki contradict` without loading full page bodies:

```rust
pub struct ContradictionSummary {
    pub slug: String,       // e.g. "contradictions/moe-scaling-efficiency"
    pub title: String,
    pub status: Status,     // Active | Resolved | UnderAnalysis
    pub dimension: Dimension,
    pub source_a: String,
    pub source_b: String,
}
```

---

## `cluster(graph, slugs)` — adjacent concept pages

Given a list of contradiction slugs, returns all concept/source pages that share
a graph edge with any of those nodes. Useful for identifying which concept areas
are most affected by active contradictions.

```rust
let affected = contradiction::cluster(&graph, &active_slugs);
// e.g. ["concepts/mixture-of-experts", "sources/switch-transformer-2021"]
```
