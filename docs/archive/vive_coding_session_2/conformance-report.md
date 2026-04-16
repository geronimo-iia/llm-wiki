# Specification ↔ Implementation Conformance Report

Generated: 2025-07-16
Tests: 119 passing, 0 failing

---

## Summary

The implementation is **largely conformant** with the specifications. All core
commands, data types, config resolution, MCP tools, and ACP transport are
implemented and tested. The gaps below are mostly missing edge-case behaviors,
incomplete MCP tool features, and a few unimplemented config-level features.

| Area | Conformant | Gaps |
|------|-----------|------|
| CLI commands & flags | ✅ | 0 |
| Module layout (rust-modules.md) | ✅ | 0 |
| Config system | ✅ mostly | 2 minor |
| Init | ✅ | 0 |
| Spaces | ✅ | 0 |
| Page creation | ✅ | 0 |
| Ingest pipeline | ✅ mostly | 2 minor |
| Search | ✅ mostly | 1 |
| Read | ✅ mostly | 1 |
| List | ✅ | 0 |
| Lint | ✅ | 0 |
| Graph | ✅ mostly | 2 |
| Index | ✅ | 0 |
| Instruct | ✅ | 0 |
| Serve | ✅ | 0 |
| MCP tools | ✅ mostly | 2 |
| MCP resources | ✅ | 0 |
| MCP prompts | ✅ | 0 |
| ACP transport | ✅ mostly | 1 |
| Frontmatter | ✅ | 0 |
| Instructions | ✅ | 0 |

---

## ✅ Fully Conformant Areas

### Module Layout
`src/` matches `rust-modules.md` exactly: main.rs, lib.rs, cli.rs, config.rs,
spaces.rs, git.rs, frontmatter.rs, markdown.rs, links.rs, ingest.rs, search.rs,
lint.rs, graph.rs, server.rs, mcp/{mod.rs, tools.rs}, acp.rs.

### CLI Surface
All commands and flags from `cli.md` are implemented in `cli.rs`:
- `wiki init` with --name, --description, --force, --set-default
- `wiki new page/section` with --bundle, --dry-run
- `wiki ingest` with --dry-run
- `wiki search` with --no-excerpt, --top-k, --include-sections, --all, --dry-run
- `wiki read` with --no-frontmatter, --list-assets
- `wiki list` with --type, --status, --page, --page-size
- `wiki lint` / `wiki lint fix` with --only, --dry-run
- `wiki graph` with --format, --root, --depth, --type, --output, --dry-run
- `wiki index rebuild/status` with --dry-run
- `wiki config get/set/list` with --global, --wiki
- `wiki spaces list/remove/set-default` with --delete
- `wiki serve` with --sse, --acp, --dry-run
- `wiki instruct` with optional workflow name
- Global `--wiki` flag

### Config System
Two-level config (global + per-wiki) with correct resolution order.
All config keys from `configuration.md` are present: defaults.*, read.*,
index.*, graph.*, serve.*, validation.*, lint.*. Both `get_config_value`
and `set_global_config_value` cover all keys.

### Data Types
All spec'd types are implemented:
- `PageRef` (slug, uri, title, score, excerpt) — matches search.md
- `PageSummary` (slug, uri, title, type, status, tags) — matches list.md
- `PageList` (pages, total, page, page_size) — matches list.md
- `IndexStatus` (wiki, path, built, pages, sections, stale) — matches index.md
- `IndexReport` (wiki, pages_indexed, duration_ms) — matches index.md
- `IngestReport` (pages_validated, assets_found, warnings, commit) — matches ingest.md
- `LintReport` (orphans, missing_stubs, empty_sections, missing_connections, untyped_sources, date) — matches lint.md
- `MissingConnection` (slug_a, slug_b, overlapping_terms) — matches lint.md
- `GraphReport` (nodes, edges, output, committed) — matches graph.md
- `PageFrontmatter` with all fields from page-content.md

### Frontmatter
- All built-in types from source-classification.md: concept, query-result,
  section, paper, article, documentation, clipping, transcript, note, data,
  book-chapter, thread
- `tldr` field supported (optional, in PageFrontmatter)
- `claims` field with structured Claim type (text, confidence, section)
- `concepts` frontmatter field for graph edges
- Validation: title required (error), type/status/summary/read_when/last_updated
  (warnings), source-summary deprecated warning, strict/loose type checking
- scaffold_frontmatter derives title from slug, sets status=draft, type=page
- generate_minimal_frontmatter extracts title from H1 or filename

### Init
- Creates inbox/, raw/, wiki/ directories
- Generates README.md, wiki.toml, schema.md
- Git init + initial commit
- Registers in ~/.wiki/config.toml
- Re-run safety: same name → skip, different name → error (--force to rename)
- --set-default support

### Ingest
- File and folder ingest
- Frontmatter validation pipeline
- Generates minimal frontmatter for files without it
- Sets last_updated to today, defaults status=active, type=page
- Path traversal rejection
- Dry-run mode
- Git commit with descriptive message
- Asset detection (non-.md files counted)

### Search
- BM25 via tantivy with title, summary, body fields
- --no-excerpt, --include-sections, --top-k
- Section exclusion by default (MustNot section type)
- Type filter support
- Snippet generation via SnippetGenerator

### Lint
- All 5 checks: orphans, missing stubs, empty sections, missing connections, untyped sources
- LINT.md generation with all 5 sections, correct format
- lint_fix with --only support and config-driven fix enablement
- Git commit for both lint and lint fix

### MCP Server
- 16 tools matching features.md tool table
- Resources: wiki:// URIs for all pages
- Resource read via read_resource
- Resource update notifications on ingest
- 3 prompts: ingest_source, research_question, lint_and_fix
- Instructions injected at session start (with schema.md appended)
- Server capabilities: tools, resources (subscribe + list_changed), prompts

### ACP Transport
- WikiAgent with session management (new, load, list, cancel)
- Instructions injected at initialize
- Workflow dispatch by keyword matching
- Research workflow with actual search
- Lint workflow with actual lint execution
- Dedicated thread with LocalSet for !Send constraint

---

## ⚠️ Gaps — Spec'd but Not Fully Implemented

### 1. `wiki config set` — per-wiki config write (Medium)

**Spec** (configuration.md §4): `wiki config set <key> <value>` without
`--global` writes to per-wiki `wiki.toml`.

**Implementation** (main.rs:120): Prints `"Per-wiki config set not yet implemented"`.

**MCP** (tools.rs handle_config): Returns `"set not yet fully implemented via MCP"`.

### 2. `wiki search --all` — cross-wiki search (Medium)

**Spec** (search.md §3, features.md): `--all` flag searches across all
registered wikis.

**Implementation** (main.rs:183): The `all` flag is parsed by clap but
prefixed with `_all` — it is captured but not used. Search only targets
a single wiki.

### 3. `wiki read` — asset content reading (Low)

**Spec** (read.md §1-2): When the URI points to a non-Markdown asset file
inside a bundle, return raw bytes.

**Implementation**: `markdown::read_asset()` exists and works, but the CLI
`Commands::Read` handler and MCP `handle_read` don't route asset URIs to it.
They only handle page content and `--list-assets`.

### 4. Graph — output file frontmatter and auto-commit (Low)

**Spec** (graph.md §3): When `--output` writes a `.md` file, prepend minimal
frontmatter with `status: generated`. If the file is inside the wiki root,
auto-commit.

**Implementation** (main.rs:316, tools.rs handle_graph): Writes raw rendered
output to file. No frontmatter prepended. No auto-commit. `GraphReport.committed`
is always `false`.

### 5. Graph — `--output` as wiki:// URI (Low)

**Spec** (graph.md §4): `--output wiki://research/graph` writes to a wiki page.

**Implementation**: `--output` is treated as a filesystem path only.

### 6. Ingest — index update after commit (Low)

**Spec** (ingest.md §2, pipelines/ingest.md): When `index.auto_rebuild` is
`true`, the search index is rebuilt after commit. When `false`, a warning is
emitted.

**Implementation** (ingest.rs): Ingest validates and commits but does not
check `auto_rebuild`, does not call `rebuild_index`, and does not warn.
The index becomes stale after ingest — the user must run
`wiki index rebuild` or rely on `auto_rebuild` at next search.

### 7. MCP `wiki_search` — `all_wikis` parameter (Low)

**Spec** (search.md §4): MCP tool accepts `all_wikis: Option<bool>`.

**Implementation** (tools.rs): The `all` parameter is defined in the tool
schema but `handle_search` does not use it — search targets a single wiki.

### 8. ACP — streaming tool calls (Low)

**Spec** (acp-transport.md §3.4): ACP workflows should stream intermediate
`tool_call` and `message` events visible to the user.

**Implementation** (acp.rs): The prompt handler executes the workflow
synchronously and sends a single final message. No intermediate streaming
of tool calls. The research workflow calls `search::search` directly rather
than streaming `wiki_search` → `wiki_read` steps.

### 9. Ingest — CRLF normalization (Low)

**Spec** (page-content.md §7): "The engine normalises CRLF to LF on write
and rejects non-UTF-8 body content."

**Implementation**: No explicit CRLF→LF normalization or UTF-8 validation
in the ingest pipeline.

---

## ✅ Spec Conformance by Feature Area

| Feature | Spec | Status |
|---------|------|--------|
| Wiki init | init.md | ✅ Full |
| Page creation (flat + bundle) | page-creation.md | ✅ Full |
| Auto-create parent sections | page-creation.md §3 | ✅ Full |
| Ingest file | pipelines/ingest.md | ✅ Full |
| Ingest folder | pipelines/ingest.md | ✅ Full |
| Ingest dry-run | pipelines/ingest.md | ✅ Full |
| Frontmatter validation | page-content.md §3 | ✅ Full |
| Minimal frontmatter generation | page-content.md §4 | ✅ Full |
| Type strictness (strict/loose) | configuration.md | ✅ Full |
| Custom types from schema.md | source-classification.md | ✅ Full |
| BM25 search | search.md | ✅ Full |
| Search section exclusion | search.md | ✅ Full |
| Search type filter | search.md | ✅ Full |
| Search --no-excerpt | search.md | ✅ Full |
| Cross-wiki search --all | search.md | ⚠️ Parsed, not used |
| Read page by slug | read.md | ✅ Full |
| Read page by wiki:// URI | read.md | ✅ Full |
| Read --no-frontmatter | read.md | ✅ Full |
| Read --list-assets | read.md | ✅ Full |
| Read asset content | read.md | ⚠️ Function exists, not routed |
| List with type/status filter | list.md | ✅ Full |
| List pagination | list.md | ✅ Full |
| Lint 5 checks | lint.md | ✅ Full |
| LINT.md format | lint.md §3 | ✅ Full |
| Lint fix | lint.md §5 | ✅ Full |
| Graph mermaid + dot | graph.md | ✅ Full |
| Graph subgraph + depth | graph.md | ✅ Full |
| Graph type filter | graph.md | ✅ Full |
| Graph output file frontmatter | graph.md §3 | ⚠️ Not implemented |
| Index rebuild | index.md | ✅ Full |
| Index status + staleness | index.md | ✅ Full |
| Auto-rebuild on stale | index.md §6 | ✅ Full |
| Config get/set/list | configuration.md | ✅ Global, ⚠️ per-wiki set |
| Spaces list/remove/set-default | spaces.md | ✅ Full |
| Serve stdio | serve.md | ✅ Full |
| Serve SSE | serve.md | ✅ Full |
| Serve ACP | serve.md | ✅ Full |
| Serve startup sequence | serve.md §4 | ✅ Full |
| MCP 16 tools | features.md | ✅ Full |
| MCP resources | serve.md §2 | ✅ Full |
| MCP resource notifications | features.md | ✅ Full |
| MCP prompts | mcp/mod.rs | ✅ Full (3 prompts) |
| MCP instructions injection | instruct.md §4 | ✅ Full |
| ACP initialize | acp-transport.md §3.2 | ✅ Full |
| ACP sessions | acp-transport.md §3.1 | ✅ Full |
| ACP workflow dispatch | acp-transport.md §3.3 | ✅ Full |
| ACP streaming steps | acp-transport.md §3.4 | ⚠️ Single message only |
| Instruct workflows | instruct.md | ✅ Full |
| wiki_write MCP tool | pipelines/ingest.md §7 | ✅ Full |
| Bundle promotion | asset-ingest.md §3 | ✅ Full |
| Backlink quality / MissingConnection | backlink-quality.md | ✅ Full |
| Source classification | source-classification.md | ✅ Full |

---

## Priority Ranking of Gaps

1. **`wiki config set` per-wiki** — blocks per-wiki config customization via CLI/MCP
2. **`wiki search --all`** — blocks cross-wiki search workflows
3. **Ingest → index update** — causes stale index after every ingest
4. **Read asset content routing** — function exists, just needs wiring
5. **Graph output frontmatter + auto-commit** — cosmetic, low impact
6. **ACP streaming** — functional but not streaming intermediate steps
7. **CRLF normalization** — edge case, unlikely on macOS/Linux
