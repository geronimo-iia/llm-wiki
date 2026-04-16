# Changelog

## [Unreleased]

### Added
- (list features as they land)

## [0.1.0] — TBD

### Added
- `wiki init` — initialize a new wiki repository with default structure and git repo
- `wiki ingest` — validate, commit, and index files already in the wiki tree
- `wiki new page` / `wiki new section` — create scaffolded pages and sections
- `wiki search` — full-text BM25 search with PageRef return type (`wiki://` URIs)
- `wiki read` — fetch page content by slug or `wiki://` URI
- `wiki list` — paginated page enumeration with type and status filters
- `wiki index rebuild` / `wiki index status` — tantivy index management
- `wiki lint` — structural audit with LINT.md output (orphans, missing stubs, empty sections)
- `wiki lint fix` — auto-fix missing stubs and empty sections
- `wiki graph` — concept graph in Mermaid or DOT format with subgraph support
- `wiki serve` — MCP server on stdio and SSE for Claude Code and other agents
- `wiki serve --acp` — ACP agent with session-oriented streaming workflows
- `wiki instruct` — embedded workflow instructions for LLMs (ingest, research, lint, crystallize, frontmatter)
- `wiki config` — two-level configuration (global + per-wiki) with get/set/list
- `wiki spaces` — multi-wiki management (list, remove, set-default)
- Frontmatter validation with built-in and custom type taxonomy
- Claude Code plugin with slash commands (`/llm-wiki:ingest`, `/llm-wiki:research`, `/llm-wiki:lint`, etc.)
- MCP resources namespaced by wiki name with update notifications on ingest
- Session bootstrap: instructions + schema.md injected at MCP/ACP session start
