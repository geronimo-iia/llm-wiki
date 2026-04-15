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
├── registry.rs         # Registry, WikiEntry, resolve_name(), resolve_uri()
├── git.rs              # init_repo(), commit(), current_head(), diff_last()
├── markdown.rs         # frontmatter parse/write, scaffold, slug helpers,
│                       # promote_to_bundle, read_page, list_assets, read_asset
├── analysis.rs         # Enrichment, QueryResult, Asset, Analysis — new schema
├── ingest.rs           # Input, DirectIngestOptions, ingest()
├── integrate.rs        # integrate_direct_file/folder, integrate_enrichment,
│                       # integrate_query_result, create_page, create_section,
│                       # write_assets, IngestReport
├── search.rs           # PageRef, PageList, tantivy index, search(), list(),
│                       # IndexStatus, IndexReport, index-status.toml
├── lint.rs             # LintReport, orphan/stub/section detection, LINT.md
├── graph.rs            # petgraph build, render_mermaid/dot, GraphReport
├── server.rs           # rmcp WikiServer — all MCP tools, resources, prompts
└── acp.rs              # WikiAgent, AcpSession, workflow dispatch
```

---

## Phase 1 — Foundation: Schema + Config + Registry

**Goal:** The new data model compiles. Config and registry load correctly.
`wiki init`, `wiki config`, and `wiki registry` work end-to-end.

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

### 1.3 `registry.rs` — `wiki://` URI resolution

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
- `wiki registry list/remove/set-default`
- MCP tools: `wiki_init`, `wiki_config`, `wiki_registry_*`

**Deliverable:** `cargo test` green. `wiki init` creates a wiki and registers it.

---

## Phase 2 — Core Write Loop: Ingest + Page Creation

**Goal:** `wiki ingest <path> --target <uri>` writes pages and assets to the
wiki. `wiki new page/section <uri>` creates scaffolded pages.

### 2.1 `markdown.rs` additions

```rust
pub fn generate_minimal_frontmatter(title: &str, slug: &str) -> PageFrontmatter
pub fn scaffold_frontmatter(slug: &str) -> PageFrontmatter  // for wiki new
pub fn read_page(slug: &str, wiki_root: &Path, no_frontmatter: bool) -> Result<String>
pub fn list_assets(slug: &str, wiki_root: &Path) -> Result<Vec<String>>  // wiki:// URIs
pub fn read_asset(slug: &str, filename: &str, wiki_root: &Path) -> Result<Vec<u8>>
```

`PageFrontmatter` updated: remove `contradictions` field, add `claims`.

### 2.2 `integrate.rs` — ingest

```rust
pub fn integrate_file(path: &Path, target: &ResolvedTarget, options: &IngestOptions, wiki_root: &Path) -> Result<IngestReport>
pub fn integrate_folder(path: &Path, target: &ResolvedTarget, options: &IngestOptions, wiki_root: &Path) -> Result<IngestReport>
pub fn write_assets(assets: &[Asset], wiki_root: &Path) -> Result<usize>
pub fn create_page(uri: &str, bundle: bool, registry: &Registry) -> Result<String>
pub fn create_section(uri: &str, registry: &Registry) -> Result<String>
```

`IngestReport`: `pages_written`, `assets_written`, `bundles_created`, `commit`.

### 2.3 `ingest.rs`

```rust
pub enum Input { Direct(PathBuf), }
pub struct IngestOptions { target: Option<String>, update: bool }
```

### 2.4 CLI + MCP

- `wiki ingest <path> --target --update --dry-run`
- `wiki new page <uri> --bundle --dry-run`
- `wiki new section <uri> --dry-run`
- MCP tools: `wiki_ingest`, `wiki_new_page`, `wiki_new_section`

**Deliverable:** `wiki ingest ~/agent-skills/semantic-commit/ --target wiki://research/skills`
writes pages + assets, commits.

---

## Phase 3 — Frontmatter Validation + Type Taxonomy

**Goal:** Engine validates frontmatter on ingest. Unified type taxonomy
(knowledge types + source types + custom) enforced. Frontmatter authoring
guide in instructions.

### 3.1 `markdown.rs` — validation

```rust
pub fn validate_frontmatter(fm: &PageFrontmatter, schema: &SchemaConfig) -> Result<Vec<Warning>>
// Checks: required fields present, type in built-in + custom list, source-summary deprecated
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
Unified `PageRef` return type. `index-status.toml` committed on rebuild.

### 4.1 `search.rs` — unified return types + full frontmatter indexing

```rust
pub struct PageRef { slug, uri, title, score, excerpt: Option<String> }
pub struct PageList { pages: Vec<PageSummary>, total, page, page_size }
pub struct PageSummary { slug, uri, title, r#type, status, tags }
pub struct IndexStatus { wiki, path, built: Option<String>, pages, sections, stale }
pub struct IndexReport { wiki, pages_indexed, duration_ms }
```

All frontmatter fields indexed in tantivy schema (not just `slug`, `title`,
`tags`, `body`, `type`). `index-status.toml` written and committed on rebuild.
Staleness detection: compare `commit` field vs `git HEAD`.

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
`source-summary` type. See [backlink-quality.md](specifications/backlink-quality.md)
and [source-classification.md](specifications/source-classification.md).

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

### 6.1 `server.rs` — complete

All tools from `specifications/features.md` MCP Tools table. Resources
namespaced by wiki name. Prompts: `ingest_source`, `research_question`,
`lint_and_enrich`. `src/instructions.md` structured as:
`## help`, `## new`, `## ingest`, `## research`, `## lint`,
`## crystallize`, `## frontmatter`.

Remove: `wiki_context` tool, `analyse_contradiction` prompt, contradiction
references in all prompts.

### 6.2 Session bootstrap

See [session-bootstrap.md](specifications/session-bootstrap.md).

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
pub struct WikiAgent { registry: Arc<Registry>, sessions: Mutex<HashMap<String, AcpSession>> }
pub struct AcpSession { id, label, wiki: Option<String>, created_at, active_run }
impl Agent for WikiAgent { initialize, new_session, load_session, list_sessions, prompt, cancel }
```

Workflow dispatch: `ingest`, `research`, `lint`, `enrich`. Instructions injected
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
- Commands: `help`, `init`, `new`, `ingest`, `research`, `enrich`, `lint`
- `SKILL.md` updated — no contradiction workflow
- `wiki instruct <workflow>` returns correct step-by-step for all workflows

**Deliverable:** `claude plugin add /path/to/llm-wiki` → `/llm-wiki:ingest` works.

---

## What Each Phase Unlocks

| After phase | You can… |
|-------------|----------|
| 1 | Initialize wikis, manage registry and config |
| 2 | Ingest any file or folder, create pages and sections |
| 3 | Frontmatter validation on ingest, unified type taxonomy enforced, authoring guide in instructions |
| 4 | Search (with classification filter), read pages and assets, manage the index |
| 5 | Audit wiki structure (orphans, stubs, missing connections, unclassified sources), visualize concept graph |
| 6 | Use the wiki from Claude Code with full MCP access, crystallize sessions, session bootstrap |
| 7 | `wiki serve --acp` — native Zed / VS Code streaming agent |
| 8 | `/llm-wiki:ingest` and `/llm-wiki:crystallize` as one-command slash workflows |
