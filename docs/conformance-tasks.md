# Conformance Fix Tasks

Ordered by impact on the core DKR loop. Each task is self-contained and
independently shippable.

---

## Task 1 — Ingest: index update after commit (respects `auto_rebuild`)

**Gap:** ingest.md §2 says the search index is rebuilt after commit when
`index.auto_rebuild` is `true`, and a warning is emitted when `false`.
Currently ingest does neither — no rebuild, no warning.

**Why first:** When `auto_rebuild` is enabled, the ingest→search→read→write
loop (core DKR workflow) is silently broken. When disabled, the user gets no
feedback that the index is stale.

**Spec update:** ingest.md §2 "Index" section updated to document the
`auto_rebuild` dependency and the warning behavior.

### Code changes

Keep index rebuild in the callers (not in `ingest.rs`) — `ingest.rs` stays
focused on validation + git. The callers already have access to resolved
config.

- `src/main.rs` — in `Commands::Ingest`, after a successful non-dry-run
  ingest:
  - If `resolved.index.auto_rebuild` is `true`: call
    `search::rebuild_index()`. Log rebuild failure as a warning (non-fatal —
    the commit already succeeded).
  - If `resolved.index.auto_rebuild` is `false`: print
    `"warning: search index is stale — run `wiki index rebuild`"`.
- `src/mcp/tools.rs` — in `handle_ingest`, same logic after successful
  ingest:
  - If `auto_rebuild`: call `rebuild_index`, add rebuild failure as warning
    in the report.
  - If not `auto_rebuild`: append a warning string to `report.warnings`:
    `"search index is stale — run wiki_index_rebuild"`.

### Tests

- `tests/ingest.rs` — two new tests (both use a helper that sets up a repo
  + index):
  - `ingest_rebuilds_index_when_auto_rebuild_enabled` — set up repo, build
    initial index, ingest a new page, assert `index_status().stale == false`
    (index was rebuilt).
  - `ingest_warns_when_auto_rebuild_disabled` — ingest a page with
    `auto_rebuild = false`, assert `index_status().stale == true` and the
    CLI output contains the warning string.
- Existing tests unchanged.

### Exit criteria

- `auto_rebuild = true`: `wiki ingest foo.md` → `wiki search "foo"` returns
  the page without `wiki index rebuild` in between.
- `auto_rebuild = false`: `wiki ingest foo.md` prints the stale index
  warning. `wiki search "foo"` does not find the page until
  `wiki index rebuild` is run.
- MCP `wiki_ingest` behaves identically (rebuild or warning in report).
- Spec ingest.md §2 documents the `auto_rebuild` dependency.
- `cargo test` passes.

---

## Task 2 — `wiki config set` per-wiki

**Gap:** configuration.md §4 says `wiki config set <key> <value>` without
`--global` writes to per-wiki `wiki.toml`. Currently prints a stub message.
MCP `handle_config` returns a stub string for `set`.

### Code changes

- `src/config.rs` — add `save_wiki(config: &WikiConfig, wiki_root: &Path)`.
  Mirrors `save_global`: serialize to TOML, write to `wiki.toml`.
  Add `set_wiki_config_value(wiki_cfg: &mut WikiConfig, key: &str, value: &str)`.
  Only keys with `global / per-wiki` scope in the spec are writable:
  `defaults.*`, `validation.*`, `lint.*`. Reject `global.*`, `serve.*`,
  `read.*`, `index.*`, `graph.*` with an error (global-only keys).
- `src/main.rs` — in `ConfigAction::Set`, when `!is_global`: resolve wiki,
  load `WikiConfig`, call `set_wiki_config_value`, call `save_wiki`.
- `src/mcp/tools.rs` — in `handle_config` `"set"` branch: implement the
  same logic. Distinguish global vs per-wiki via the `global` arg.

### Tests

- `tests/config.rs` — new tests:
  - `save_wiki_roundtrips` — save a WikiConfig, reload, assert values match.
  - `set_wiki_config_value_sets_defaults_search_top_k` — set key, assert
    field updated.
  - `set_wiki_config_value_rejects_global_only_key` — set `serve.sse`,
    assert error.

### Exit criteria

- `wiki config set defaults.search_top_k 25 --wiki research` → `wiki.toml`
  updated, `wiki config get defaults.search_top_k --wiki research` returns 25.
- `wiki config set serve.sse true --wiki research` → error (global-only key).
- MCP `wiki_config { action: "set", key: "defaults.search_top_k", value: "25" }`
  writes to per-wiki config.
- `cargo test` passes.

---

## Task 3 — `wiki search --all` cross-wiki search

**Gap:** search.md §3 defines `--all` to search across all registered wikis.
The flag is parsed by clap (`_all`) but unused. Same for MCP `all` parameter.

### Code changes

- `src/search.rs` — add `search_all(query, opts, wikis: &[(String, PathBuf)]) -> Vec<PageRef>`.
  Iterates each wiki's index, calls `search()`, merges results, re-sorts by
  score descending, truncates to `top_k`.
- `src/main.rs` — in `Commands::Search`, when `all` is true: load all wiki
  entries from global config, build `(name, index_path)` pairs, call
  `search_all`. Remove the `_all` prefix.
- `src/mcp/tools.rs` — in `handle_search`, when `all` arg is true: same
  logic.

### Tests

- `tests/search.rs` — new tests:
  - `search_all_merges_results_from_multiple_wikis` — create two temp wikis
    with different pages, call `search_all`, assert results from both appear.
  - `search_all_sorts_by_score_descending` — assert merged results are
    score-ordered.

### Exit criteria

- `wiki search "MoE" --all` returns results from all registered wikis with
  correct `wiki://` URIs.
- MCP `wiki_search { query: "MoE", all: true }` returns cross-wiki results.
- `cargo test` passes.

---

## Task 4 — Read asset content routing

**Gap:** read.md §1-2 says asset URIs return raw bytes. `markdown::read_asset()`
exists but CLI and MCP handlers don't route to it.

**Spec update:** read.md §2 "Slug vs Asset Resolution" added to document the
engine-internal detection algorithm.

### Resolution algorithm (from spec §2)

1. Try `resolve_slug(slug)` → success: it's a page, return content.
2. If that fails, check the **last path segment** for a non-`.md` extension:
   - No extension → error: page not found (original error).
   - Has non-`.md` extension → split at last `/` into `(parent_slug, filename)`.
     Call `read_asset(parent_slug, filename)` → success: return bytes.
     Failure: error: asset not found.

Why this is solid:
- Page slugs never have extensions (engine strips `.md` on derivation).
- Assets always have non-`.md` extensions.
- Split at last `/` maps exactly to bundle directory structure.
- `concepts/moe/index.md` resolves as page at step 1 (never reaches step 2).
- `concepts/v2.0-release` has no extension in the last segment, so step 2
  correctly falls through.

### Code changes

- `src/markdown.rs` — add `resolve_slug_or_asset(slug, wiki_root) -> ReadTarget`
  enum that returns either `Page(PathBuf)` or `Asset(String, String)` (parent
  slug + filename). Encapsulates the two-step resolution.
- `src/main.rs` — in `Commands::Read`: use `resolve_slug_or_asset`. If
  `Asset`, call `read_asset` and write raw bytes to stdout.
- `src/mcp/tools.rs` — in `handle_read`: same detection. If asset, return
  base64-encoded content for binary, or text content for UTF-8 assets.

### Tests

- `tests/markdown.rs` — new tests:
  - `resolve_slug_or_asset_returns_page_for_valid_slug` — flat and bundle.
  - `resolve_slug_or_asset_returns_asset_for_bundle_file` — e.g.
    `concepts/moe/diagram.png` → `Asset("concepts/moe", "diagram.png")`.
  - `resolve_slug_or_asset_returns_error_for_missing_page_without_extension`
    — `concepts/missing` → error.
  - `resolve_slug_or_asset_returns_error_for_missing_asset` —
    `concepts/moe/missing.png` → error.

### Exit criteria

- `wiki read concepts/moe/diagram.png` returns the file content.
- MCP `wiki_read { uri: "concepts/moe/diagram.png" }` returns content.
- `wiki read concepts/moe` still returns the page (step 1 wins).
- `wiki read concepts/missing` returns "page not found" error.
- Spec read.md §2 documents the resolution algorithm.
- `cargo test` passes.

---

## Task 5 — Graph output frontmatter + auto-commit

**Gap:** graph.md §3 says `--output` to a `.md` file prepends frontmatter
with `status: generated`. If inside wiki root, auto-commit. Currently writes
raw output, no frontmatter, `committed` always false.

Auto-commit follows the spec as-is: always commit when output is inside the
wiki root. A future improvement may add a config flag to gate this.

### Code changes

- `src/graph.rs` — add `wrap_graph_md(rendered: &str, format: &str, filter: &GraphFilter) -> String`.
  Prepends YAML frontmatter block with title, generated timestamp, format,
  root, depth, types, `status: generated`.
- `src/main.rs` — in `Commands::Graph`, when `output` ends with `.md`:
  call `wrap_graph_md`, write result. If output path is inside wiki root,
  call `git::commit`. Set `committed` in report.
- `src/mcp/tools.rs` — same logic in `handle_graph`.

### Tests

- `tests/graph.rs` — new tests:
  - `graph_output_md_has_frontmatter` — write graph to `.md`, parse output,
    assert frontmatter contains `status: generated`.
  - `graph_output_md_inside_wiki_commits` — write to wiki root, assert git
    HEAD changed.

### Exit criteria

- `wiki graph --output wiki/graph.md` produces a file with valid frontmatter
  and `status: generated`.
- If output is inside wiki root, git commit is created.
- `GraphReport.committed` is `true` when auto-committed.
- `cargo test` passes.

---

## Task 6 — Ingest: CRLF normalization

**Gap:** page-content.md §7 says "the engine normalises CRLF to LF on write
and rejects non-UTF-8 body content."

### Code changes

- `src/ingest.rs` — in `process_file()`, after `read_to_string` (which
  already rejects non-UTF-8), normalize CRLF→LF: `content.replace("\r\n", "\n")`.
  Apply before frontmatter parsing.

### Tests

- `tests/ingest.rs` — new test: `ingest_normalizes_crlf_to_lf` — write a
  page with `\r\n` line endings, ingest, read back, assert no `\r` remains.

### Exit criteria

- A file with CRLF line endings is normalized to LF after ingest.
- `cargo test` passes.

---

## Task 7 — MCP `wiki_search` `all_wikis` parameter

**Gap:** Depends on Task 3. The MCP tool schema defines `all` but
`handle_search` ignores it.

**Note:** This is resolved as part of Task 3. No separate task needed — listed
here for traceability.

---

## Task 8 — MCP `wiki_config` set (per-wiki)

**Gap:** Depends on Task 2. MCP `handle_config` returns stub for `set`.

**Note:** This is resolved as part of Task 2. No separate task needed.

---

## Task 9 — ACP streaming tool calls

**Gap:** acp-transport.md §3.4 says workflows should stream intermediate
`tool_call` and `message` events. Currently sends a single final message.

### Code changes

- `src/acp.rs` — in the `prompt()` method, for the research workflow: send
  intermediate messages via `self.send_message()` before and after each step
  (search, read). E.g. "Searching for: {query}..." → results → "Reading
  top result..." → content.
- Same pattern for lint and ingest workflows.

### Tests

- `tests/acp.rs` — verify that the prompt handler sends multiple messages
  (requires capturing the `update_tx` channel output). Assert message count > 1
  for a research workflow.

### Exit criteria

- ACP research workflow sends intermediate streaming messages visible to the
  client.
- `cargo test` passes.

---

## Summary

| # | Task | Priority | Effort | Dependencies |
|---|------|----------|--------|--------------|
| 1 | Ingest → index rebuild | High | Small | None |
| 2 | Config set per-wiki | High | Medium | None |
| 3 | Search --all cross-wiki | Medium | Medium | None |
| 4 | Read asset routing | Low | Small | None |
| 5 | Graph output frontmatter | Low | Small | None |
| 6 | CRLF normalization | Low | Tiny | None |
| 7 | MCP search --all | Low | — | Task 3 |
| 8 | MCP config set | — | — | Task 2 |
| 9 | ACP streaming | Low | Medium | None |
