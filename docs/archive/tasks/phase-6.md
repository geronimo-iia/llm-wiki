# Phase 6 — Multi-wiki + SSE

Goal: one `wiki` process manages multiple repos.
Remote agents connect via SSE.

---

## `registry.rs`

- [x] `WikiRegistry::load(config_path: &Path) -> Result<WikiRegistry>` — parse `[[wikis]]` from `~/.wiki/config.toml`
- [x] `resolve(name: Option<&str>) -> Result<&WikiConfig>` — return default wiki if name is `None`
- [x] Error if no default wiki configured and name is `None`
- [x] Error if named wiki not found — list available names in message
- [x] `~/.wiki/config.toml` schema: `name`, `path`, `default` (bool), `remote` (optional git remote URL)
- [x] `wiki init --register` — add newly created wiki to `~/.wiki/config.toml`

## CLI — `--wiki` flag

- [x] Thread `--wiki <name>` global flag through to `registry.resolve()` on all subcommands:
  `ingest`, `search`, `context`, `lint`, `list`, `contradict`, `graph`, `diff`
- [x] Default: resolve default wiki (no flag needed for single-wiki setups)

## Cross-wiki search

- [x] `search::search_all(registry: &WikiRegistry, query: &str, limit: usize) -> Result<Vec<SearchResultWithWiki>>`
  — fan out tantivy queries to all registered wikis, merge results by score
- [x] `SearchResultWithWiki` — adds `wiki_name: String` field to `SearchResult`
- [x] `wiki search --all "<term>"` — use `search_all`, display wiki name in results table

## SSE transport

- [x] `wiki serve --sse :<port>` — start MCP server on SSE transport
- [x] Multiple clients can connect simultaneously
- [x] Each client gets an independent session (no shared mutable state)
- [x] Graceful shutdown on Ctrl-C

## MCP multi-wiki

- [x] All MCP tools accept `wiki: Option<String>` — resolve via registry
- [x] Resources namespaced by wiki: `wiki://{wiki_name}/{type}/{slug}`
- [x] `wiki_search` with `all_wikis: true` calls `search_all`
- [x] MCP server mounts all registered wikis at startup (resource templates for each)

## Tests

**Test file:** `tests/registry.rs`

### Unit tests

- [x] `registry::load` — two wikis configured → both resolved by name
- [x] `registry::resolve(None)` — returns wiki with `default = true`
- [x] `registry::resolve(Some("work"))` — returns wiki named "work"
- [x] `registry::resolve(Some("unknown"))` — error listing available names
- [x] `registry::resolve(None)` with no default configured → error
- [x] `search_all` — two wikis, term present in one → result has correct `wiki_name`
- [x] `search_all` — term present in both → results from both, merged by score

### Integration tests

- [x] `wiki --wiki research ingest analysis.json` — pages written to research wiki root, not work wiki root
- [x] `wiki --wiki work search "term"` — searches work wiki index only
- [x] `wiki search --all "term"` — returns results from both wikis with wiki label
- [x] `wiki serve --sse :0` (random port) — server starts, second client connects after first
- [x] MCP tool `wiki_ingest` with `wiki: "work"` — ingests into work wiki
- [x] MCP resource `wiki://research/concepts/foo` — reads from research wiki root

### Config tests

- [x] `~/.wiki/config.toml` with missing `path` field → clear error naming the wiki
- [x] `~/.wiki/config.toml` with `path` pointing to non-existent directory → clear error
- [x] Two wikis both marked `default = true` → error on load

## Changelog

- [x] `CHANGELOG.md` — add Phase 6 section: multi-wiki registry, `--wiki` flag, `--all` search, SSE transport

## README

- [x] **Multi-wiki** section — `~/.wiki/config.toml` example with two wikis, `--wiki` flag, `wiki search --all`
- [x] **SSE** section — `wiki serve --sse :<port>`, when to use SSE vs stdio
- [x] CLI reference — add `--wiki <name>` global flag note, `--all` flag on `wiki search`

## Dev documentation

- [x] `docs/dev/multi-wiki.md` — `~/.wiki/config.toml` schema, `--wiki` flag, `--all` search, SSE setup and client connection
- [x] `docs/dev/multi-wiki.md` — cross-wiki contradictions surfaced in `search --all` output
- [ ] Update `.claude-plugin/.mcp.json` — document `--wiki` arg option for non-default wiki setups
- [x] Update `docs/dev/architecture.md` — mark Phase 6 complete, final module map
