# Phase 9 — Direct Ingest + Enrichment Contract

Goal: `wiki ingest <path>` works for files and folders without an LLM step.
`analysis.json` is replaced by `enrichment.json` — frontmatter enrichment only,
no page body authoring. `SuggestedPage` and `Action` are removed.

Depends on: Phase 8 complete (bundle support required for folder ingest).
Design refs: [ingest.md](../design/ingest.md),
[asset-ingest.md](../design/asset-ingest.md),
[design-evolution.md](../design/design-evolution.md).

---

## `analysis.rs`

- [ ] Add `Enrichment` struct:
  ```rust
  pub struct Enrichment {
      pub slug:       String,
      pub claims:     Vec<Claim>,
      pub concepts:   Vec<String>,
      pub tags:       Vec<String>,
      pub read_when:  Vec<String>,
      pub confidence: Option<Confidence>,
      pub sources:    Vec<String>,
  }
  ```
- [ ] Add `QueryResult` struct:
  ```rust
  pub struct QueryResult {
      pub slug:      String,
      pub title:     String,
      pub tldr:      String,
      pub body:      String,
      pub tags:      Vec<String>,
      pub read_when: Vec<String>,
      pub sources:   Vec<String>,
  }
  ```
- [ ] Add `Asset` struct (from `asset-ingest.md § 7`):
  `slug`, `filename`, `kind: Option<AssetKind>`, `content_encoding`,
  `content`, `caption`, `referenced_by`
- [ ] Add `AssetKind` enum: `Image`, `Yaml`, `Toml`, `Json`, `Script`, `Data`, `Other`
- [ ] Add `ContentEncoding` enum: `Utf8`, `Base64`
- [ ] Replace `Analysis` struct — new fields:
  ```rust
  pub struct Analysis {
      pub source:        String,
      pub enrichments:   Vec<Enrichment>,
      pub query_results: Vec<QueryResult>,
      pub contradictions: Vec<Contradiction>,
      pub assets:        Vec<Asset>,
  }
  ```
- [ ] Remove `SuggestedPage`, `Action`, `DocType`, `PageType` — breaking change,
  document in CHANGELOG
- [ ] Keep `Claim`, `Confidence`, `Contradiction`, `Dimension`, `Status` unchanged

## `ingest.rs`

- [ ] `Input` enum:
  ```rust
  pub enum Input {
      Direct(PathBuf),          // file or folder — default
      AnalysisOnly(PathBuf),    // --analysis-only flag — legacy
  }
  ```
- [ ] `DirectIngestOptions`:
  ```rust
  pub struct DirectIngestOptions {
      pub prefix:   Option<String>,
      pub update:   bool,
      pub analysis: Option<PathBuf>,  // optional enrichment JSON
  }
  ```
- [ ] `ingest(input: Input, opts: DirectIngestOptions, wiki_root: &Path) -> Result<IngestReport>`
  — dispatch to `integrate_direct_file`, `integrate_direct_folder`, or
  `integrate_analysis` based on input variant and path type
- [ ] `parse_enrichment(json: &str) -> Result<Analysis>` — deserialize new schema
- [ ] Validate `enrichment.slug` exists in wiki before applying (error if not)
- [ ] Validate `query_result.slug` does not exist (error if already present,
  unless `--update`)

## `integrate.rs`

- [ ] `validate_slug_direct(slug: &str) -> Result<()>`  ← new
  — relaxed: rejects path traversal and absolute paths only
  — any prefix allowed (user-defined prefixes from `--prefix` are valid)
  — used by `integrate_direct_file` and `integrate_direct_folder`
- [ ] Remove `validate_slug_analysis` — enrichment slugs are validated by
  existence check (`enrichment.slug` must exist in wiki); query result slugs
  are validated by non-existence check; no prefix enforcement needed
- [ ] Remove `VALID_PREFIXES` constant — no longer needed
- [ ] `integrate_direct_file(path: &Path, slug: &str, wiki_root: &Path, update: bool) -> Result<PageAction>`
  — read file, preserve frontmatter if present, generate minimal if absent
  — write to `{slug}.md` or `{slug}/index.md`
- [ ] `integrate_direct_folder(folder: &Path, prefix: Option<&str>, wiki_root: &Path, update: bool) -> Result<IngestReport>`
  — walk folder recursively
  — `.md` files → `integrate_direct_file`
  — non-`.md` files → `write_asset_colocated` (from Phase 8)
  — derive slug from path relative to folder root + optional prefix
- [ ] `integrate_enrichment(enrichment: &Enrichment, wiki_root: &Path) -> Result<()>`
  — read existing page frontmatter
  — union `tags`, `read_when`, `sources`, `concepts`
  — set `confidence` if provided
  — append `claims` to frontmatter `claims` list
  — set `last_updated` to today
  — write back (body untouched)
- [ ] `integrate_query_result(qr: &QueryResult, wiki_root: &Path) -> Result<()>`
  — generate frontmatter from `QueryResult` fields
  — write `{slug}.md` with body
  — uses `validate_slug_analysis` (query results must be under `queries/`)
- [ ] `integrate_analysis(analysis: &Analysis, wiki_root: &Path) -> Result<IngestReport>`
  — apply all `enrichments`, `query_results`, `contradictions`, `assets`
  — replaces old `integrate` function
  — call `search::update_index(wiki_root, &changed_slugs)` after writing pages
- [ ] Remove old `integrate` function and `Action`-based dispatch
- [ ] `IngestReport` — add `assets_written`, `bundles_created`, `enrichments_applied`,
  `query_results_written`; remove `pages_created`, `pages_updated`, `pages_appended`

## `markdown.rs`

- [ ] `generate_minimal_frontmatter(title: &str, slug: &str) -> PageFrontmatter`
  — `title` from H1 heading or filename stem, `status: active`, `last_updated: today`
- [ ] `extract_h1(body: &str) -> Option<String>` — find first `# Heading` in body
- [ ] `merge_enrichment(fm: &mut PageFrontmatter, e: &Enrichment)`
  — union tags, read_when, sources; set confidence; append claims

## `cli.rs`

- [ ] `wiki ingest <path>` — primary form (file or folder)
  — flags: `--prefix <slug>`, `--update`, `--append`, `--analysis <file>`, `--dry-run`
- [ ] `wiki ingest --analysis-only <file>` — legacy form
- [ ] `wiki ingest --dry-run` — print what would be written, no commit, no disk writes
- [ ] Remove old `wiki ingest <file|->` (analysis-JSON-only) as primary form

## `server.rs`

- [ ] Rename `wiki_ingest` → `wiki_ingest_analysis` (legacy, keep for compatibility)
- [ ] Add new `wiki_ingest` tool:
  ```rust
  async fn wiki_ingest(
      path: String,
      prefix: Option<String>,
      update: Option<bool>,
      analysis: Option<serde_json::Value>,
      wiki: Option<String>,
  ) -> IngestReport
  ```
- [ ] `wiki_ingest_analysis` — accepts new `Analysis` schema (enrichments + query_results)

## Tests

**Test files:** `tests/ingest.rs` (extend), new `tests/direct_ingest.rs`

### Unit tests — `analysis.rs`

- [ ] `Enrichment` round-trip JSON serialization
- [ ] `QueryResult` round-trip JSON serialization
- [ ] `Asset` round-trip — base64 content preserved
- [ ] `Analysis` with empty `enrichments` and `query_results` → valid
- [ ] `parse_enrichment` — unknown field → error (strict validation)
- [ ] `parse_enrichment` — missing `source` → error

### Unit tests — `integrate.rs`

  — `validate_slug_analysis` tests covered in Phase 8
- [ ] `validate_slug_direct` — `skills/foo` → ok (user-defined prefix)
- [ ] `validate_slug_direct` — `guides/python/style` → ok
- [ ] `validate_slug_direct` — `../evil` → error (path traversal)
- [ ] `validate_slug_direct` — `/absolute/path` → error (absolute path)

- [ ] `integrate_direct_file` — file with frontmatter → frontmatter preserved
- [ ] `integrate_direct_file` — file without frontmatter → minimal generated
- [ ] `integrate_direct_file` — H1 heading used as title when no frontmatter
- [ ] `integrate_direct_folder` — `.md` files written, non-`.md` co-located
- [ ] `integrate_direct_folder` — slug derivation with `--prefix skills`
- [ ] `integrate_enrichment` — tags unioned, body unchanged
- [ ] `integrate_enrichment` — slug not found → error
- [ ] `integrate_query_result` — page written with correct frontmatter
- [ ] `integrate_query_result` — slug already exists without `--update` → error

### Integration tests

- [ ] `wiki ingest SKILL.md --prefix skills` → `skills/skill.md` on disk, git commit
- [ ] `wiki ingest agent-skills/semantic-commit/ --prefix skills`
  → `skills/semantic-commit/index.md` + co-located assets, one commit
- [ ] `wiki ingest folder/ --analysis enrichment.json`
  → files written first, then enrichment applied, one commit
- [ ] `wiki ingest --analysis-only enrichment.json`
  → enrichments applied to existing pages, query results written
- [ ] `wiki ingest --dry-run folder/` → nothing written, report printed
- [ ] `wiki ingest` with `enrichment.slug` pointing to non-existent page → error,
  no partial writes

## Changelog

- [ ] `CHANGELOG.md` — Phase 9: direct ingest, enrichment contract,
  `SuggestedPage`/`Action`/`DocType` removal (breaking), new `Analysis` schema,
  `IngestReport` field changes

## README

- [ ] **Quick start** — update to `wiki ingest <path>` as primary form
- [ ] **`enrichment.json` contract** — replace `analysis.json` section with new schema
- [ ] CLI reference — update `wiki ingest` entry, add `--analysis`, `--dry-run` flags

## Dev documentation

- [ ] `docs/dev/ingest.md` — update pipeline walkthrough for three modes
- [ ] `docs/dev/enrichment.md` — enrichment contract, field merge rules,
  query_result authoring
- [ ] Update `docs/dev/architecture.md` — mark Phase 8 modules updated
