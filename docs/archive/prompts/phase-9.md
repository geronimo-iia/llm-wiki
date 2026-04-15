# Phase 9 — Direct Ingest + Enrichment Contract

You are implementing Phase 9 (projects/llm-wiki/docs/tasks/phase-9.md) of llm-wiki (projects/llm-wiki). Phase 8 (bundle support) is complete.

## Context

The full task list is in `docs/tasks/phase-9.md`.
Design refs: `docs/design/ingest.md`, `docs/design/design-evolution.md`,
`docs/design/page-content.md`.

## The Goal

This is a **breaking change** phase. The old `analysis.json` contract
(LLM writes page bodies via `SuggestedPage`) is replaced by:

1. **Direct ingest** — `wiki ingest <path>` ingests files/folders with no LLM
2. **Enrichment** — LLM annotates existing pages via `enrichment.json`
   (frontmatter only, body never touched)

`SuggestedPage`, `Action`, `DocType`, `PageType` are removed from `analysis.rs`.

## What to implement

### `src/analysis.rs` — rebuild the contract

Remove: `SuggestedPage`, `Action`, `DocType`, `PageType`
Keep: `Claim`, `Confidence`, `Contradiction`, `Dimension`, `Status`

Add:
```rust
pub struct Enrichment {
    pub slug: String,
    pub claims: Vec<Claim>,
    pub concepts: Vec<String>,
    pub tags: Vec<String>,
    pub read_when: Vec<String>,
    pub confidence: Option<Confidence>,
    pub sources: Vec<String>,
}

pub struct QueryResult {
    pub slug: String,
    pub title: String,
    pub tldr: String,
    pub body: String,
    pub tags: Vec<String>,
    pub read_when: Vec<String>,
    pub sources: Vec<String>,
}

pub struct Asset { /* slug, filename, kind: Option<AssetKind>, content_encoding,
                      content, caption, referenced_by */ }
pub enum AssetKind { Image, Yaml, Toml, Json, Script, Data, Other }
pub enum ContentEncoding { Utf8, Base64 }

pub struct Analysis {
    pub source: String,
    pub enrichments: Vec<Enrichment>,
    pub query_results: Vec<QueryResult>,
    pub contradictions: Vec<Contradiction>,
    pub assets: Vec<Asset>,
}
```

### `src/ingest.rs` — new input model

```rust
pub enum Input {
    Direct(PathBuf),       // file or folder — default
    AnalysisOnly(PathBuf), // --analysis-only — legacy
}

pub struct DirectIngestOptions {
    pub prefix: Option<String>,
    pub update: bool,
    pub analysis: Option<PathBuf>, // optional enrichment JSON
}
```

### `src/integrate.rs` — new functions, remove old

Add:
- `validate_slug_direct(slug)` — rejects traversal/absolute only, any prefix ok
- `integrate_direct_file(path, slug, wiki_root, update)` — preserve or generate frontmatter, write page
- `integrate_direct_folder(folder, prefix, wiki_root, update)` — walk, call integrate_direct_file + write_asset_colocated
- `integrate_enrichment(enrichment, wiki_root)` — merge fields into frontmatter, body untouched
- `integrate_query_result(qr, wiki_root)` — generate frontmatter, write body
- `integrate_analysis(analysis, wiki_root)` — apply all enrichments/query_results/contradictions/assets

Remove: old `integrate()`, `Action`-based dispatch, `VALID_PREFIXES`

Frontmatter merge rules for `integrate_enrichment`:
- UNION: tags, read_when, sources, concepts
- APPEND: claims
- SET if provided: confidence, last_updated
- PRESERVE: title, summary, tldr, status, contradictions, type

### `src/markdown.rs` — three new functions

```
generate_minimal_frontmatter(title, slug) -> PageFrontmatter
  — title from H1 or filename stem, status: active, last_updated: today

extract_h1(body) -> Option<String>
  — find first # Heading in body

merge_enrichment(fm: &mut PageFrontmatter, e: &Enrichment)
  — apply union/append/set rules above
```

### `src/cli.rs` — ingest command overhaul

Primary form: `wiki ingest <path> [--prefix] [--update] [--analysis <file>] [--dry-run]`
Legacy form: `wiki ingest --analysis-only <file>`
Remove: old `wiki ingest <file|->` as primary

### `src/server.rs` — tool rename

Rename `wiki_ingest` → `wiki_ingest_analysis` (legacy).
Add new `wiki_ingest(path, prefix, update, analysis, wiki)` as primary tool.

## Tests

New file `tests/direct_ingest.rs`, extend `tests/ingest.rs`:

- `Enrichment`, `QueryResult`, `Asset` round-trip JSON
- `Analysis` with empty arrays → valid
- `validate_slug_direct` — user prefix ok, traversal rejected
- `integrate_direct_file` — frontmatter preserved / generated
- `integrate_direct_folder` — slug derivation with --prefix
- `integrate_enrichment` — tags unioned, body unchanged, slug-not-found error
- `integrate_query_result` — correct frontmatter, slug-exists error
- Integration: `wiki ingest SKILL.md --prefix skills` → on disk + committed
- Integration: `wiki ingest folder/ --analysis enrichment.json` → files then enrichment
- Integration: `wiki ingest --dry-run` → nothing written

## Acceptance

```bash
cargo test
wiki ingest agent-skills/semantic-commit/ --prefix skills
# → pages + co-located assets, no LLM needed
wiki ingest --analysis-only enrichment.json
# → enrichments applied, query results written
```

## Constraints

- No LLM dependency
- `SuggestedPage`, `Action`, `DocType`, `PageType` must not exist after this phase
- Document breaking changes clearly in `CHANGELOG.md`
- Update `docs/dev/ingest.md` and create `docs/dev/enrichment.md`
