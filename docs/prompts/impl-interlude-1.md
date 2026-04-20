# Implement Schema File Content Hash

## Context

Phase 2 is complete. Before starting Phase 3 (Typed Graph), we need
to fix the schema hash computation so it detects file content changes.

The full design is in `docs/interlude-roadmap.md` section "1. Schema file content in hash".

## Read first

1. `docs/interlude-roadmap.md` — section 1 (design, impact, tests)
2. `src/type_registry.rs` — `compute_hashes`, `RegisteredType`,
   `compile_schema`, `discover_from_dir`, `discover_from_embedded`
3. `src/space_builder.rs` — `assemble`, `assemble_without_overrides`,
   `parse_schema_file`
4. `src/indexing.rs` — `index_status`, `rebuild_index` (writes state.toml)
5. `src/engine.rs` — startup staleness check
6. `src/ops/index.rs` — `index_status` wrapper
7. `tests/indexing.rs` — `status_stale_on_schema_hash_mismatch`

## Steps

1. Add `sha2 = "0.10"` to `Cargo.toml`
2. Add `content_hash: String` to `RegisteredType`
3. Update `compile_schema` to compute SHA-256 of the content string
   and store in `content_hash`
4. Update `discover_from_dir` and `discover_from_embedded` to pass
   content hash through
5. Update `space_builder.rs` to compute content hash when creating
   `RegisteredType` directly
6. Replace `DefaultHasher` with SHA-256 in `compute_hashes` — hash
   `schema_path` + `aliases` + `content_hash` per type
7. Add `compute_disk_hashes(repo_root) -> Result<(String, HashMap)>`
   standalone function
8. Update `indexing::index_status` — remove `current_schema_hash`
   parameter, call `compute_disk_hashes` internally
9. Update `engine.rs` startup — use `compute_disk_hashes` instead of
   `type_registry.schema_hash()`
10. Update `ops/index.rs` — remove `schema_hash` parameter from call
11. Update all tests that pass `schema_hash` to `index_status`
12. Rewrite `status_stale_on_schema_hash_mismatch` test to modify a
    file on disk
13. Add new tests per the test plan in the interlude roadmap
14. `cargo test` — all pass
15. `cargo clippy -- -D warnings` — clean

## Rules

- Follow `docs/implementation/rust.md` for style
- SHA-256 output as lowercase hex string (64 chars)
- `compute_disk_hashes` must handle: schemas/ dir present, schemas/
  dir missing (embedded fallback), wiki.toml overrides
- The registry's `schema_hash()` and `type_hashes()` methods stay —
  they return the build-time hashes
- `compute_disk_hashes` is independent of the registry — it reads
  files directly
- After this change, old `state.toml` files trigger a full rebuild
  (expected — hash format changed)
