# Phase 3 — Graph + Lint + Contradiction Surfacing

Goal: structural quality signals. Contradiction files written since Phase 1 become queryable.
`wiki lint` produces an actionable report committed to git.

---

## `graph.rs`

- [x] Parse `[[wikilinks]]` syntax from page bodies using `comrak`
- [x] Parse `related_concepts` and `contradictions` frontmatter fields as edges
- [x] `build_graph(wiki_root: &Path) -> Result<WikiGraph>` — `petgraph::DiGraph<String, EdgeKind>`
- [x] `EdgeKind`: `WikiLink`, `RelatedConcept`, `Contradiction`
- [x] `orphans(graph: &WikiGraph) -> Vec<String>` — nodes with in-degree = 0, excluding `raw/`
- [x] `missing_stubs(graph: &WikiGraph, wiki_root: &Path) -> Vec<String>` — edges pointing to non-existent files
- [x] `dot_output(graph: &WikiGraph) -> String` — GraphViz DOT
- [x] `mermaid_output(graph: &WikiGraph) -> String` — Mermaid graph syntax

## `contradiction.rs`

- [x] `list(wiki_root: &Path, status: Option<Status>) -> Result<Vec<ContradictionSummary>>` — walk `contradictions/`, parse frontmatter, filter by status
- [x] `ContradictionSummary` — fields: `slug`, `title`, `dimension`, `status`, `source_a`, `source_b`
- [x] `cluster(graph: &WikiGraph, slugs: &[String]) -> Vec<String>` — concept pages connected to given contradiction slugs

## `lint.rs`

- [x] `lint(wiki_root: &Path) -> Result<LintReport>`
  - collect orphan pages
  - collect missing stubs
  - collect active contradiction pages (`status: active | under-analysis`)
- [x] `write_lint_report(wiki_root: &Path, report: &LintReport) -> Result<()>` — write `LINT.md` with structured sections
- [x] `LINT.md` format:
  - `## Orphans` — pages with no inbound links
  - `## Missing Stubs` — referenced but non-existent slugs
  - `## Active Contradictions` — table: slug, title, dimension, sources
- [x] Commit `LINT.md` via `git::commit` — message `"lint: <date> — M orphans, K stubs, N active contradictions"`

## CLI

- [x] `wiki lint` — run lint, write and commit `LINT.md`, print summary
- [x] `wiki contradict` — list all contradiction pages (table: slug, title, status, dimension)
- [x] `wiki contradict --status active|resolved|under-analysis` — filter by status
- [x] `wiki list` — list all pages (table: slug, title, type)
- [x] `wiki list --type concept|source|contradiction|query` — filter by type
- [x] `wiki graph` — print DOT to stdout
- [x] `wiki graph --format mermaid` — print Mermaid to stdout
- [x] `wiki diff` — `git diff HEAD~1` wrapper, scoped to wiki root

## Tests

**Test file:** `tests/graph.rs`

### Unit tests

- [x] `graph::orphans` — page with no inbound links appears in orphans
- [x] `graph::orphans` — page referenced by another page does not appear in orphans
- [x] `graph::orphans` — pages in `raw/` excluded from orphan detection
- [x] `graph::missing_stubs` — edge to non-existent file appears in missing stubs
- [x] `graph::missing_stubs` — edge to existing file does not appear
- [x] `graph::dot_output` — output contains `digraph` keyword, no empty node names
- [x] `graph::mermaid_output` — output starts with `graph`
- [x] `contradiction::list` — returns all contradiction pages when no status filter
- [x] `contradiction::list` — `status: active` filter returns only active contradictions
- [x] `contradiction::list` — `status: resolved` filter excludes active ones
- [x] `lint` — orphan page appears in `LintReport.orphans`
- [x] `lint` — missing stub appears in `LintReport.missing_stubs`
- [x] `lint` — active contradiction appears in `LintReport.active_contradictions`
- [x] `lint` — resolved contradiction does not appear in active list

### Integration tests

- [x] `wiki lint` on a wiki with one orphan page → `LINT.md` contains orphan slug, git commit present
- [x] `wiki lint` on a clean wiki (no issues) → `LINT.md` written with empty sections, still commits
- [x] `wiki contradict --status active` → lists only active contradiction pages
- [x] `wiki list --type concept` → lists only concept pages, no source or contradiction pages
- [x] `wiki graph` → output parses as valid DOT (run through `dot -Tsvg` in test)
- [x] `wiki diff` → output is non-empty after an ingest

## Changelog

- [x] `CHANGELOG.md` — add Phase 3 section: `wiki lint`, `wiki contradict`, `wiki graph`, `wiki list`, `wiki diff`, contradiction surfacing

## README

- [x] CLI reference — add `wiki lint`, `wiki contradict`, `wiki graph`, `wiki list`, `wiki diff`
- [x] **Contradictions** section — brief explanation of first-class contradiction pages, link to design doc

## Dev documentation

- [x] `docs/dev/graph.md` — graph node/edge model, orphan detection rules, DOT + Mermaid output
- [x] `docs/dev/lint.md` — `LINT.md` structure, what each section means, how external LLM should use it
- [x] `docs/dev/contradictions.md` — contradiction page lifecycle: written at ingest, surfaced at lint, enriched by external LLM, re-ingested
- [x] Update `docs/dev/architecture.md` — mark Phase 3 modules as implemented
