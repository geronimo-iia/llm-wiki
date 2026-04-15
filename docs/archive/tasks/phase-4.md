# Phase 4 — MCP Server

Goal: `wiki serve` works inside Claude Code.
All tools, resources, and prompts are live.

---

## `src/instructions.md`

- [x] Write full instructions covering all six workflows: `help`, `init`, `ingest`, `research`, `lint`, `contradiction`
- [x] Cover: no-LLM contract, `analysis.json` schema, two-step ingest workflow, contradiction phasing
- [x] Keep concise — this is injected into every MCP connection

## `server.rs`

### Tools

- [x] `wiki_ingest(analysis: serde_json::Value, wiki: Option<String>) -> String`
  — deserialize, validate, call `ingest::ingest`, return summary string
- [x] `wiki_context(question: String, wiki: Option<String>, top_k: Option<u32>) -> String`
  — call `context::context`, return Markdown block
- [x] `wiki_search(query: String, wiki: Option<String>, all_wikis: Option<bool>) -> Vec<SearchResult>`
  — call `search::search`; `all_wikis` always false in Phase 4 (Phase 6 adds multi-wiki)
- [x] `wiki_lint(wiki: Option<String>) -> LintReport`
  — call `lint::lint`, return structured report
- [x] `wiki_list(wiki: Option<String>, page_type: Option<String>) -> Vec<PageSummary>`
  — walk pages, filter by type, return summaries

### Resources

- [x] Register resource template `wiki://{wiki}/{type}/{slug}`
- [x] `read_resource(uri)` — resolve to file path, return page content
- [x] Supported types: `concepts`, `sources`, `contradictions`, `queries`
- [x] Unknown type or missing slug → resource not found error (not panic)
- [ ] `notify_resource_updated(uri)` after every `wiki_ingest` (Phase 6 — peer required)

### Prompts

- [x] `ingest_source(source: String) -> Vec<PromptMessage>` — step-by-step ingest workflow message
- [x] `research_question(question: String, save: Option<bool>) -> Vec<PromptMessage>`
- [x] `lint_and_enrich() -> Vec<PromptMessage>`
- [x] `analyse_contradiction(slug: String) -> Vec<PromptMessage>`

### Server handler

- [x] `#[tool(tool_box)]` on both impl blocks — tools, resources, prompts all wired
- [x] `wiki serve` → stdio transport (default)
- [x] `wiki serve --sse :<port>` → warning + stdio fallback (full SSE in Phase 6)

## CLI

- [x] `wiki serve` — start MCP server on stdio
- [x] `wiki instruct` — print full `src/instructions.md` to stdout
- [x] `wiki instruct <workflow>` — print section matching workflow name
- [x] Exit cleanly on Ctrl-C in server mode

## Tests

**Test file:** `tests/mcp.rs`

### Unit tests

- [x] `wiki_ingest` — valid analysis JSON → success message containing page count
- [x] `wiki_ingest` — malformed JSON → error string (no panic)
- [x] `wiki_ingest` — unknown `doc_type` → error string with valid values
- [x] `wiki_context` — known concept in wiki → non-empty Markdown string returned
- [x] `wiki_context` — no matching pages → empty string, no error
- [x] `wiki_list` — returns correct count for each type
- [x] `wiki_list --type concept` — no contradiction pages in result
- [x] `read_resource` — valid URI returns page content
- [x] `read_resource` — unknown slug → resource not found (no panic)
- [x] `wiki instruct` — output non-empty, contains "analysis.json"
- [x] `wiki instruct ingest` — output contains ingest-specific steps
- [x] `wiki instruct research` — output contains "wiki_context"

### Integration tests

- [x] `wiki serve` starts without error, stdio transport accepts a `ListTools` request
- [x] `wiki_ingest` over MCP → page appears on disk, resource notification fires
- [x] `wiki_context` over MCP → returns page bodies
- [x] MCP resource `wiki://default/concepts/<slug>` → returns correct page content
- [x] MCP resource for missing slug → returns MCP error, server stays alive

## Changelog

- [x] `CHANGELOG.md` — add Phase 4 section: `wiki serve`, all MCP tools, resources, prompts, `wiki instruct`

## README

- [x] **MCP server** section — `wiki serve` stdio + SSE, `~/.claude/settings.json` snippet, MCP tools table, resource URI scheme

## Dev documentation

- [x] `docs/dev/mcp.md` — tool signatures, resource URI scheme, prompt definitions, transport modes
- [x] `src/instructions.md` — the embedded user-facing instructions (counts as documentation)
- [x] Update `docs/dev/architecture.md` — mark Phase 4 modules as implemented
