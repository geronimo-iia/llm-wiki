# Phase 8 — Repository Layout + Bundle Support

You are implementing Phase 8 of llm-wiki. Phase 7 (incremental search index)
is complete.

## Context

The full task list is in `docs/tasks/phase-8.md`.
Design refs: `docs/design/repository-layout.md`, `docs/design/asset-ingest.md`.

## The Goal

The wiki currently only supports flat `.md` files. After this phase it supports
two page forms:

- **Flat** — `concepts/foo.md` — page with no assets
- **Bundle** — `concepts/foo/index.md` + co-located files — page with assets

A slug (`concepts/foo`) resolves to whichever form exists on disk. All walkers
(search, graph, lint, context, MCP server) must handle both transparently.

## What to implement

### `src/markdown.rs` — four new functions

```
slug_for(path, wiki_root) -> String
  — index.md in a folder → slug = parent dir
  — any other .md → slug = path without extension

resolve_slug(wiki_root, slug) -> Option<PathBuf>
  — check {slug}.md first, then {slug}/index.md

promote_to_bundle(wiki_root, slug) -> Result<()>
  — move {slug}.md → {slug}/index.md (create dir)
  — no-op if already bundle

is_bundle(wiki_root, slug) -> bool
```

### `src/integrate.rs` — asset writing

```
write_asset_colocated(wiki_root, page_slug, filename, content)
  — promote page to bundle if flat
  — write asset beside index.md

write_asset_shared(wiki_root, kind, filename, content)
  — write to assets/{subdir}/ per kind→subdir table in repository-layout.md
  — update assets/index.md

regenerate_assets_index(wiki_root)
  — walk assets/, rebuild markdown table, write
```

Add `bundles_created: usize` to `IngestReport`.

### Update all walkers to use `slug_for` and `resolve_slug`

- `src/search.rs` — `build_index` and `update_index`: use `slug_for`, skip
  non-`index.md` files in bundle folders, skip `assets/` except `assets/index.md`.
  Add `path: String` (absolute) to `SearchResult`.
- `src/graph.rs` — `build_graph`: use `slug_for`
- `src/context.rs` — page path resolution: use `resolve_slug`
- `src/lint.rs` — page walker: use `slug_for`. Add orphan asset ref check.
  Add `orphan_asset_refs: Vec<String>` to `LintReport`.
- `src/server.rs` — MCP resource resolution: use `resolve_slug`. Register
  bundle asset resources at `wiki://{wiki}/{slug}/{filename}`.

### `src/cli.rs` — new subcommand

```
wiki read <slug>             # print frontmatter + body
wiki read <slug> --body-only # body only
```

Resolves via `resolve_slug`. Works for both flat and bundle pages.

## Key rules from `repository-layout.md`

- Slug resolution: check `{slug}.md` first, then `{slug}/index.md`
- Assets co-located with their page (not in central `assets/`) unless shared
- `assets/` subtree: `diagrams/`, `configs/`, `scripts/`, `data/`, `other/`
- `assets/index.md` is a committed Markdown table of all shared assets

## Tests

Add to `tests/ingest.rs`, `tests/search.rs`, `tests/graph.rs`:

- `slug_for` flat and bundle cases
- `resolve_slug` flat, bundle, missing
- `promote_to_bundle` flat→bundle, already-bundle no-op
- `write_asset_colocated` promotes flat page, writes asset
- `write_asset_shared` correct subdir
- `build_index` bundle page indexed once, asset files not indexed
- `lint` orphan asset ref detected
- Integration: ingest folder → bundle created, `wiki read` resolves it
- Integration: `wiki search` after bundle promotion — slug unchanged

## Acceptance

```bash
cargo test
wiki ingest agent-skills/semantic-commit/ --prefix skills
# → skills/semantic-commit/index.md + lifecycle.yaml co-located
wiki read skills/semantic-commit
# → full page content
```

## Constraints

- No LLM dependency
- Follow existing code style
- Update `CHANGELOG.md` with a Phase 8 entry
- Create `docs/dev/layout.md` covering slug resolution and bundle promotion
