# Phase 0 — Skeleton

Goal: everything compiles, CI is green, schema structs are locked.
No logic — only typed signatures and `todo!()` stubs.

---

## Schema structs

- [x] `src/analysis.rs` — define `Analysis`, `SuggestedPage`, `Contradiction`, `Claim` with full serde derives
- [x] `src/markdown.rs` — define `PageFrontmatter` with serde_yaml derives
- [x] Verify `Analysis` → JSON → `Analysis` round-trip compiles (no logic, just types)
- [x] Verify `PageFrontmatter` → YAML → `PageFrontmatter` round-trip compiles

## Config

- [x] `src/config.rs` — define `WikiConfig` struct (`root`, `name`), deserialize from `.wiki/config.toml`

## Module stubs (typed signatures, `todo!()` bodies)

- [x] `src/ingest.rs` — `pub async fn ingest(input: Input, config: &WikiConfig) -> Result<IngestReport>`
- [x] `src/integrate.rs` — `pub fn integrate(analysis: Analysis, wiki_root: &Path) -> Result<IngestReport>`
- [x] `src/search.rs` — `pub fn search(query: &str, wiki_root: &Path) -> Result<Vec<SearchResult>>`
- [x] `src/context.rs` — `pub fn context(question: &str, wiki_root: &Path, top_k: usize) -> Result<String>`
- [x] `src/lint.rs` — `pub fn lint(wiki_root: &Path) -> Result<LintReport>`
- [x] `src/graph.rs` — `pub fn build_graph(wiki_root: &Path) -> Result<WikiGraph>`
- [x] `src/contradiction.rs` — `pub fn list(wiki_root: &Path, status: Option<Status>) -> Result<Vec<ContradictionSummary>>`
- [x] `src/git.rs` — `pub fn commit_all(root: &Path, message: &str) -> Result<()>`
- [x] `src/server.rs` — `pub struct WikiServer` stub
- [x] `src/registry.rs` — `pub struct WikiRegistry` stub (Phase 6 placeholder)

## CLI

- [x] `src/cli.rs` — define all `Command` variants with typed args (clap `derive`):
  `Ingest`, `Search`, `Context`, `Lint`, `List`, `Contradict`, `Graph`, `Diff`, `Serve`, `Instruct`
- [x] `src/main.rs` — dispatch to stubs, compile cleanly

## Build

- [x] `Cargo.toml` — all dependencies present, no `rig-core`:
  `clap`, `anyhow`, `serde`, `serde_json`, `serde_yaml`, `toml`, `comrak`,
  `walkdir`, `git2`, `tantivy`, `petgraph`, `rmcp`, `tokio`, `async-trait`, `tracing`,
  `schemars`
- [x] `rustfmt.toml` — formatting rules set
- [x] `clippy.toml` — lint rules set

## CI

- [x] `.github/workflows/ci.yml` — jobs: `cargo check`, `cargo clippy --deny warnings`, `cargo test`
- [ ] Verify CI passes on a clean checkout

## Tests

**Test file:** `tests/schema.rs`

- [x] `tests/integration_test.rs` — placeholder, compiles and passes
- [x] Unit test: `Analysis` serde round-trip — serialize to JSON, deserialize back, fields match
- [x] Unit test: `PageFrontmatter` serde round-trip — serialize to YAML, deserialize back, fields match
- [x] Unit test: `WikiConfig` load from minimal TOML string

## Project infrastructure (see `docs/tasks/project.md` for full detail)

- [x] `CONTRIBUTING.md` — build, test, lint, commit format, no-LLM-dependency rule
- [x] `.github/ISSUE_TEMPLATE/bug_report.md`
- [x] `.github/ISSUE_TEMPLATE/feature_request.md`
- [x] `.github/ISSUE_TEMPLATE/config.yml` — disable blank issues
- [x] `.github/dependabot.yml` — cargo + github-actions, weekly, group patches
- [x] `CHANGELOG.md` — establish Keep a Changelog format, add `[Unreleased]` section

## Dev documentation

- [x] `docs/dev/architecture.md` — module map, dependency graph, design principles (no-LLM contract)
- [x] Inline doc comments on `Analysis`, `SuggestedPage`, `Contradiction`, `PageFrontmatter` fields
