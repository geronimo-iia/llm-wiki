---
title: "Lint"
summary: "Structural audit of the wiki — orphan pages, missing stubs, empty sections, missing connections, and untyped sources. Produces a LintReport and writes LINT.md."
read_when:
  - Implementing or extending the lint pipeline
  - Understanding what llm-wiki lint checks and reports
  - Integrating llm-wiki lint into an LLM maintenance workflow
status: draft
last_updated: "2025-07-15"
---

# Lint

`llm-wiki lint` is a structural audit. It walks the wiki, checks five things, and
produces a `LintReport`. The report is written to `LINT.md`. The
wiki binary makes no content judgments — it surfaces structural problems and
hands them to the LLM.

---

## 1. What Lint Checks

Five structural checks, always run.

### Orphans

Pages with no incoming links — in-degree 0 in the petgraph concept graph.
An orphan is not necessarily wrong, but it is invisible to navigation and
context retrieval.

### Missing stubs

Slugs referenced in frontmatter (`sources`, `concepts`) or page body links
that do not exist as pages. A missing stub means the wiki has a broken reference.

### Empty sections

Directories that exist but have no `index.md`. An empty section is invisible
to search and navigation.

### Missing connections

Page pairs with significant term overlap in frontmatter and body but no mutual
links. Flagged as candidates — not every pair should be linked. See
[backlink-quality.md](../llm/backlink-quality.md) for the quality test.

### Untyped sources

Pages that appear to be source summaries but have a missing or deprecated
`source-summary` type. See [source-classification.md](../core/source-classification.md).

---

## 2. Return Type — `LintReport`

```rust
pub struct MissingConnection {
    pub slug_a:            String,
    pub slug_b:            String,
    pub overlapping_terms: Vec<String>,
}

pub struct LintReport {
    pub orphans:             Vec<PageRef>,
    pub missing_stubs:       Vec<String>,            // slugs referenced but not existing
    pub empty_sections:      Vec<String>,            // slugs of sections missing index.md
    pub missing_connections: Vec<MissingConnection>, // candidate pairs with term overlap
    pub untyped_sources:     Vec<String>,            // slugs with missing/deprecated type
    pub date:                String,                 // ISO date of the lint run
}
```

`PageRef` is the unified type from [search.md](search.md) — slug, uri, title,
score. Score is always `0.0` for lint results (not a search ranking).

---

## 3. `LINT.md` Format Specification

`llm-wiki lint` overwrites `LINT.md` at the repository root. Git history
is the archive — no previous report is preserved in the file itself.

### Structure

Five sections always present, even when empty. Empty sections show an explicit
`none` notice so the reader knows the check ran and found nothing.

```
# Lint Report — {ISO date}

## Orphans ({count})

{table or none notice}

## Missing Stubs ({count})

{table or none notice}

## Empty Sections ({count})

{table or none notice}

## Missing Connections ({count})

{table or none notice}

## Untyped Sources ({count})

{table or none notice}
```

### Orphans table

Pages that exist but have no incoming links. `uri` and `path` included for
direct navigation from the report.

```markdown
## Orphans (3)

| slug | title | uri | path |
|------|-------|-----|------|
| concepts/sparse-attention | Sparse Attention | wiki://research/concepts/sparse-attention | /wikis/research/concepts/sparse-attention.md |
| sources/llama-2023 | LLaMA (2023) | wiki://research/sources/llama-2023 | /wikis/research/sources/llama-2023.md |
| queries/moe-efficiency-2024 | MoE efficiency — synthesis | wiki://research/queries/moe-efficiency-2024 | /wikis/research/queries/moe-efficiency-2024.md |
```

When empty:

```markdown
## Orphans (0)

_No orphans found._
```

### Missing Stubs table

Slugs referenced in frontmatter or body links that do not exist as pages.
No `uri` or `path` — the page does not exist yet.

```markdown
## Missing Stubs (2)

| slug |
|------|
| concepts/flash-attention |
| sources/chinchilla-2022 |
```

When empty:

```markdown
## Missing Stubs (0)

_No missing stubs found._
```

### Empty Sections table

Directories that exist but have no `index.md`. No `uri` or `path` since there
is no file yet.

```markdown
## Empty Sections (1)

| slug |
|------|
| skills/experimental |
```

When empty:

```markdown
## Empty Sections (0)

_No empty sections found._
```

### Missing Connections table

Candidate page pairs with significant term overlap but no mutual links.

```markdown
## Missing Connections (1)

| page_a | page_b | shared terms |
|--------|--------|--------------|
| concepts/mixture-of-experts | concepts/scaling-laws | MoE, compute efficiency, parameter count |
```

When empty:

```markdown
## Missing Connections (0)

_No missing connections found._
```

_Candidates only — evaluate each pair against the [backlink quality test](../llm/backlink-quality.md) before linking._

### Untyped Sources table

Source pages with a missing or deprecated `source-summary` type.

```markdown
## Untyped Sources (2)

| slug | current type |
|------|-------------|
| sources/random-blog-post | (missing) |
| sources/meeting-notes-2025-03 | source-summary |
```

When empty:

```markdown
## Untyped Sources (0)

_No untyped sources found._
```

Git message (when committed via `llm-wiki commit`): `lint: <date> — N orphans, M stubs, K empty sections`

`LINT.md` is a generated operational artifact — it must not have frontmatter
and is excluded from tantivy indexing, orphan detection, and graph traversal.

---

## 4. What Lint Checks

Five checks, always run:

| Check | Auto-fixable |
|-------|--------------|
| Orphan pages | No — requires content judgment |
| Missing stubs | Yes — `llm-wiki new page <slug>` |
| Empty sections | Yes — `llm-wiki new section <slug>` |
| Missing connections | No — requires quality judgment |
| Untyped sources | No — requires type assignment |

---

## 5. CLI Interface

```
llm-wiki lint                          # audit + write LINT.md
llm-wiki lint fix                      # run all enabled auto-fixes (from config)
             [--only <check>]      # missing-stubs | empty-sections
             [--dry-run]           # show what would be fixed
             [--wiki <name>]
```

`llm-wiki lint fix` reads `[lint]` config to determine which fixes are enabled.
CLI flags override config per-call.

Neither `lint` nor `lint fix` commits. Use `llm-wiki commit` after reviewing.

---

## 6. MCP Tool

```rust
#[tool(description = "Run a structural lint pass — orphans, missing stubs, empty sections")]
async fn wiki_lint(
    &self,
    #[tool(param)] wiki: Option<String>,
    #[tool(param)] dry_run: Option<bool>,
) -> LintReport { ... }
```

---

## 7. LLM Workflow

`llm-wiki lint` surfaces structural problems — it never auto-resolves them. The
LLM reads the `LintReport` and decides what to act on:

- **Orphans** — search for related concept pages and add links, or decide the
  page is intentionally standalone
- **Missing stubs** — create scaffold pages via `wiki_new_page` for references
  that should exist
- **Empty sections** — create `index.md` via `wiki_new_section` for directories
  that need one
- **Missing connections** — evaluate each candidate pair against the
  [backlink quality test](../llm/backlink-quality.md) before adding any link
- **Untyped sources** — assign the correct source type (`paper`, `article`,
  `documentation`, etc.) and re-ingest

All decisions are delegated to the LLM. The llm-wiki instruct `lint` workflow
guides the LLM through this sequence step by step.
