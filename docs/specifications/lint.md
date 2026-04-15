---
title: "Lint"
summary: "Structural audit of the wiki — orphan pages, missing stubs, and empty sections. Produces a LintReport and commits LINT.md."
read_when:
  - Implementing or extending the lint pipeline
  - Understanding what wiki lint checks and reports
  - Integrating wiki lint into an LLM maintenance workflow
status: draft
last_updated: "2025-07-15"
---

# Lint

`wiki lint` is a structural audit. It walks the wiki, checks three things, and
produces a `LintReport`. The report is written to `LINT.md` and committed. The
wiki binary makes no content judgments — it surfaces structural problems and
hands them to the LLM.

---

## 1. What Lint Checks

Three structural checks, always run. See [lint.md](lint.md) for the full spec.

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

---

## 2. Return Type — `LintReport`

```rust
pub struct LintReport {
    pub orphans:        Vec<PageRef>,
    pub missing_stubs:  Vec<String>,   // slugs referenced but not existing
    pub empty_sections: Vec<String>,   // slugs of sections missing index.md
    pub date:           String,        // ISO date of the lint run
}
```

`PageRef` is the unified type from [search.md](search.md) — slug, uri, path,
title, score. Score is always `0.0` for lint results (not a search ranking).

---

## 3. `LINT.md` Format Specification

`wiki lint` overwrites `LINT.md` at the wiki root and commits it. Git history
is the archive — no previous report is preserved in the file itself.

### Structure

Three sections always present, even when empty. Empty sections show an explicit
`none` notice so the reader knows the check ran and found nothing.

```
# Lint Report — {ISO date}

## Orphans ({count})

{table or none notice}

## Missing Stubs ({count})

{table or none notice}

## Empty Sections ({count})

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

### Active Contradictions table

Contradiction pages with `status: active` or `status: under-analysis`.
`uri` and `path` included for direct navigation.

```markdown
## Active Contradictions (2)

| slug | title | uri | path |
|------|-------|-----|------|
| contradictions/moe-scaling-efficiency | MoE scaling efficiency: contradictory views | wiki://research/contradictions/moe-scaling-efficiency | /wikis/research/contradictions/moe-scaling-efficiency.md |
| contradictions/attention-complexity | Attention complexity: contradictory views | wiki://research/contradictions/attention-complexity | /wikis/research/contradictions/attention-complexity.md |
```

When empty:

```markdown
## Active Contradictions (0)

_No active contradictions found._
```

Git commit: `lint: <date> — N orphans, M stubs, K empty sections`

`LINT.md` is a generated operational artifact — it must not have frontmatter
and is excluded from tantivy indexing, orphan detection, and graph traversal.

---

## 4. What Lint Checks

Three checks, always run:

| Check | Auto-fixable |
|-------|--------------|
| Orphan pages | No — requires content judgment |
| Missing stubs | Yes — `wiki new page <slug>` |
| Empty sections | Yes — `wiki new section <slug>` |

---

## 5. CLI Interface

```
wiki lint                          # audit + write LINT.md
wiki lint fix                      # run all enabled auto-fixes (from config)
             [--only <check>]      # missing-stubs | empty-sections
             [--dry-run]           # show what would be fixed, no commit
             [--wiki <name>]
```

`wiki lint fix` reads `[lint]` config to determine which fixes are enabled.
CLI flags override config per-call.

Git commit for fix: `lint(fix): <date> — +N stubs, +M sections`

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

The lint report is the input to the `lint_and_enrich` MCP prompt:

```
1. wiki_lint                          → LintReport
2. LLM creates missing stub pages via wiki_new page <slug>
3. LLM creates empty section index pages via wiki_new section <slug>
4. LLM links orphan pages from related concept pages
```

The wiki never auto-resolves anything — all decisions are delegated to the LLM.

---

## 8. Rust Module Changes

| Module | Change |
|--------|--------|
| `lint.rs` | Define `LintReport`; implement orphan detection, stub detection, empty section scan |
| `graph.rs` | Expose `in_degree(slug)` for orphan detection |
| `markdown.rs` | Expose `extract_links(page)` for stub detection |
| `cli.rs` | Add `fix` subcommand with `--only`, `--dry-run` to `lint` |
| `server.rs` | Update `wiki_lint` return type to `LintReport` |

---

## 9. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki lint` structural audit | implemented (partial) |
| `LintReport` struct | **not implemented** |
| Orphan detection | implemented |
| Missing stub detection | **not implemented** |
| Empty section detection | **not implemented** |
| `LINT.md` commit | implemented |
| `wiki lint fix` | **not implemented** |
| `wiki_lint` returning `LintReport` | **not implemented** |
