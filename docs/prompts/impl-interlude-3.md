# Implement SpaceIndexManager

## Context

Interlude §3. Introduce `SpaceIndexManager` — convert the free
functions in `src/indexing.rs` into methods on a struct.

The full design is in `docs/implementation/index-manager.md`.
The migration strategy is in `docs/interlude-roadmap.md` section 3.

## Read first

1. `docs/interlude-roadmap.md` — section 3 (design, migration steps)
2. `docs/implementation/index-manager.md` — complete module design
3. `src/indexing.rs` — current free functions
4. `src/engine.rs` — `SpaceState`, startup sequence
5. `src/ops/index.rs` — `index_rebuild`, `index_status`
6. `src/ops/search.rs` — uses `index_path`, `RecoveryContext`
7. `src/search.rs` — `search()`, `list()`, `search_all()`
8. `src/graph.rs` — `build_graph()`
9. `src/lib.rs` — module declarations
10. `tests/indexing.rs` — main test file for index operations

## Steps

### Step 1: Copy and declare

1. Copy `src/indexing.rs` → `src/index_manager.rs`
2. Add `pub mod index_manager` to `src/lib.rs` (keep `pub mod indexing`)
3. `cargo test` — passes (no callers changed)

### Step 2: Introduce struct + methods

In `src/index_manager.rs`:

4. Add `SpaceIndexManager` struct with `wiki_name: String` and
   `index_path: PathBuf`
5. Add `SpaceIndexManager::new(wiki_name, index_path) -> Self`
6. Add `pub fn index_path(&self) -> &Path`
7. Add `pub fn wiki_name(&self) -> &str`
8. Convert `rebuild_index` → `SpaceIndexManager::rebuild()` method
   (move the logic in, adjust to use `self.wiki_name`, `self.index_path`)
9. Convert `update_index` → `SpaceIndexManager::update()` method
10. Convert `index_status` → `SpaceIndexManager::status()` method
11. Convert `last_indexed_commit` → `SpaceIndexManager::last_commit()` method
12. Convert `delete_by_type` → `SpaceIndexManager::delete_by_type()` method
13. Convert `open_index` → `SpaceIndexManager::open_index()` method
14. Mark the original free functions as `#[deprecated]` — each creates
    a temporary `SpaceIndexManager` and delegates to the method
15. `cargo test` — passes (deprecated wrappers keep old API working)
16. `cargo clippy -- -D warnings` — clean (allow deprecated in tests)

### Step 3: Migrate engine.rs

17. Add `index_manager: SpaceIndexManager` to `SpaceState`
18. Remove `index_path: PathBuf` from `SpaceState`
19. Update `EngineManager::build()` — construct `SpaceIndexManager`,
    use `index_manager.status()` and `index_manager.rebuild()`
20. Update `EngineManager::on_ingest()` — use `index_manager.update()`
21. Update `EngineManager::rebuild_index()` — use `index_manager.rebuild()`
22. Add `pub fn index_path(&self) -> &Path` to `SpaceState` that
    delegates to `self.index_manager.index_path()` (temporary, for
    callers not yet migrated)
23. `cargo test` — passes

### Step 4: Migrate ops/

24. `src/ops/index.rs` — use `space.index_manager.status()` and
    `space.index_manager.rebuild()`
25. `src/ops/search.rs` — use `space.index_manager.index_path()` and
    `space.index_manager.open_index()` for recovery context
26. Check `src/mcp/handlers.rs` — if it references `space.index_path`
    directly, update to `space.index_manager.index_path()`
27. Check `src/main.rs` — same
28. `cargo test` — passes

### Step 5: Migrate tests

29. `tests/indexing.rs` — change imports to `use llm_wiki::index_manager::*`,
    update test helpers to use `SpaceIndexManager` directly
30. `tests/search.rs` — update if it imports from `indexing`
31. `tests/graph.rs` — update if it imports from `indexing`
32. `tests/ops.rs` — update if it imports from `indexing`
33. `tests/schema_integration.rs` — update if it imports from `indexing`
34. `cargo test` — passes

### Step 6: Remove old module

35. Delete `src/indexing.rs`
36. Remove `pub mod indexing` from `src/lib.rs`
37. Remove `#[deprecated]` attributes and the wrapper functions from
    `src/index_manager.rs`
38. Remove any `#[allow(deprecated)]` annotations
39. Remove `SpaceState::index_path()` helper if no longer needed
40. `cargo test` — passes
41. `cargo clippy -- -D warnings` — clean

## Rules

- Follow `docs/implementation/rust.md` for style
- Each step must compile and pass tests before moving to the next
- Do NOT change `src/search.rs` or `src/graph.rs` signatures — they
  still accept `index_path: &Path` (that changes in §4)
- The `#[deprecated]` wrappers are temporary scaffolding — they exist
  only to avoid breaking all callers at once
- Data types (`IndexReport`, `UpdateReport`, `IndexStatus`,
  `IndexState`, `RecoveryContext`) keep the same shape — just move
  to the new module
- Private helpers (`build_document`, `yaml_to_text`, `index_value`,
  `yaml_to_strings`) stay private in `index_manager.rs`
- Method signatures should eliminate parameters that the struct owns
  (e.g. `rebuild` does not take `index_path` or `wiki_name`)
