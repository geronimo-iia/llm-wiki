# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Phase 8 (2026-04-15)

#### Added
- `markdown::slug_for(path, wiki_root)` — derives the slug for any wiki `.md`
  file: `index.md` in a folder → slug = parent dir; any other `.md` → slug =
  path without extension.
- `markdown::resolve_slug(wiki_root, slug)` — resolves a slug to its file path;
  checks `{slug}.md` first, then `{slug}/index.md`; returns `None` if neither
  exists.
- `markdown::promote_to_bundle(wiki_root, slug)` — moves `{slug}.md` →
  `{slug}/index.md`, creating the directory; no-op if already a bundle.
- `markdown::is_bundle(wiki_root, slug)` — returns `true` if
  `{slug}/index.md` exists.
- `integrate::write_asset_colocated(wiki_root, page_slug, filename, content)`
  — promotes the page to a bundle if flat, then writes the asset beside
  `index.md`.
- `integrate::write_asset_shared(wiki_root, kind, filename, content)` — writes
  to `assets/{subdir}/` per the kind→subdir table and regenerates
  `assets/index.md`.
- `integrate::assets_index_path(wiki_root)` — returns path to `assets/index.md`.
- `integrate::regenerate_assets_index(wiki_root)` — walks `assets/`, rebuilds
  the Markdown table, writes `assets/index.md`.
- `IngestReport::bundles_created: usize` — count of flat pages promoted to
  bundles during an ingest session.
- `LintReport::orphan_asset_refs: Vec<String>` — bundle pages with `./asset`
  references that don't exist in the bundle folder.
- `wiki read <slug>` CLI subcommand — prints frontmatter + body of a page;
  resolves via `resolve_slug` (works for flat and bundle).
- `wiki read <slug> --body-only` — prints body only.
- `docs/dev/layout.md` — slug resolution rules, bundle promotion, asset
  placement decision, `assets/index.md` format.
- 22 new tests across `tests/ingest.rs`, `tests/search.rs`, `tests/graph.rs`.

#### Changed
- `search::build_index` — uses `slug_for` for consistent slug derivation;
  skips non-`index.md` files inside bundle folders; skips `assets/` subtree
  except `assets/index.md`.
- `search::update_index` — uses `resolve_slug` to find the page file (flat or
  bundle) when re-indexing a changed slug.
- `SearchResult` gains `path: String` (absolute path to the page file).
- `SearchResultWithWiki` gains `path: String`.
- `graph::build_graph` — uses `slug_for`; skips non-`index.md` files inside
  bundle folders so bundle assets are not treated as pages.
- `graph::missing_stubs` — uses `resolve_slug` to check existence (handles
  both flat and bundle forms).
- `context::context` — uses `resolve_slug` for page path resolution.
- `lint::lint` — uses `slug_for` in the orphan asset ref scanner; adds
  `## Orphan Asset References` section to `LINT.md`.
- `server::do_read_resource` — uses `resolve_slug`; also handles bundle asset
  URIs `wiki://{wiki}/{slug}/{filename}`.
- `server::list_pages_in_root` — uses `slug_for`; skips non-`index.md` files
  inside bundle folders.

### Phase 7 (2026-04-13)

#### Added
- `search::open_or_build_index(wiki_root, index_dir)` — opens an existing
  tantivy index or builds from scratch on first use / corrupt index.
- `search::update_index(wiki_root, index_dir, changed_slugs)` — incremental
  index update: deletes and re-indexes only the changed slugs; deleted pages
  are removed from the index; no-op for empty slug list.
- `IngestReport::index_updated: bool` — true when the index was updated
  incrementally after an ingest; false when the index did not yet exist.
- `IngestReport::changed_slugs: Vec<String>` — slugs of all pages written
  during the ingest session (used internally for index update).
- 7 new tests in `tests/search.rs`: `open_or_build_index` lifecycle,
  `update_index` for new/modified/deleted/empty-slug cases, and two-consecutive-
  search mtime stability.

#### Changed
- `search::search()` now calls `open_or_build_index` instead of `build_index`
  — no full rebuild on every query. Pass `rebuild_index = true` to force a
  full rebuild (used by `wiki search --rebuild-index`).
- `search::search_all()` uses `open_or_build_index` per wiki.
- `integrate::integrate()` collects slugs of all pages written and stores them
  in `IngestReport::changed_slugs`.
- `ingest::ingest()` calls `search::update_index` after the git commit when
  the index directory already exists.
- Updated `cli_search_new_page_reflected_without_explicit_rebuild` test to
  reflect the incremental policy (renamed to
  `cli_search_new_page_reflected_after_update_index`).

### Phase 6 (2026-04-13)

#### Added
- `src/registry.rs` — `WikiRegistry` with `load(config_path)` and `resolve(name)`.
  Parses `~/.wiki/config.toml` (`[[wikis]]` array) and validates: at most one
  default, all paths must exist.
- `WikiEntry` struct: `name`, `path`, `default`, `remote` (optional git URL).
- `global_config_path()` — resolves `~/.wiki/config.toml` from `$HOME`.
- `register_wiki(name, path, config_path)` — appends an entry to the global
  config, creating the file if absent; first wiki becomes `default = true`.
- `--wiki <name>` global CLI flag (already declared in Phase 4) is now
  **wired** through `resolve_wiki_config()` into all subcommands: `ingest`,
  `search`, `context`, `lint`, `list`, `contradict`, `graph`, `diff`.
- `wiki init --register` — registers the new wiki in `~/.wiki/config.toml`
  after initialisation.
- `SearchResultWithWiki` — search result tagged with `wiki_name`.
- `search::search_all(registry, query, limit)` — fan-out search across all
  registered wikis, merged by descending BM25 score.
- `wiki search --all "<term>"` — cross-wiki search with a `WIKI` column in
  the output table.
- `wiki serve --sse :<port>` — MCP server on HTTP SSE transport using
  `rmcp::transport::sse_server::SseServer` (rmcp `transport-sse-server`
  feature; axum-backed).  Each connecting client gets an independent
  `WikiServer` session.  Ctrl-C triggers graceful shutdown.
- `WikiServer::new_with_registry(root, registry)` — multi-wiki constructor.
- `WikiServer::resolve_root(wiki)` — resolves the target root from the
  registry or falls back to `self.wiki_root`.
- `WikiServer::do_ingest_with_wiki(analysis, wiki)` — ingest into a named wiki.
- All five MCP tools (`wiki_ingest`, `wiki_context`, `wiki_search`,
  `wiki_lint`, `wiki_list`) now resolve the target root via the registry when
  present.
- `wiki_search` with `all_wikis: true` calls `search_all`.
- Resources namespaced as `wiki://{wiki_name}/{type}/{slug}`.
  `list_resources` and `list_resource_templates` enumerate all registered wikis
  when a registry is present.
- `docs/dev/multi-wiki.md` — multi-wiki developer guide.
- 16 registry tests in `tests/registry.rs` (0 failures).

### Phase 5 (2026-04-13)

#### Added
- `wiki init [<path>]` — initialise a new wiki repository: runs `git init` (skipped
  if `.git/` already exists), creates `concepts/`, `sources/`, `contradictions/`,
  `queries/`, `raw/`, `.wiki/config.toml`. Idempotent. Prints an MCP config snippet
  and instructs the user to run `/llm-wiki:init`.
- `src/init.rs` — `init_wiki(root)` and `mcp_config_snippet(root)` exposed as
  library functions so tests can call them directly (no subprocess).
- `.claude-plugin/plugin.json` — `commands` array populated with all 6 slash
  commands: `help`, `init`, `ingest`, `research`, `lint`, `contradiction`.
- `src/instructions.md` — all six `## {name}-workflow` sections complete and
  actionable:
  - `help-workflow` — slash commands table + MCP tools list
  - `init-workflow` — verify install → `wiki init` → MCP config snippet →
    `/llm-wiki:init`
  - `ingest-workflow` — two-step workflow (wiki_context → analysis.json →
    wiki_ingest), schema reminder, contradiction gate
  - `research-workflow` — wiki_context, synthesis, optional save
  - `lint-workflow` — wiki_lint, orphan/stub/contradiction remediation loop
  - `contradiction-workflow` — wiki_list + wiki_context, dimension analysis,
    enrichment with resolution, never-delete rule
- 8 tests in `tests/plugin.rs`: 6 instruction-completeness unit tests +
  2 `wiki init` directory-structure tests using `tempfile::TempDir`.

### Phase 4 (2026-04-13)

#### Added
- `wiki serve` — starts the MCP server on stdio; `--sse` prints a warning and
  falls back to stdio (full SSE in Phase 6)
- `wiki instruct` — prints `src/instructions.md` in full to stdout
- `wiki instruct <workflow>` — prints the named `## {workflow}-workflow` section
  (available workflows: `help`, `init`, `ingest`, `research`, `lint`, `contradiction`)
- `WikiServer` — rmcp 0.1 `ServerHandler` with five MCP tools:
  - `wiki_ingest(analysis, wiki?)` — deserialises `analysis.json`, calls
    `integrate::integrate`, commits via `git::commit`, returns a summary string
  - `wiki_context(question, wiki?, top_k?)` — calls `context::context`, returns
    top-K relevant pages as Markdown
  - `wiki_search(query, wiki?, all_wikis?)` — calls `search::search`, returns a
    JSON array of `{slug, title, snippet, score, page_type}` objects
  - `wiki_lint(wiki?)` — calls `lint::lint`, writes `LINT.md`, returns a JSON
    summary of orphans, missing stubs, and active contradictions
  - `wiki_list(wiki?, page_type?)` — walks the wiki, filters by type, returns a
    JSON array of `{slug, title, page_type}` objects
- MCP resources:
  - Resource template `wiki://default/{type}/{slug}` — page URI scheme
  - `list_resources` — all pages as `wiki://default/{slug}` URIs
  - `read_resource` — reads a page file by URI; unknown type or missing slug
    returns a resource-not-found error without panic
- MCP prompts: `ingest_source`, `research_question`, `lint_and_enrich`,
  `analyse_contradiction` — step-by-step workflow messages for the calling LLM
- `server::PageSummary` and `server::LintSummary` — serialisable return types
  exposed to tests and future callers
- `src/instructions.md` — embedded LLM usage guide covering all six workflows
  and the full `analysis.json` schema; injected into every MCP connection via
  `ServerInfo.instructions`
- 17 tests in `tests/mcp.rs`: 12 unit tests exercising `WikiServer::do_*` helpers
  directly + 5 integration tests verifying multi-step scenarios and on-disk state
- `docs/dev/mcp.md` — tool signatures, resource URI scheme, prompt definitions,
  transport modes

### Phase 3 (2026-04-13)

#### Added
- `wiki lint` — builds the concept graph, collects orphan pages, missing stubs,
  and active/under-analysis contradiction pages; writes `LINT.md`; commits with
  message `"lint: <date> — M orphans, K stubs, N active contradictions"`
- `wiki contradict` — lists all contradiction pages as a table (slug, title,
  status, dimension); `--status active|resolved|under-analysis` filter
- `wiki list` — lists all wiki pages as a table (slug, title, type);
  `--type concept|source|contradiction|query` filter
- `wiki graph` — prints the concept graph as DOT to stdout;
  `--format mermaid` for Mermaid output
- `wiki diff` — thin `git diff HEAD~1` wrapper; prints the diff of the last commit
- `graph::build_graph(wiki_root)` — `petgraph::DiGraph<String, EdgeKind>` built
  from `[[wikilinks]]` in page bodies (comrak), `related_concepts`, and
  `contradictions` frontmatter fields
- `EdgeKind` enum: `WikiLink`, `RelatedConcept`, `Contradiction`
- `graph::orphans(graph)` — nodes with in-degree = 0, excluding `raw/`
- `graph::missing_stubs(graph, wiki_root)` — reference targets with no `.md` on disk
- `graph::dot_output(graph)` — GraphViz DOT with per-kind edge styles
- `graph::mermaid_output(graph)` — Mermaid `graph TD` block
- `contradiction::list(wiki_root, status)` — walk `contradictions/`, parse
  frontmatter, filter by `Status`
- `contradiction::cluster(graph, slugs)` — concept pages adjacent to given
  contradiction slugs in the graph
- `ContradictionSummary` — lightweight view: slug, title, status, dimension,
  source\_a, source\_b
- `git::diff_last(root)` — diff between HEAD and HEAD~1 as a unified diff string
- `lint::LintReport` — orphan\_pages, missing\_stubs, active\_contradictions
- `lint::write_lint_report(wiki_root, report)` — writes `LINT.md` with Orphans,
  Missing Stubs, and Active Contradictions sections
- 20 tests in `tests/graph.rs` covering orphan detection, missing stubs,
  DOT/Mermaid output, contradiction listing, lint report, and integration behaviour
- `docs/dev/graph.md` — graph node/edge model, orphan and stub detection rules,
  DOT and Mermaid output format
- `docs/dev/lint.md` — LINT.md structure, section semantics, external LLM workflow
- `docs/dev/contradictions.md` — contradiction page lifecycle, enrichment workflow,
  status transitions, `cluster()` usage

### Phase 2 (2026-04-13)

#### Added
- `wiki search "<term>"` — BM25 full-text search via tantivy; prints a ranked
  results table (SLUG / TITLE / SCORE)
- `wiki search "<term>" --top <n>` — limit displayed results (default 20)
- `wiki search --rebuild-index` — rebuild the tantivy index and exit; useful
  for fresh clones or CI pre-warm
- `wiki context "<question>"` — returns the top-K most relevant wiki pages as
  a single Markdown string for an external LLM to synthesise from
- `wiki context "<question>" --top-k <n>` — control page count (default 5)
- `search::build_index(wiki_root, index_dir)` — walks all `.md` files, parses
  frontmatter + body, indexes each page; skips `raw/` and `.wiki/`
- `search::search_index(index, query, limit)` — BM25 query against a tantivy
  `Index`; returns empty `Vec` (not an error) for unknown terms
- `search::search(query, wiki_root, rebuild_index)` — always rebuilds the
  index before querying to ensure fresh results
- `SearchResult` fields: `slug`, `title`, `snippet` (first 200 chars of body),
  `score`, `page_type`
- `context::context(question, wiki_root, top_k)` — runs `search`, loads full
  page content, formats as `# {title}\n\n{body}\n---\n\n` blocks
- Contradiction pages included in context results — never filtered
- `.wiki/search-index/` added to `.gitignore` (index rebuilt locally, never committed)
- 14 tests in `tests/search.rs` covering index lifecycle, BM25 ranking, context
  assembly, and CLI behaviour
- `docs/dev/search.md` — tantivy schema, index lifecycle, rebuild policy,
  gitignore rationale, context output format

#### Changed
- `cli.rs` — `Search.query` changed to `Option<String>` (may be omitted with
  `--rebuild-index`); `--top <n>` flag added
- `src/main.rs` — `Command::Search` and `Command::Context` arms implemented

### Phase 1 (2026-04-13)

#### Added
- `wiki ingest <file|->` — reads `analysis.json` from a file path or stdin,
  writes wiki pages, and commits atomically (`src/ingest.rs`)
- `integrate::integrate` — core write loop: creates, updates, or appends `.md`
  files under `concepts/`, `sources/`, `queries/`; writes `contradictions/*.md`
  when `contradictions[]` is non-empty (`src/integrate.rs`)
- `markdown::parse_frontmatter` — splits a wiki `.md` file into
  `PageFrontmatter` + body; returns a clear error if the frontmatter block is
  absent or the YAML is malformed
- `markdown::write_page` — serialises `PageFrontmatter` to YAML and writes
  `---\n<yaml>\n---\n\n<body>` atomically; normalises CRLF to LF
- `markdown::frontmatter_from_page` — generates a fresh `PageFrontmatter` from
  a `SuggestedPage` at `create` time (`status: active`, `confidence: medium`,
  `last_updated: today`, empty `sources`/`contradictions`)
- `git::init_if_needed` — runs `git init` when no `.git` is present
- `git::stage_all` / `git::commit` — stage all changes and create a commit via
  libgit2; handles the initial-commit (no-parent) case
- Commit message format: `ingest: <title> — +N pages`
- Slug validation: rejects path traversal (`../`) and unknown category prefixes;
  only `concepts/`, `sources/`, `queries/`, `contradictions/` are accepted
- `IngestReport` with `Display` impl: counts for `pages_created`,
  `pages_updated`, `pages_appended`, `contradictions_written`
- 19 tests in `tests/ingest.rs` covering all unit and integration scenarios;
  17 internal module tests in `markdown.rs` and `integrate.rs`

#### Changed
- `src/main.rs` — `Command::Ingest` arm fully implemented; exits 0 on success,
  1 on any validation or write error

### Phase 0 (2026-04-13)

#### Added
- Schema structs with full serde derives and doc comments: `Analysis`, `Claim`,
  `SuggestedPage`, `Contradiction`, `DocType`, `Action`, `Status`, `Dimension`,
  `PageType`, `Confidence` (`src/analysis.rs`)
- `PageFrontmatter` with serde_yaml derives (`src/markdown.rs`)
- `WikiConfig` struct deserialised from `.wiki/config.toml` (`src/config.rs`)
- Typed stubs for all planned modules: `ingest`, `integrate`, `search`, `context`,
  `lint`, `graph`, `contradiction`, `git`, `server`, `registry`
- CLI skeleton with all command variants and typed args via clap derive (`src/cli.rs`)
- `src/main.rs` dispatches to stubs and compiles cleanly
- Unit tests: `Analysis` JSON round-trip, `PageFrontmatter` YAML round-trip,
  `WikiConfig` TOML load
- Integration test placeholder (`tests/integration_test.rs`)
- CI workflow: check, fmt, clippy, build, test (`.github/workflows/ci.yml`)
- `CONTRIBUTING.md` — build, test, lint, commit format, no-LLM-dependency rule
- GitHub issue templates: bug report, feature request, blank-issues disabled
- Dependabot config: Cargo + GitHub Actions, weekly, patch groups
- Developer architecture doc (`docs/dev/architecture.md`)
