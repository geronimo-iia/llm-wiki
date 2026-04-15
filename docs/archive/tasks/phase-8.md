# Phase 8 — Repository Layout + Bundle Support

Goal: the wiki supports both flat pages and bundle folders (page + co-located
assets). Slug resolution handles both forms transparently. All walkers updated.

Depends on: Phase 7 complete (incremental index update required for bundle
page indexing).
Design refs: [repository-layout.md](../design/repository-layout.md),
[asset-ingest.md](../design/asset-ingest.md).

**Status: complete.**

---

## `markdown.rs`

- [x] `slug_for(path: &Path, wiki_root: &Path) -> String`
  — if filename is `index.md` → slug = parent dir relative to wiki root
  — otherwise → slug = path without extension relative to wiki root
- [x] `resolve_slug(wiki_root: &Path, slug: &str) -> Option<PathBuf>`
  — check `{slug}.md` first, then `{slug}/index.md`
  — return `None` if neither exists
- [x] `promote_to_bundle(wiki_root: &Path, slug: &str) -> Result<()>`
  — move `{slug}.md` → `{slug}/index.md`, creating the directory
  — no-op if already a bundle
- [x] `is_bundle(wiki_root: &Path, slug: &str) -> bool`
  — true if `{slug}/index.md` exists

## `integrate.rs`

- [x] `write_asset_colocated(wiki_root, page_slug, filename, content) -> Result<()>`
  — promote page to bundle if currently flat
  — write asset to `{page_slug}/{filename}`
- [x] `write_asset_shared(wiki_root, kind, filename, content) -> Result<()>`
  — write to `assets/{subdir}/{filename}` per kind→subdir table
  — update `assets/index.md`
- [x] `assets_index_path(wiki_root) -> PathBuf` — `assets/index.md`
- [x] `regenerate_assets_index(wiki_root) -> Result<()>`
  — walk `assets/` (excluding `index.md`), rebuild table, write
- [x] `IngestReport` gains `bundles_created: usize`

## `search.rs`

- [x] Update `build_index` and `update_index` walkers — use `slug_for` instead
  of naive path stripping
- [x] Skip non-`index.md` files inside bundle folders (they are assets, not pages)
- [x] Skip `assets/` subtree except `assets/index.md` (index is a page, assets are not)
- [x] Add `path: String` (absolute) field to `SearchResult`

## `graph.rs`

- [x] Update `build_graph` walker — use `slug_for` for consistent slug derivation
- [x] Bundle assets not treated as pages in the graph
- [x] `missing_stubs` uses `resolve_slug` to check existence (flat or bundle)

## `context.rs`

- [x] Update page path resolution to use `resolve_slug`

## `lint.rs`

- [x] Update page walker to use `slug_for`
- [x] Add orphan asset reference check: page body references `./asset` that does
  not exist in the bundle folder → report in `LintReport.orphan_asset_refs`
- [x] `LintReport` gains `orphan_asset_refs: Vec<String>`

## `server.rs`

- [x] Update MCP resource resolution to use `resolve_slug`
- [x] Register bundle asset resources: `wiki://{wiki}/{slug}/{filename}`
  — read from `{wiki_root}/{slug}/{filename}`
- [x] `list_pages_in_root` uses `slug_for`; skips non-`index.md` bundle files

## `cli.rs`

- [x] `wiki read <slug>` — new subcommand: print full content of one page to stdout
  — resolves via `resolve_slug`, prints frontmatter + body
- [x] `wiki read <slug> --body-only` — body only, no frontmatter

## Tests

**Test files:** `tests/ingest.rs` (extended), `tests/search.rs` (extended),
`tests/graph.rs` (extended), `tests/mcp.rs` (extended)

### Unit tests

- [x] `slug_for` — flat file `concepts/foo.md` → `"concepts/foo"`
- [x] `slug_for` — bundle `concepts/foo/index.md` → `"concepts/foo"`
- [x] `resolve_slug` — flat file exists → returns `.md` path
- [x] `resolve_slug` — bundle exists → returns `index.md` path
- [x] `resolve_slug` — neither exists → `None`
- [x] `resolve_slug` — flat wins when both forms exist
- [x] `promote_to_bundle` — flat `.md` moved to `index.md`, directory created
- [x] `promote_to_bundle` — already bundle → no-op, no error
- [x] `is_bundle` — true for bundle, false for flat
- [x] `write_asset_colocated` — flat page promoted, asset written beside `index.md`
- [x] `write_asset_colocated` — bundle page → asset written, no promotion needed
- [x] `write_asset_shared` — written to correct `assets/{subdir}/` path (all kinds)
- [x] `regenerate_assets_index` — table contains all files under `assets/`
- [x] `search::build_index` — bundle page indexed once (not twice)
- [x] `search::build_index` — asset files not indexed as pages
- [x] `search::build_index` — `assets/` subtree not indexed
- [x] `lint` — orphan asset ref detected when bundle asset missing
- [x] `lint` — no orphan asset ref when asset exists
- [x] `graph::build_graph` — bundle page has correct slug (not `…/index`)
- [x] `graph::build_graph` — bundle asset not treated as a page node

### Integration tests

- [x] `wiki search` after bundle promotion — slug unchanged, still found
- [x] Ingest flat page, then add co-located asset → page promoted to bundle,
  `git log` shows at least 2 commits
- [x] `wiki read concepts/foo` — resolves flat page, prints content
- [x] `wiki read concepts/bundle-read` — resolves bundle, prints content
- [x] `wiki read <slug> --body-only` — body present, frontmatter absent
- [x] `wiki read <missing>` — exits non-zero
- [x] MCP resource `wiki://default/concepts/foo/diagram.png` — returns asset content
- [x] MCP bundle page still readable after promotion
- [x] `wiki lint` — orphan asset ref appears in `LINT.md`

## Changelog

- [x] `CHANGELOG.md` — Phase 8 entry added

## README

- [x] CLI reference — `wiki read <slug>` and `wiki read <slug> --body-only` added
- [x] **Repository layout** section — flat vs bundle, when each is used, asset
  co-location, links to design and dev docs

## Dev documentation

- [x] `docs/dev/layout.md` — slug resolution rules, bundle promotion, asset
  placement decision (co-located vs shared), `assets/index.md` format
- [x] `docs/dev/architecture.md` — Phase 8 modules annotated, phase table updated
