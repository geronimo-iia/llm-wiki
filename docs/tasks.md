---
title: "Tasks"
summary: "Full implementation task list — one checkbox per concrete action, ordered within each phase."
read_when:
  - Starting implementation work on a phase
  - Checking what remains in the current phase
  - Picking up where a previous session left off
status: active
last_updated: "2025-07-15"
---

# Tasks

Phases match `docs/roadmap.md`. Check off tasks as they are completed.
When all tasks in a phase are done, archive the phase to `docs/archive/`.

---

## Phase 1 — Foundation: Schema + Config + Spaces

### `config.rs`
- [x] Define `GlobalSection { default_wiki: String }`
- [x] Define `WikiEntry { name, path, description, remote }`
- [x] Define `Defaults { search_top_k, search_excerpt, search_sections, page_mode, list_page_size }`
- [x] Define `ReadConfig { no_frontmatter }`
- [x] Define `IndexConfig { auto_rebuild }`
- [x] Define `GraphConfig { format, depth, r#type, output }`
- [x] Define `ServeConfig { sse, sse_port, acp }`
- [x] Define `LintConfig { fix_missing_stubs, fix_empty_sections }`
- [x] Define `ValidationConfig { type_strictness }` — `strict | loose`
- [x] Define `SchemaConfig { custom_types: Vec<String> }`
- [x] Define `GlobalConfig` composing all sections
- [x] Define `WikiConfig { name, description }`
- [x] Define `ResolvedConfig` — merged global + per-wiki
- [x] Implement `resolve(global, per_wiki) -> ResolvedConfig`
- [x] Implement `load_global(path: &Path) -> Result<GlobalConfig>`
- [x] Implement `load_wiki(wiki_root: &Path) -> Result<WikiConfig>`
- [x] Implement `load_schema(wiki_root: &Path) -> Result<SchemaConfig>`

### `spaces.rs`
- [x] Implement `resolve_uri(uri, global) -> Result<(WikiEntry, String)>`
- [x] Implement `resolve_name(name, global) -> Result<WikiEntry>`
- [x] Implement `register(entry, force, config_path) -> Result<()>`
- [x] Implement `remove(name, delete, config_path) -> Result<()>`
- [x] Implement `load_all(global) -> Vec<WikiEntry>`
- [x] Implement `set_default(name, config_path) -> Result<()>`

### `git.rs`
- [x] Implement `init_repo(path: &Path) -> Result<()>`
- [x] Implement `commit(repo_root, message) -> Result<String>`
- [x] Implement `current_head(repo_root) -> Result<String>`
- [x] Implement `diff_last(repo_root) -> Result<Vec<String>>`

### `cli.rs` — Phase 1 commands
- [x] `wiki init <path> --name --description --force --set-default`
- [x] `wiki config get <key>`
- [x] `wiki config set <key> <value> [--global] [--wiki]`
- [x] `wiki config list [--global] [--wiki]`
- [x] `wiki spaces list`
- [x] `wiki spaces remove <name> [--delete]`
- [x] `wiki spaces set-default <name>`

### `mcp.rs` — Phase 1 tools
- [x] `wiki_init`
- [x] `wiki_config`
- [x] `wiki_spaces_list`
- [x] `wiki_spaces_remove`
- [x] `wiki_spaces_set_default`

### `init.rs` logic (called by cli + mcp)
- [x] Create directory structure (`inbox/`, `raw/`, `wiki/`)
- [x] Write `README.md`
- [x] Write `wiki.toml`
- [x] Write default `schema.md`
- [x] Run `git init` + initial commit `init: <name>`
- [x] Register in `~/.wiki/config.toml`

### `tests/config.rs`
- [x] `load_global` parses a valid `config.toml`
- [x] `load_global` returns error on malformed TOML
- [x] `resolve` per-wiki value overrides global value
- [x] `resolve` falls back to global when per-wiki key absent
- [x] `load_schema` parses custom types from `schema.md`
- [x] `load_schema` returns empty custom types when `schema.md` absent

### `tests/spaces.rs`
- [x] `resolve_uri` parses full URI `wiki://name/slug` correctly
- [x] `resolve_uri` uses default wiki for short URI `wiki://slug`
- [x] `resolve_uri` returns error for unknown wiki name
- [x] `register` appends entry to `config.toml`
- [x] `register` with `force` updates existing entry
- [x] `register` errors on duplicate name without `force`
- [x] `remove` removes entry from `config.toml`
- [x] `remove` with `--delete` removes directory from disk
- [x] `remove` errors when wiki is current default

### `tests/git.rs`
- [x] `init_repo` creates a git repository at path
- [x] `commit` creates a commit and returns hash
- [x] `current_head` returns the current commit hash
- [x] `current_head` matches hash returned by `commit`

### Exit criteria
- [x] `cargo test` green
- [x] `wiki init ~/wikis/test --name test` creates structure and registers
- [x] `wiki config list` shows resolved config
- [x] `wiki spaces list` shows registered wikis

---

## Phase 2 — Core Write Loop: Ingest + Page Creation

### `frontmatter.rs`
- [x] Define `PageFrontmatter` struct (all fields per spec)
- [x] Implement `parse_frontmatter(content: &str) -> Result<(PageFrontmatter, String)>`
- [x] Implement `write_frontmatter(fm: &PageFrontmatter, body: &str) -> String`
- [x] Implement `generate_minimal_frontmatter(title: &str) -> PageFrontmatter`
- [x] Implement `scaffold_frontmatter(slug: &str) -> PageFrontmatter`

### `markdown.rs`
- [x] Implement `slug_for(path, wiki_root) -> String`
- [x] Implement `resolve_slug(slug, wiki_root) -> Result<PathBuf>`
- [x] Implement `read_page(slug, wiki_root, no_frontmatter) -> Result<String>`
- [x] Implement `list_assets(slug, wiki_root) -> Result<Vec<String>>`
- [x] Implement `read_asset(slug, filename, wiki_root) -> Result<Vec<u8>>`
- [x] Implement `promote_to_bundle(slug, wiki_root) -> Result<()>`
- [x] Implement `create_page(slug, bundle, wiki_root) -> Result<PathBuf>`
- [x] Implement `create_section(slug, wiki_root) -> Result<PathBuf>`

### `ingest.rs`
- [x] Define `IngestOptions { dry_run: bool }`
- [x] Define `IngestReport { pages_validated, assets_found, warnings, commit }`
- [x] Implement `ingest(path, options, wiki_root) -> Result<IngestReport>`
  - [x] Walk path, collect `.md` files and assets
  - [x] Validate each `.md` (title present, YAML valid)
  - [x] Generate minimal frontmatter for files without it
  - [x] Set `last_updated` to today
  - [x] `git add` + commit
  - [ ] Update tantivy index

### `cli.rs` — Phase 2 commands
- [x] `wiki ingest <path> [--dry-run]`
- [x] `wiki new page <wiki:// URI> [--bundle] [--dry-run]`
- [x] `wiki new section <wiki:// URI> [--dry-run]`

### `mcp.rs` — Phase 2 tools
- [x] `wiki_write`
- [x] `wiki_ingest`
- [x] `wiki_new_page`
- [x] `wiki_new_section`

### `tests/frontmatter.rs`
- [x] `parse_frontmatter` round-trips all required fields
- [x] `parse_frontmatter` returns error on invalid YAML
- [x] `parse_frontmatter` returns error when no frontmatter block
- [x] `write_frontmatter` produces valid YAML block + blank line + body
- [x] `generate_minimal_frontmatter` sets title from H1, falls back to filename
- [x] `generate_minimal_frontmatter` sets status `active`, type `page`
- [x] `scaffold_frontmatter` derives title from slug segments
- [x] `scaffold_frontmatter` sets status `draft`, type `page`

### `tests/markdown.rs`
- [x] `slug_for` flat file returns path without extension
- [x] `slug_for` bundle `index.md` returns parent directory path
- [x] `resolve_slug` finds flat `.md` file
- [x] `resolve_slug` finds bundle `index.md`
- [x] `resolve_slug` returns error for missing slug
- [x] `read_page` returns full content including frontmatter
- [x] `read_page` with `no_frontmatter=true` strips frontmatter block
- [x] `list_assets` returns empty vec for flat page
- [x] `list_assets` returns `wiki://` URIs for bundle assets
- [x] `read_asset` returns raw bytes for a co-located asset
- [x] `promote_to_bundle` moves `{slug}.md` to `{slug}/index.md`
- [x] `promote_to_bundle` slug resolves correctly after promotion
- [x] `create_page` creates flat `.md` with scaffold frontmatter
- [x] `create_page` with `bundle=true` creates `{slug}/index.md`
- [x] `create_page` auto-creates missing parent sections
- [x] `create_section` creates `{slug}/index.md` with section frontmatter

### `tests/ingest.rs`
- [x] `ingest` validates a valid page and commits
- [x] `ingest` rejects a page with no `title` field
- [x] `ingest` rejects a page with invalid YAML frontmatter
- [x] `ingest` generates minimal frontmatter for a file without it
- [x] `ingest` sets `last_updated` to today on every page
- [x] `ingest` with `dry_run=true` does not commit
- [x] `ingest` on a folder ingests all `.md` files recursively
- [x] `ingest` detects co-located assets and includes them in `assets_found`
- [x] `IngestReport` commit hash matches `git HEAD` after ingest

### Exit criteria
- [x] `wiki new page wiki://test/concepts/foo` creates scaffolded page and commits
- [x] `wiki ingest wiki/concepts/foo.md` validates, commits, indexes
- [x] `wiki ingest wiki/` ingests all pages in the tree
- [x] `--dry-run` shows what would happen without committing

---

## Phase 3 — Frontmatter Validation + Type Taxonomy

### `frontmatter.rs` — validation
- [x] Define built-in type list (concept, query-result, section, paper, article, documentation, clipping, transcript, note, data, book-chapter, thread)
- [x] Implement `validate_frontmatter(fm, schema) -> Result<Vec<Warning>>`
  - [x] Required fields: title, summary, read_when, status, type, last_updated
  - [x] Type recognized (built-in + custom from schema)
  - [x] `source-summary` deprecated warning
  - [x] `strict` mode: unknown type → error
  - [x] `loose` mode: unknown type → warning

### `ingest.rs` — wire validation
- [x] Call `validate_frontmatter` on every `.md` during ingest
- [x] Respect `validation.type_strictness` from resolved config
- [x] Include warnings in `IngestReport`

### `src/instructions.md`
- [x] Write `## frontmatter` section (condensed type taxonomy + per-type templates + common mistakes)
- [x] Write `## help` section
- [x] Write `## new` section
- [x] Write `## ingest` section
- [x] Write `## research` section
- [x] Write `## lint` section
- [x] Write `## crystallize` section

### `tests/frontmatter.rs` — Phase 3 additions
- [x] `validate_frontmatter` passes for a fully valid page
- [x] `validate_frontmatter` warns on missing `read_when`
- [x] `validate_frontmatter` warns on missing `summary`
- [x] `validate_frontmatter` warns on `source-summary` type
- [x] `validate_frontmatter` in `loose` mode warns on unknown type, does not error
- [x] `validate_frontmatter` in `strict` mode errors on unknown type
- [x] `validate_frontmatter` accepts custom type defined in `SchemaConfig`

### Exit criteria
- [x] `wiki ingest` warns on missing `read_when`
- [x] `wiki ingest` warns on `source-summary` type
- [x] `wiki ingest` rejects unknown type in strict mode

---

## Phase 4 — Search + Read + Index

### `search.rs`
- [x] Define `PageRef { slug, uri, title, score, excerpt: Option<String> }`
- [x] Define `PageSummary { slug, uri, title, r#type, status, tags }`
- [x] Define `PageList { pages, total, page, page_size }`
- [x] Define `IndexStatus { wiki, path, built, pages, sections, stale }`
- [x] Define `IndexReport { wiki, pages_indexed, duration_ms }`
- [x] Implement tantivy schema with all frontmatter fields
- [x] Implement `rebuild_index(wiki_root, index_path) -> Result<IndexReport>`
  - [x] Walk `wiki/`, index all `.md` files
  - [x] Write `state.toml` with commit hash, page count, built date
- [x] Implement `index_status(wiki_name, index_path, wiki_root) -> Result<IndexStatus>`
  - [x] Read `state.toml`, compare commit vs `git HEAD`
- [x] Implement `search(query, options, index_path) -> Result<Vec<PageRef>>`
- [x] Implement `list(filter, page, page_size, index_path) -> Result<PageList>`
- [x] Staleness check before search/list — warn or auto-rebuild per config

### `cli.rs` — Phase 4 commands
- [x] `wiki search "<query>" [--no-excerpt] [--top-k] [--include-sections] [--all] [--wiki]`
- [x] `wiki read <slug|uri> [--no-frontmatter] [--list-assets] [--wiki]`
- [x] `wiki list [--type] [--status] [--page] [--page-size] [--wiki]`
- [x] `wiki index rebuild [--wiki] [--dry-run]`
- [x] `wiki index status [--wiki]`

### `mcp.rs` — Phase 4 tools
- [x] `wiki_search`
- [x] `wiki_read`
- [x] `wiki_list`
- [x] `wiki_index_rebuild`
- [x] `wiki_index_status`

### `tests/search.rs`
- [x] `rebuild_index` indexes all pages in `wiki/` and writes `state.toml`
- [x] `rebuild_index` stores commit hash in `state.toml`
- [x] `index_status` returns `stale: false` immediately after rebuild
- [x] `index_status` returns `stale: true` after a new commit
- [x] `index_status` returns `built: None` when index does not exist
- [x] `search` returns results ranked by BM25 score
- [x] `search` with `no_excerpt` returns `PageRef` with `excerpt: None`
- [x] `search` with `include_sections=false` excludes `type: section` pages
- [x] `search` with `include_sections=true` includes `type: section` pages
- [x] `search` `--type paper` filters results to paper pages only
- [x] `list` returns all pages ordered by slug
- [x] `list` with `--type concept` returns only concept pages
- [x] `list` with `--status draft` returns only draft pages
- [x] `list` pagination returns correct page and total

### Exit criteria
- [x] `wiki search "MoE scaling"` returns ranked `Vec<PageRef>` with `wiki://` URIs
- [x] `wiki read wiki://test/concepts/foo` returns full page content
- [x] `wiki list --type concept` returns paginated concept pages
- [x] `wiki index status` shows stale/fresh correctly
- [x] `wiki index rebuild` rebuilds and writes `state.toml`

---

## Phase 5 — Lint + Graph

### `links.rs`
- [x] Implement `extract_links(content: &str) -> Vec<String>` — frontmatter slugs + body `[[links]]`

### `lint.rs`
- [x] Define `MissingConnection { slug_a, slug_b, overlapping_terms }`
- [x] Define `LintReport { orphans, missing_stubs, empty_sections, missing_connections, untyped_sources, date }`
- [x] Implement `lint(wiki_root, config) -> Result<LintReport>`
  - [x] Orphan detection — in-degree 0 via petgraph
  - [x] Missing stub detection — referenced slugs that don't exist
  - [x] Empty section detection — dirs without `index.md`
  - [x] Missing connection detection — term overlap heuristic
  - [x] Untyped source detection — missing or `source-summary` type
- [x] Implement `write_lint_md(report, repo_root) -> Result<()>` — all 5 sections
- [x] Implement `lint_fix(wiki_root, config, only) -> Result<()>`
  - [x] Create stub pages for missing stubs
  - [x] Create `index.md` for empty sections

### `graph.rs`
- [x] Define `PageNode { slug, title, r#type }`
- [x] Define `GraphFilter { root, depth, types }`
- [x] Define `GraphReport { nodes, edges, output, committed }`
- [x] Implement `build_graph(wiki_root, filter) -> DiGraph<PageNode, ()>`
- [x] Implement `render_mermaid(graph) -> String`
- [x] Implement `render_dot(graph) -> String`
- [x] Implement `subgraph(graph, root, depth) -> DiGraph<PageNode, ()>`
- [x] Implement `in_degree(graph, slug) -> usize`

### `cli.rs` — Phase 5 commands
- [x] `wiki lint [--wiki] [--dry-run]`
- [x] `wiki lint fix [--only missing-stubs|empty-sections] [--dry-run] [--wiki]`
- [x] `wiki graph [--format] [--root] [--depth] [--type] [--output] [--dry-run] [--wiki]`

### `mcp.rs` — Phase 5 tools
- [x] `wiki_lint`
- [x] `wiki_graph`

### `tests/links.rs`
- [x] `extract_links` returns slugs from `sources` frontmatter field
- [x] `extract_links` returns slugs from `concepts` frontmatter field
- [x] `extract_links` returns slugs from body `[[wikilinks]]`
- [x] `extract_links` deduplicates repeated slugs
- [x] `extract_links` returns empty vec for page with no links

### `tests/lint.rs`
- [x] `lint` detects orphan pages (in-degree 0)
- [x] `lint` does not flag pages with at least one incoming link as orphans
- [x] `lint` detects missing stubs (referenced slug does not exist)
- [x] `lint` detects empty sections (dir without `index.md`)
- [x] `lint` detects untyped sources (`source-summary` type)
- [x] `lint` detects untyped sources (missing type on source-like page)
- [x] `write_lint_md` writes all 5 sections always, even when empty
- [x] `write_lint_md` shows `_No X found._` for empty sections
- [x] `lint_fix` creates stub pages for missing stubs
- [x] `lint_fix` creates `index.md` for empty sections
- [x] `lint_fix` with `only=missing-stubs` does not touch empty sections

### `tests/graph.rs`
- [x] `build_graph` creates edges from `sources` frontmatter
- [x] `build_graph` creates edges from `concepts` frontmatter
- [x] `build_graph` creates edges from body `[[links]]`
- [x] `build_graph` skips broken references (missing stubs)
- [x] `in_degree` returns 0 for orphan page
- [x] `in_degree` returns correct count for linked page
- [x] `render_mermaid` produces valid Mermaid `graph TD` block
- [x] `render_dot` produces valid DOT `digraph` block
- [x] `subgraph` returns only nodes within depth hops of root
- [x] `subgraph` with `depth=0` returns only the root node

### Exit criteria
- [x] `wiki lint` writes `LINT.md` at repository root with all 5 sections
- [x] `wiki lint fix` creates missing stubs and empty section indexes
- [x] `wiki graph` outputs Mermaid to stdout
- [x] `wiki graph --format dot` outputs DOT format
- [x] `wiki graph --root <slug> --depth 2` outputs subgraph

---

## Phase 6 — MCP Server + Session Bootstrap

### `server.rs`
- [x] Implement `WikiServer` with all registered wikis mounted at startup
- [x] Implement startup sequence (load config → mount wikis → check staleness → start stdio)
- [x] Implement SSE transport (`--sse [:<port>]`)
- [x] Inject `instructions.md` + `schema.md` at session start

### `mcp.rs` — complete
- [x] Wire all tools from phases 1–5 into `WikiServer`
- [x] Add `wiki` param to all tools (target specific wiki)
- [x] MCP resources namespaced by wiki name (`wiki://<name>/<slug>`)
- [x] MCP resource update notifications on every ingest
- [x] Prompts: `ingest_source`, `research_question`, `lint_and_fix`
- [x] Remove `wiki_context` tool

### `src/instructions.md` — session bootstrap
- [x] Write `## session-orientation` preamble
- [x] Write `## linking-policy` preamble
- [x] Add orientation step to every workflow section

### `cli.rs` — Phase 6 commands
- [x] `wiki serve [--sse [:<port>]] [--acp] [--dry-run]`
- [x] `wiki instruct [<workflow>]`

### Exit criteria
- [x] `wiki serve` starts, all registered wikis accessible via MCP
- [ ] Claude Code can call all MCP tools
- [x] `wiki instruct crystallize` prints the crystallize workflow
- [x] `wiki instruct frontmatter` prints the frontmatter guide
- [x] `schema.md` is injected alongside instructions at session start

---

## Phase 7 — ACP Transport

### `acp.rs`
- [ ] Add `agent-client-protocol = "0.10"` and `agent-client-protocol-tokio = "0.1"` to `Cargo.toml`
- [ ] Define `AcpSession { id, label, wiki, created_at, active_run }`
- [ ] Define `WikiAgent { spaces, sessions }`
- [ ] Implement `Agent::initialize` — inject `instructions.md` as system context
- [ ] Implement `Agent::new_session`
- [ ] Implement `Agent::load_session`
- [ ] Implement `Agent::list_sessions`
- [ ] Implement `Agent::prompt` — workflow dispatch (ingest, research, lint, crystallize)
- [ ] Implement `Agent::cancel`
- [ ] Implement `serve_acp(spaces) -> Result<()>`

### `server.rs`
- [ ] Start ACP stdio server alongside MCP when `--acp`

### Exit criteria
- [ ] `wiki serve --acp` starts without error
- [ ] Zed agent panel connects and lists sessions
- [ ] `ingest` workflow streams tool calls visibly
- [ ] `research` workflow streams answer

---

## Phase 8 — Claude Plugin

### `.claude-plugin/`
- [ ] Update `plugin.json` to spec
- [ ] Update `marketplace.json` to spec
- [ ] Update `.mcp.json` to spec
- [ ] Write `commands/help.md`
- [ ] Write `commands/init.md`
- [ ] Write `commands/new.md`
- [ ] Write `commands/ingest.md`
- [ ] Write `commands/research.md`
- [ ] Write `commands/crystallize.md`
- [ ] Write `commands/lint.md`
- [ ] Update `skills/llm-wiki/SKILL.md` — remove contradiction workflow
- [ ] Verify `wiki instruct <workflow>` returns correct instructions for all 7 workflows

### Exit criteria
- [ ] `claude plugin add /path/to/llm-wiki` succeeds
- [ ] `/llm-wiki:ingest` triggers the ingest workflow
- [ ] `/llm-wiki:crystallize` triggers the crystallize workflow

---

## Phase 9 — Documentation

- [ ] Rewrite `README.md` — features, workflows, quick start, MCP client setup
- [ ] Rewrite `CONTRIBUTING.md` — module architecture, dev setup, test patterns, release process
- [ ] Rewrite `CHANGELOG.md` — feature-oriented, not a git log

### Exit criteria
- [ ] A new contributor can read `README.md` and run `wiki init` within 5 minutes
- [ ] `CONTRIBUTING.md` references `docs/implementation/rust.md` for dev standards
- [ ] `CHANGELOG.md` describes what the tool can do at each version
