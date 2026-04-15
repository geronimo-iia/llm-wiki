---
title: "Roadmap"
summary: "Phase-by-phase implementation plan derived from the specifications. Each phase is independently shippable and unlocks a concrete capability."
read_when:
  - Planning implementation work
  - Understanding the target architecture and delivery order
  - Deciding what to implement next
status: active
last_updated: "2025-07-15"
---

# Roadmap

---

## Target Architecture

```
src/
├── main.rs             # CLI entry point — dispatch only
├── lib.rs              # module declarations
├── cli.rs              # clap Command enum — all subcommands and flags
├── config.rs           # GlobalConfig, WikiConfig, two-level resolution
├── spaces.rs           # Spaces, WikiEntry, resolve_name(), resolve_uri()
├── git.rs              # init_repo(), commit(), current_head(), diff_last()
├── frontmatter.rs      # parse/write, scaffold_frontmatter(), validate_frontmatter(),
│                       # generate_minimal_frontmatter()
├── markdown.rs         # read_page, list_assets, read_asset,
│                       # promote_to_bundle, slug helpers
├── links.rs            # extract_links()
├── analysis.rs         # Enrichment, QueryResult, Asset, Analysis — new schema
├── ingest.rs           # IngestOptions, validate → git add → commit → index
├── search.rs           # PageRef, PageList, tantivy index, search(), list(),
│                       # IndexStatus, IndexReport, state.toml
├── lint.rs             # LintReport, orphan/stub/section detection, LINT.md
├── graph.rs            # petgraph build, render_mermaid/dot, GraphReport
├── server.rs           # WikiServer, startup, stdio + SSE transport wiring
├── mcp.rs              # all MCP tools, resources, prompts
└── acp.rs              # WikiAgent, AcpSession, workflow dispatch
```

---

## Phase 1 — Foundation: Schema + Config + Spaces

**Goal:** The new data model compiles. Config and spaces load correctly.
`wiki init`, `wiki config`, and `wiki spaces` work end-to-end.

### 1.1 New `analysis.rs` schema

Replace the old contract with the spec-defined types:

```rust
// Keep
pub struct Claim { text, confidence, section }
pub enum Confidence { High, Medium, Low }

// New
pub struct Asset { slug, filename, kind, content_encoding, content, caption }
pub enum AssetKind { Image, Yaml, Toml, Json, Script, Data, Other }
pub enum ContentEncoding { Utf8, Base64 }

// Remove
DocType, PageType, Action, SuggestedPage, Contradiction, Dimension, Status,
Enrichment, QueryResult, Analysis
```

### 1.2 `config.rs` — two-level config

```rust
pub struct GlobalConfig {
    pub global:   GlobalSection,   // default_wiki
    pub wikis:    Vec<WikiEntry>,
    pub defaults: Defaults,        // search_top_k, search_excerpt, page_mode, etc.
    pub index:    IndexConfig,     // auto_rebuild
    pub graph:    GraphConfig,     // format, depth, type, output
    pub serve:    ServeConfig,     // sse, sse_port, acp
    pub lint:     LintConfig,      // fix_missing_stubs, fix_empty_sections
    pub read:     ReadConfig,      // no_frontmatter
}
pub struct WikiConfig { name, root, description }
pub fn resolve(global: &GlobalConfig, per_wiki: &WikiConfig) -> ResolvedConfig
```

### 1.3 `spaces.rs` — `wiki://` URI resolution

```rust
pub fn resolve_uri(uri: &str, global: &GlobalConfig) -> Result<(WikiEntry, String)>
// "wiki://research/concepts/foo" → (research entry, "concepts/foo")
// "wiki://concepts/foo"          → (default wiki entry, "concepts/foo")
pub fn register(entry: WikiEntry, force: bool, config_path: &Path) -> Result<()>
pub fn remove(name: &str, delete: bool, config_path: &Path) -> Result<()>
```

### 1.4 CLI + MCP

- `wiki init <path> --name --description --force --set-default`
- `wiki config get/set/list`
- `wiki spaces list/remove/set-default`
- MCP tools: `wiki_init`, `wiki_config`, `wiki_spaces_*`

**Deliverable:** `cargo test` green. `wiki init` creates a wiki and registers it.

---

## Phase 2 — Core Write Loop: Ingest + Page Creation

**Goal:** `wiki ingest <path>` validates, commits, and indexes files already
in the wiki tree. `wiki new page/section` creates scaffolded pages.

### 2.1 `markdown.rs` additions

```rust
pub fn generate_minimal_frontmatter(title: &str) -> PageFrontmatter
pub fn scaffold_frontmatter(slug: &str) -> PageFrontmatter  // for wiki new
pub fn read_page(slug: &str, wiki_root: &Path, no_frontmatter: bool) -> Result<String>
pub fn list_assets(slug: &str, wiki_root: &Path) -> Result<Vec<String>>  // wiki:// URIs
pub fn read_asset(slug: &str, filename: &str, wiki_root: &Path) -> Result<Vec<u8>>
```

`PageFrontmatter` updated: remove `contradictions` field, add `claims`.

### 2.2 `ingest.rs` — validate, commit, index

```rust
pub struct IngestOptions { dry_run: bool }
pub fn ingest(path: &Path, options: &IngestOptions, wiki_root: &Path) -> Result<IngestReport>
// validate frontmatter → git add → commit → index
```

`IngestReport`: `pages_validated`, `assets_found`, `warnings`, `commit`.

No `integrate_file`, `integrate_folder`, or file placement logic — files are
already in the wiki tree.

### 2.3 `mcp.rs` — write + ingest tools

```rust
pub async fn wiki_write(path: String, content: String) -> WriteResult
pub async fn wiki_ingest(path: String, dry_run: Option<bool>) -> IngestReport
pub async fn wiki_new_page(slug: String, bundle: Option<bool>) -> String
pub async fn wiki_new_section(slug: String) -> String
```

### 2.4 CLI

- `wiki ingest <path> --dry-run`
- `wiki new page <slug> --bundle --dry-run`
- `wiki new section <slug> --dry-run`

**Deliverable:** Author writes a file into the wiki tree, `wiki ingest` validates,
commits, and indexes it.

---

## Phase 3 — Frontmatter Validation + Type Taxonomy

**Goal:** Engine validates frontmatter on ingest. Unified type taxonomy
(knowledge types + source types + custom) enforced. Frontmatter authoring
guide in instructions.

### 3.1 `markdown.rs` — validation

```rust
pub fn validate_frontmatter(fm: &PageFrontmatter, schema: &SchemaConfig) -> Result<Vec<Warning>>
// Checks: required fields present, type in built-in + custom list, source-summary deprecated
// No folder-to-type inference — type is independent of path
```

### 3.2 `config.rs` — schema.md parsing

```rust
pub struct SchemaConfig {
    pub custom_types: Vec<String>,  // additional types from schema.md
}
pub fn load_schema(wiki_root: &Path) -> Result<SchemaConfig>
```

### 3.3 Instructions

- `## frontmatter` section in `src/instructions.md`
- Condensed version of frontmatter-authoring.md with type taxonomy

**Deliverable:** `wiki ingest` validates frontmatter and warns on missing
recommended fields or deprecated `source-summary` type. LLM has frontmatter
authoring guide with full type taxonomy in context.

---

## Phase 4 — Search + Read + Index

**Goal:** `wiki search`, `wiki read`, `wiki list`, `wiki index` work.
Unified `PageRef` return type. `state.toml` committed on rebuild.

### 4.1 `search.rs` — unified return types + full frontmatter indexing

```rust
pub struct PageRef { slug, uri, title, score, excerpt: Option<String> }
pub struct PageList { pages: Vec<PageSummary>, total, page, page_size }
pub struct PageSummary { slug, uri, title, r#type, status, tags }
pub struct IndexStatus { wiki, path, built: Option<String>, pages, sections, stale }
pub struct IndexReport { wiki, pages_indexed, duration_ms }
```

All frontmatter fields indexed in tantivy schema (not just `slug`, `title`,
`tags`, `body`, `type`). Index stored in `~/.wiki/indexes/<name>/search-index/`.
`state.toml` written alongside the index on rebuild.
Staleness detection: compare `commit` field in `state.toml` vs `git HEAD`.

### 4.2 CLI + MCP

- `wiki search "<query>" --no-excerpt --top-k --include-sections --all`
- `wiki read <uri> --no-frontmatter --list-assets`
- `wiki read <uri>/<asset-filename>`
- `wiki list --type --status --page --page-size`
- `wiki index rebuild/status`
- MCP tools: `wiki_search`, `wiki_read`, `wiki_list`, `wiki_index_rebuild/status`

**Deliverable:** `wiki search "MoE scaling"` returns `Vec<PageRef>` with
`wiki://` URIs. `wiki read wiki://research/concepts/mixture-of-experts` returns
full page content.

---

## Phase 5 — Lint + Graph

**Goal:** `wiki lint` produces a `LintReport` and commits `LINT.md`.
`wiki graph` emits Mermaid or DOT.

### 5.1 `lint.rs`

```rust
pub struct MissingConnection { slug_a: String, slug_b: String, overlapping_terms: Vec<String> }
pub struct LintReport { orphans: Vec<PageRef>, missing_stubs: Vec<String>, empty_sections: Vec<String>, missing_connections: Vec<MissingConnection>, untyped_sources: Vec<String>, date: String }
pub fn lint(wiki_root: &Path) -> Result<LintReport>
pub fn lint_fix(wiki_root: &Path, config: &LintConfig, only: Option<&str>) -> Result<()>
```

`LINT.md` format from spec: all sections always present, empty sections show
`_No X found._`, `uri` and `path` in orphan/contradiction tables.
Missing connections section shows candidate pairs with overlapping terms.
Untyped sources section lists source pages with missing or deprecated
`source-summary` type. See [backlink-quality.md](specifications/llm/backlink-quality.md)
and [source-classification.md](specifications/core/source-classification.md).

### 5.2 `graph.rs`

```rust
pub struct GraphReport { nodes: usize, edges: usize, output: String, committed: bool }
pub fn build_graph(wiki_root: &Path, filter: &GraphFilter) -> DiGraph<PageNode, ()>
pub fn render_mermaid(graph: &DiGraph<PageNode, ()>) -> String
pub fn render_dot(graph: &DiGraph<PageNode, ()>) -> String
pub fn subgraph(graph: &DiGraph<PageNode, ()>, root: &str, depth: usize) -> DiGraph<PageNode, ()>
```

Output file gets minimal frontmatter with `status: generated`. Auto-committed
if output path is inside wiki root.

### 5.3 CLI + MCP

- `wiki lint`, `wiki lint fix --only missing-stubs|empty-sections --dry-run`
- `wiki graph --format --root --depth --type --output --dry-run`
- MCP tools: `wiki_lint`, `wiki_graph`

**Deliverable:** `wiki lint` writes `LINT.md` with orphans, missing stubs,
empty sections. `wiki graph` outputs Mermaid to stdout.

---

## Phase 6 — MCP Server + Session Bootstrap

**Goal:** `wiki serve` works with all registered wikis mounted. All MCP tools,
resources, and prompts from the spec live. `wiki instruct` structured by workflow.
Session bootstrap complete.

### 6.1 `mcp.rs` — complete

All tools from `specifications/features.md` MCP Tools table. Resources
namespaced by wiki name. Prompts: `ingest_source`, `research_question`,
`lint_and_fix`. `src/instructions.md` structured as:
`## help`, `## new`, `## ingest`, `## research`, `## lint`,
`## crystallize`, `## frontmatter`.

Remove: `wiki_context` tool, `analyse_contradiction` prompt, contradiction
references in all prompts.

### 6.2 Session bootstrap

See [session-bootstrap.md](specifications/llm/session-bootstrap.md).

- `schema.md` injected alongside instructions at MCP server start
- `## session-orientation` preamble in `src/instructions.md`
- `## linking-policy` preamble in `src/instructions.md`
- Every instruct workflow begins with orientation step

### 6.3 CLI

- `wiki serve [--sse [:<port>]] [--acp]`
- `wiki instruct [help|new|ingest|research|lint|crystallize|frontmatter]`

**Deliverable:** Claude Code can use all wiki tools via MCP. Crystallize
workflow guides session knowledge capture. Session bootstrap orients the LLM
from the wiki's current state. All registered wikis accessible via
`wiki://<name>/<slug>`.

---

## Phase 7 — ACP Transport

**Goal:** `wiki serve --acp` works as a native Zed / VS Code agent.

### 7.1 `acp.rs`

```rust
pub struct WikiAgent { spaces: Arc<Spaces>, sessions: Mutex<HashMap<String, AcpSession>> }
pub struct AcpSession { id, label, wiki: Option<String>, created_at, active_run }
impl Agent for WikiAgent { initialize, new_session, load_session, list_sessions, prompt, cancel }
```

Workflow dispatch: `ingest`, `research`, `lint`, `crystallize`. Instructions injected
at `initialize`. All registered wikis accessible per session.

### 7.2 Cargo.toml

```toml
agent-client-protocol       = "0.10"
agent-client-protocol-tokio = "0.1"
```

**Deliverable:** `wiki serve --acp` starts. Zed agent panel connects and
streams ingest/research workflows.

---

## Phase 8 — Claude Plugin

**Goal:** `.claude-plugin/` is complete and installable. All slash commands work.

- `plugin.json`, `marketplace.json`, `.mcp.json` updated to spec
- Commands: `help`, `init`, `new`, `ingest`, `research`, `crystallize`, `lint`
- `SKILL.md` updated — no contradiction workflow
- `wiki instruct <workflow>` returns correct step-by-step for all workflows

**Deliverable:** `claude plugin add /path/to/llm-wiki` → `/llm-wiki:ingest` works.

---

## Phase 9 — Documentation

**Goal:** Project documentation reflects the current design and is useful to
a new contributor or user arriving cold.

- `README.md` — rewrite around features and workflows, not implementation history
- `CONTRIBUTING.md` — rewrite to reflect the new module architecture, dev setup, and contribution workflow
- `CHANGELOG.md` — rewrite as a feature-oriented changelog (what the tool can do, not a git log)

**Deliverable:** A new contributor can read `README.md`, understand what llm-wiki
does, run it, and know where to start contributing.

---

## What Each Phase Unlocks

| After phase | You can… |
|-------------|----------|
| 1 | Initialize wikis, manage spaces and config |
| 2 | Validate, commit, and index files in the wiki tree; create pages and sections |
| 3 | Frontmatter validation on ingest, unified type taxonomy enforced, authoring guide in instructions |
| 4 | Search (with classification filter), read pages and assets, manage the index |
| 5 | Audit wiki structure (orphans, stubs, missing connections, unclassified sources), visualize concept graph |
| 6 | Use the wiki from Claude Code with full MCP access, crystallize sessions, session bootstrap |
| 7 | `wiki serve --acp` — native Zed / VS Code streaming agent |
| 8 | `/llm-wiki:ingest` and `/llm-wiki:crystallize` as one-command slash workflows |
| 9 | README, CONTRIBUTING, and CHANGELOG reflect the current design |
