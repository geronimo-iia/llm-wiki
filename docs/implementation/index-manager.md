---
title: "Index Manager Implementation"
summary: "SpaceIndexManager — incremental update, full rebuild, staleness detection, corruption recovery."
status: ready
last_updated: "2025-07-17"
---

# Index Manager Implementation

Implementation reference for the index manager. Not a specification —
see [index-management.md](../specifications/engine/index-management.md)
for the design.

Follows the [manager pattern](manager-pattern.md).

## Core Struct

```rust
struct SpaceIndexManager {
    wiki_name: String,
    wiki_root: PathBuf,
    index_root: PathBuf,       // ~/.llm-wiki/indexes/<name>/
    state: SpaceIndex,
}

impl SpaceIndexManager {
    /// Build from committed files (startup or full rebuild)
    fn build(wiki_name: &str, wiki_root: &Path, index_root: &Path,
             schema: &IndexSchema) -> Result<Self>;

    /// Check if index is stale (commit or schema_hash mismatch)
    fn has_changed(&self, repo_root: &Path) -> Result<bool>;

    /// Incremental update from git diffs
    fn update(&mut self, registry: &SpaceTypeRegistry) -> Result<UpdateReport>;

    /// Full rebuild from committed files
    fn rebuild(&mut self, registry: &SpaceTypeRegistry) -> Result<RebuildReport>;

    /// Partial rebuild — re-index pages of specific types only
    fn rebuild_types(&mut self, types: &[String],
                     registry: &SpaceTypeRegistry) -> Result<RebuildReport>;

    /// Try to open, recover if corrupt
    fn open_or_recover(&mut self, registry: &SpaceTypeRegistry) -> Result<()>;

    /// Get the current index (read-only)
    fn state(&self) -> &SpaceIndex;
}
```

## Reports

```rust
struct UpdateReport {
    pages_updated: usize,
    pages_deleted: usize,
    duration_ms: u64,
}

struct RebuildReport {
    pages_indexed: usize,
    duration_ms: u64,
    reason: RebuildReason,
}

enum RebuildReason {
    Explicit,           // llm-wiki index rebuild
    FirstBuild,         // no state.toml
    SchemaChange,       // schema_hash mismatch
    Corruption,         // Index::open() failed
    IncrementalFailed,  // fallback from update()
}
```

## Operations

### Incremental update

Called by `WikiEngine.refresh_index()`. Uses two git diffs to find
changed pages:

```
A = working tree vs HEAD           (uncommitted changes)
B = state.toml.commit vs HEAD      (commits since last update)

changed = A union B

for each changed path:
    delete_term(slug)
    if file exists: add_document()
writer.commit()
```

### Full rebuild

Called when `schema_hash` mismatches, on corruption recovery, or
explicitly via `llm-wiki index rebuild`:

```
delete_all_documents()
walk wiki/ -> parse each .md -> add_document()
writer.commit()
writer.wait_merging_threads()
update state.toml
```

### Partial rebuild

Called when `SpaceTypeRegistryManager.refresh()` reports some types
changed but not all:

```
for each changed type:
    collect all pages with that type from the index
    delete each
    re-walk wiki/ -> re-parse pages of that type -> add_document()
writer.commit()
update state.toml
```

### Corruption recovery

Called when `Index::open()` fails and `index.auto_recovery` is true:

```
1. Delete index directory
2. Full rebuild
3. Retry open
4. If still fails -> error propagated
```

Recovery is attempted once.

## Staleness Check

```rust
fn has_changed(&self, repo_root: &Path) -> Result<bool> {
    // 1. Read state.toml
    // 2. Compare commit against git HEAD
    // 3. Call compute_disk_hashes(repo_root) to get current schema_hash
    // 4. Compare against stored schema_hash
    // 5. Either mismatch -> true
}
```

## State File

Reads and writes `~/.llm-wiki/indexes/<name>/state.toml`:

```toml
schema_hash = "a1b2c3d4..."
commit      = "a3f9c12..."
pages       = 142
sections    = 8
built       = "2025-07-17T14:32:01Z"

[types]
concept  = "e5f6a7b8..."
paper    = "c9d0e1f2..."
skill    = "3a4b5c6d..."
```

Updated after every successful rebuild or update.

## Called by WikiEngine

```
WikiEngine.refresh_index(wiki)
    -> SpaceIndexManager.update(registry)

WikiEngine.rebuild_index(wiki)
    -> SpaceIndexManager.rebuild(registry)
```

## Initial Scope

- `build`, `has_changed`, `update`, `rebuild`, `open_or_recover`
- `rebuild_types` returns "full rebuild required" until partial rebuild
  is implemented
