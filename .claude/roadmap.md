# llm-wiki-engine — Roadmap to 1.0.0
Source: `.claude/plans/2026-08-17-review-findings.md` (v0.5.9 review)
Date: 2026-08-17

**Target branch:** `release/1.0.0` (cut from `main` at v0.5.9)
All phase branches are cut from `release/1.0.0`, developed, and merged back into `release/1.0.0`.
When Phase 5 is complete and all gates pass, `release/1.0.0` is tagged `1.0.0` and merged into `main`.

1.0.0 signals three things: no known unsoundness, a stable public API surface, and production-grade
reliability under concurrent load. Every item below is grouped by theme and sequenced by dependency. Phases do not map to release versions — multiple phases may ship in a single release, or a phase may span several.
Items are ordered by impact within each version.

---

## ~~Phase 2 — Concurrency & Correctness~~

~~The async runtime must not be blocked. Panics in hot paths must be eliminated.~~
~~Gate: (1) no blocking I/O on Tokio thread, (2) no hot-path panics, (3) ACP server survives mutex poisoning (P2.3 must not slip).~~

### P2.1 · Concurrency: blocking I/O on Tokio thread `[MAJOR · D5]`
- `src/watch.rs:82` — `engine.schema_rebuild()` called directly in `async fn run_watcher`.
- Blocks all MCP requests and ACP sessions for the full rebuild duration (file I/O + `git log` subprocess + Tantivy fsync).
- **Action:** `tokio::task::spawn_blocking(move || engine.schema_rebuild(wiki_name)).await??`.

### P2.2 · Concurrency: read lock held for full rebuild duration `[MAJOR · D5]`
- `src/engine.rs:~185-225` — `schema_rebuild` holds `state.read()` for the entire rebuild.
- Blocks `mount_wiki`/`unmount_wiki` (which need `state.write()`) for seconds under concurrent MCP load.
- **Action:** Extract space metadata under the read lock, drop the lock, then run the rebuild. The index manager's own `RwLock<IndexInner>` guards Tantivy state independently.

### P2.3 · Concurrency: `std::sync::Mutex` inside async handlers `[MAJOR · D1]`
- `src/acp/server.rs:96,123` — poisoned mutex permanently crashes the ACP server task; `std::sync::Mutex` unsafe to hold across future `.await` points.
- **Action:** Replace with `tokio::sync::Mutex`; `.lock().unwrap()` → `.lock().await`.

### P2.4 · Concurrency: no rebuild guard — redundant concurrent rebuilds `[MINOR · D5]`
- Two rapid watcher events produce two concurrent `schema_rebuild` calls; second blocks then runs redundantly.
- **Action:** `AtomicBool` rebuild-in-progress flag per wiki; skip and `tracing::debug!` if already rebuilding.

### P2.5 · Correctness: `is_wiki_md` hardcodes `/wiki/` path `[MINOR · D8]`
- `src/watch.rs:177` — watcher silently drops all events when `wiki_root` is not `"wiki"`. Affects every user with a non-default `wiki_root`.
- **Action:** Delete `is_wiki_md`. The existing `path.starts_with(wiki_root)` guard already scopes correctly; add only the `.extension() == Some("md")` check.

### P2.6 · Rust: `.unwrap()` on HashMap lookups in Louvain `[MAJOR · D1]`
- `src/graph.rs:251,257,1032,1040,1060` — `community.get(&node).unwrap()` panics if invariant breaks.
- **Action:** Replace with `.expect("node must be in community map")`. Add `debug_assert!(community.len() == adj.len())` at `louvain_phase1` entry.


---

## Phase 3 — Performance

Community detection and graph building must scale to realistic wiki sizes.

### P3.1 · Performance: Louvain O(N³) `sigma_tot` `[MAJOR · D6]`
- `src/graph.rs:262-268` — `sigma_tot` rebuilt by full `community.iter()` for every node in every pass: O(N²) per pass × O(N) passes = O(N³) worst case.
- Hits users at ~5 000 nodes; unusable at 20 000.
- **Action:** Precompute `sigma_tot` once before the node loop; update incrementally on each move (subtract from old community, add to new). Standard Louvain is O(M) per pass.

### P3.2 · Performance: `TopDocs::with_limit(100_000)` silently truncates graph `[MAJOR · D6]`
- `src/graph.rs:328` — wikis with >100 000 pages silently produce an incomplete graph with no warning.
- **Action:** Replace with `DocSetCollector` (no limit), or emit `tracing::warn!` when count exceeds limit.

### P3.3 · Performance: community detection re-run on every graph cache miss `[MINOR · D6]`
- `compute_communities` (Louvain, O(N³)) re-runs on every index rebuild even when topology is unchanged.
- **Action:** Cache `CommunityData` separately with invalidation key (node count + edge count hash).

### P3.4 · Operational: `state.toml` writes `pages: 0` after incremental updates `[MINOR · D6]`
- `src/index_manager.rs:672-673` — `wiki_index_status` always shows 0 pages after any watcher-triggered update.
- **Action:** After `update()`, read actual count via `searcher.num_docs()` and write to `state.toml`.

---

## Phase 4 — Test Coverage & Invariant Hardening

1.0.0 requires that every fix since 0.5.0 has a regression test, and that invariants are enforced
by code rather than convention.

### P4.1 · Test: colon-query regression `[MAJOR · D2]`
- No test covers the `parse_query_lenient` fallback path fixed in 0.5.6. If removed, nothing catches it.
- **Action:** `test_search_colon_query` in `test_search.py` (`query: "Layer 1: Attention"`). Unit test in `search.rs`.

### P4.2 · Test: config rollback regression `[MAJOR · D2]`
- No test verifies `spaces_create`/`spaces_register` rollback after `mount_wiki` failure.
- `SpaceIndexManager::fail_next_reload` exists for exactly this injection but is unused.
- **Action:** Rust unit test using `fail_next_reload`; assert `spaces_list` does not include the wiki after failure.

### P4.3 · Test: Louvain community detection correctness `[MAJOR · D6]`
- 0.8.1 fixes O(N³) Louvain but ships with no regression test. If the incremental `sigma_tot` update is wrong, community assignments silently change.
- **Action:** Unit test in `src/graph.rs` that runs `louvain_phase1` on a known small graph (e.g. two clear clusters of 4 nodes each) and asserts nodes in the same cluster receive the same community ID. Run before and after 0.8.1 to confirm correctness is preserved.

### P4.4 · Test: cross-wiki body link lint integration test `[MINOR · D2]`
- No `test_lint.py` test for `[text](wiki://other/slug)` body link false positive (fixed in 0.5.9).
- **Action:** `test_lint_cross_wiki_body_link_no_false_positive` in `test_lint.py`.

### P4.5 · Test: `test_spaces_set_default` checks text only, not engine state `[MINOR · D2]`
- **Action:** After `wiki_spaces_set_default`, call `wiki_spaces_list` and assert returned default matches.

### P4.6 · Test: smoke tests for untested modules `[MINOR · D2]`
- `watch.rs`, `git.rs`, `search.rs`, `engine.rs`, `config.rs`, `ingest.rs` have zero `#[cfg(test)]` blocks.
- **Action:** At minimum: `search::search()` with in-memory index; `config::load_global` with tempfile; `ingest::ingest()` with minimal fixture.

### P4.7 · Invariant: rollback failure swallows error silently `[MINOR · D9]`
- `ops::spaces_create/register` — if `spaces::remove` itself fails during rollback, the error is dropped with `let _ =`.
- **Action:** `.inspect_err(|e| tracing::error!(error = %e, "rollback failed; wiki may be stranded in config"))`.

### P4.8 · Invariant: graph cache key intent undocumented `[MINOR · D9]`
- `WikiGraphCache::NoSnapshot` uses `generation()` as cache key, not `last_commit()`. Functionally correct but undocumented; next developer will question it.
- **Action:** Comment at `get_fresh` and `generation()` explaining the proxy relationship.

---

## Phase 5 — Stable Public API (1.0.0 gate)

The final gate. A stable API surface is the definition of 1.0.0.
**Ordering constraint:** P5.2 (`PathBuf`) and P5.3 (`NormalizedSlug`) are breaking changes and must land first in this phase — before doc comments (P5.7) and MCP descriptions (P5.6) are written against the final API shape.

### P5.1 · API: `lib.rs` all 20 modules `pub mod` — no stability boundary `[MAJOR · D3/D7]`
- Implementation internals (`acp`, `space_builder`, `mcp`, `cli`, `watch`, `server`, `index_schema`) are part of the public crate API. Any internal refactor is technically a semver violation.
- **Action:** Mark internal modules `pub(crate)`. Curated `pub use` facade for stable surface: `WikiEngine`, `GlobalConfig`, `SearchResult`, `IngestReport`, `WikiGraph`. Add `#![warn(unreachable_pub)]`.

### P5.2 · Design: `WikiEntry::path` and `WikiConfig::wiki_root` are `String` not `PathBuf` `[MINOR · D1]`
- Forces every consumer to call `PathBuf::from(...)`, loses type-system path/string distinction. Breaking change — must land before 1.0.0 freezes the API.
- **Action:** Change to `PathBuf` with `#[serde(serialize_with)]` for TOML. Migration in `save_global`/`load_global`.

### P5.3 · Design: `NormalizedSlug` newtype for compile-time slug invariant `[MINOR · D9]`
- Invariant 6 (slug normalisation) is convention-enforced only; raw `String` slugs can be compared with normalised ones silently.
- **Action:** `NormalizedSlug(String)` newtype constructable only via `Slug::normalize()`. Makes invariant compile-time enforced. Breaking change — must land before 1.0.0.

### P5.4 · Operational: MCP errors leak filesystem paths `[MINOR · D8]`
- `src/mcp/handlers.rs` — `map_err(|e| format!("{e}"))` exposes absolute paths to LLM clients.
- **Action:** Structured `WikiError { code, message }` type, or path-redaction pass before serialising.

### P5.5 · Operational: `IndexStatus` missing `degraded_reason` `[MINOR · D8]`
- `wiki_index_status` returns `"degraded"` with no explanation of cause.
- **Action:** `degraded_reason: Option<String>` on `IndexStatus`; populate at each degraded code path.

### P5.6 · Documentation: `wiki_config` tool `action` parameter not self-describing `[MINOR · D3]`
- LLM clients have no way to discover valid key paths for `get`/`set` without trial and error.
- **Action:** Extend description with valid actions, required params, and an example key path.

### P5.7 · Documentation: public functions in `watch.rs`, `server.rs`, `acp/` lack doc comments `[MINOR · D3]`
- **Action:** `///` doc comments on all `pub fn` in these modules, or downgrade to `pub(crate)`.

---

## Post-1.0 — Opportunistic

Low urgency. Fix alongside related work or when touching the file anyway.

- `src/config.rs:default_log_path` — `HOME` fallback to `"."` silent; add `tracing::warn!` `[NIT · D3]`
- `src/graph.rs` — `.unwrap()` on `edge_endpoints()` undocumented; add `// SAFETY:` comment `[MINOR · D1]`
- `src/index_manager.rs:150` — `.unwrap()` on `RwLock::read()` in `generation()`; use `.unwrap_or(0)` `[MINOR · D1]`
- `src/type_registry.rs:254,498`, `src/space_builder.rs:62,78`, `src/index_schema.rs:181,193` — `file_name().unwrap()` on dir entries; use `.unwrap_or_default()` `[MINOR · D1]`
- `src/ops/content.rs:248` — `.parent().unwrap()` on bundle path; propagate as `Result` `[MINOR · D1]`
- `src/ops/logs.rs:73` — `.unwrap()` on `entries.last()`; use `.ok_or_else(...)` `[MINOR · D1]`
- `src/index_manager.rs` — `tracing::warn!` for `.gitkeep` and hidden files; downgrade to `debug!` `[NIT · D8]`
- `src/watch.rs:137,225,229` — `let _ = tx.try_send(...)` silent drop; add `tracing::warn!` at all 3 sites `[MINOR · D8]`
- `src/mcp/tools.rs:22` — `.as_object().unwrap()` on static JSON literal; cannot panic today, hypothetical future risk only; convert `fn schema()` to `Result` when touching the file `[MAJOR · D1]`

---

## Summary

| Phase | Theme | Items | Gate |
|---------|-------|-------|------|
| ~~Phase 1~~ | ~~Security & Soundness~~ | ~~5~~ | ✅ Done |
| ~~Phase 2~~ | ~~Concurrency & Correctness~~ | ~~6~~ | ✅ Done |
| Phase 3 | Performance | 4 | Louvain scales to 20k nodes; graph not silently truncated |
| Phase 4 | Test Coverage & Invariants | 8 | Every shipped fix has a regression test; Louvain correctness verified |
| Phase 5 | Stable Public API | 7 | `pub(crate)` internals; `PathBuf` paths; `NormalizedSlug` |
| Post-1.0 | Opportunistic | 9 | — |
| **Open total** | | **27** | |
