# Pre-1.0.0 Backlog

All items from the 2026-08-19 review evaluation to fix before tagging v1.0.0.
D5-3 and D9-1 (atomic rollback in spaces.rs) need a plan — tracked below.

## Easy wins

- [x] **D1-1** `src/watch.rs:156` — wrap `index_manager.update()` in `spawn_blocking` (same as `RebuildIndex` branch at line 108)
- [x] **D2-1** `tests/spaces.rs:246` — add config read-back after failed `set_default_wiki`: `assert_eq!(config.global.default_wiki, "", "…")`
- [x] **D2-2** `tests/handlers.rs:268` — remove `search_index_error_includes_rebuild_hint` (asserts nothing; wrong code path)
- [x] **D2-3** `tests-integration/engine/test_search.py:34` — write fixture page with "Layer 1:" before colon search, assert `len(data["results"]) > 0`
- [x] **D2-7** `tests-integration/engine/test_spaces.py` — add `test_spaces_set_default_unknown_wiki_does_not_mutate_config`
- [x] **D2-8** `tests/spaces.rs:251` — add failure message to bare `is_err()` assertion
- [x] **D3-1** `src/lib.rs:1` — append embed example link to crate-level doc comment
- [x] **D3-2** `src/cli.rs:35,41,47` — add `#[command(about = "…")]` to all subcommand group variants
- [x] **D3-3** `src/mcp/tools.rs:244` — fix `wiki_index_status` trigger phrase: references stale `"degraded"` flat string
- [x] **D3-4** `src/mcp/tools.rs:100` — append implication sentence to `wiki_spaces_set_default` description
- [x] **D4-4** `src/config.rs:103` — validate `tokenizer` against `["en_stem", "raw", "simple", "default"]` before persisting
- [x] **D5-4** `src/graph.rs:265` — add doc comment on `graph_cache` field asserting `petgraph_live` concurrency guarantee
- [x] **D6-6** `src/graph.rs:956` — sort `external_refs` in place; remove `let mut sorted = external_refs.clone()`
- [x] **D6-7** `src/ingest.rs:203` — after write at line 202, use `normalize_line_endings(&redacted)` directly instead of re-reading from disk
- [x] **D8-4** `src/index_manager.rs:330` — promote invalid-path skip log from `debug!` to `warn!`
- [x] **D8-5** `src/index_manager.rs:559` — append `"; run wiki_index_rebuild to recover"` to the stale degraded message

## Performance

- [x] **D6-1** `src/ops/lint.rs:283` — `rule_broken_link`: collect all known slugs into `HashSet` via single `AllQuery` pass; replace per-link `slug_exists()` with O(1) set membership
- [x] **D6-2** `src/mcp/mod.rs:101` — wrap `tools::call(...)` dispatch in `tokio::task::block_in_place`
- [x] **D6-3** `src/search.rs:582` — replace `collect_facet` doc-fetch loop with Tantivy `FacetCollector` or accumulate during main result pass (needs plan)
- [x] **D6-4** `src/graph.rs:839` — merge 4 separate `node_indices()` passes in `render_llms` into one
- [x] **D6-5** `src/graph.rs:1092` — store `slug_to_node: HashMap<String, NodeIndex>` on `WikiGraph`; requires wrapper struct (`WikiGraph` is a type alias, not a struct — needs plan)
- [x] **D6-8** `src/graph.rs:1286,1336` — extract `ensure_community_data()` shared by `get_cached_community_map` and `get_cached_community_stats`

## Correctness

- [x] **D1-2** `src/mcp/mod.rs:45–67` — release `RwLockReadGuard` after copying `(wiki_name, wiki_root)` pairs; walk outside the lock
- [x] **D4-3** `src/slug.rs:44` — doc comment on `Slug::resolve` documenting symlink containment expectation; `Slug` construction already blocks `..` traversal
- [x] **D9-2** `tests/graph_cache.rs` — `graph_cache_is_cold_after_engine_restart`: fresh `WikiEngine` always starts with cold cache (Invariant 1 regression)

## Security / concurrency (direct fixes — done)

- [x] **D8-1** `src/index_manager.rs:57` — `#[serde(skip)]` on `IndexStatus.path` (absolute path in tool output)
- [x] **D8-2** `src/mcp/handlers.rs:654` — `redact_error(e)` for degraded errors in `handle_info`
- [x] **D8-3** `src/mcp/handlers.rs:635` — removed `config_path` from `handle_info` JSON output
- [x] **D4-1** `src/ingest.rs:225,233` — warning messages now use `path.strip_prefix(wiki_root)` (relative path)
- [x] **D4-2** `src/mcp/handlers.rs:16` — `PATH_RE` known limitation documented; spaces in paths require a parser, not a regex; primary protection is `redact_error()` at all call sites
- [x] **D5-1** `src/index_manager.rs` — `rebuild_lock: Mutex<()>` added to `SpaceIndexManager`; `rebuild()` acquires it at entry
- [x] **D5-2** `src/engine.rs:188` — `rebuild_index()` sets `space.rebuilding = true` before rebuild, clears after

## Security / concurrency (need plan)

- [x] **D5-3** `src/ops/spaces.rs:49` — rollback outside `with_config_lock` (atomic rollback plan needed)
- [x] **D9-1** `src/ops/spaces.rs:136` — in-memory default not rolled back on disk failure (same plan as D5-3)

## Test coverage

- [x] **D2-4** `src/acp/` — unit tests for `acp/helpers.rs` (serialization round-trips), `acp/lint.rs` (rule invocation), `acp/research.rs` (search path)
