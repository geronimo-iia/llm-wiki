# Lint Module — `lint.rs`

Structural audit of the wiki: orphan pages, missing concept stubs, and unresolved
contradiction pages. Produces a `LINT.md` report and commits it.

---

## What `wiki lint` checks

| Section | What it flags | How it's detected |
|---|---|---|
| **Orphans** | Pages no other page links to | in-degree = 0 in petgraph (`graph::orphans`) |
| **Missing stubs** | Slugs referenced but not on disk | referenced node with no `.md` file (`graph::missing_stubs`) |
| **Active contradictions** | Contradiction pages awaiting enrichment | `status: active` or `status: under-analysis` |

`raw/` pages are excluded from orphan detection — raw source files are not
expected to have inbound links.

---

## `LintReport` struct

```rust
pub struct LintReport {
    pub orphan_pages: Vec<String>,         // slugs
    pub missing_stubs: Vec<String>,        // slugs
    pub active_contradictions: Vec<ContradictionSummary>,
}
```

`active_contradictions` carries full `ContradictionSummary` objects (not just
slugs) so `write_lint_report` can populate the table without re-reading files.

---

## `LINT.md` format

```markdown
# Lint Report

_Generated: 2026-04-13_

## Orphans

Pages with no inbound links. Consider adding cross-references or removing them.

- `concepts/some-isolated-page`

## Missing Stubs

Slugs referenced by other pages but not yet created on disk.

- `concepts/not-yet-written`

## Active Contradictions

Contradiction pages awaiting enrichment (`active` or `under-analysis`).

| Slug | Title | Dimension | Source A | Source B |
|------|-------|-----------|----------|----------|
| `contradictions/moe-scaling-efficiency` | MoE scaling: contradictory views | context | `sources/switch-transformer-2021` | `sources/moe-survey-2023` |

---

_2 orphan(s), 1 missing stub(s), 1 active contradiction(s)._
```

Empty sections show `_None._` rather than an empty table.

---

## Commit message

```
lint: 2026-04-13 — 2 orphans, 1 stubs, 1 active contradictions
```

The em-dash (—) is a Unicode `U+2014`, not a double hyphen.

---

## How an external LLM should use `LINT.md`

1. **Orphans** — create cross-references from related pages, or decide the page
   should be deleted and re-ingest with no `suggested_pages` for that slug.
2. **Missing stubs** — run `wiki context "<stub name>"` to see if related content
   exists under a different slug; if not, produce analysis for that stub topic and
   ingest it.
3. **Active contradictions** — read both source pages (`wiki context`), analyse
   the dimension and epistemic value, produce an `analysis.json` with the enriched
   contradiction (status: resolved, resolution populated), and re-ingest. The
   resolved contradiction remains in `contradictions/` forever — the resolution
   *is* the knowledge.

`LINT.md` is itself committed to git, so every lint pass is auditable. `git log
LINT.md` shows how the structural health of the wiki has changed over time.
