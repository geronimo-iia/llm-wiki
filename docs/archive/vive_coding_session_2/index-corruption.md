# Index Corruption Detection & Rebuild

Gap analysis and improvement plan for tantivy index integrity.

## Current State

### What exists

**Staleness detection** (git-commit based, not corruption):
- `index_status` in `src/search.rs` compares the `commit` field in `state.toml`
  against the current `git HEAD`. If they differ → `stale: true`. If
  `state.toml` is missing → `stale: true`, pages/sections = 0.
- This is the *only* health check. It detects drift, not corruption.

**Auto-rebuild on staleness** — triggered in 3 places, all gated by
`index.auto_rebuild` config (default `false`):

| Location | Trigger |
|----------|---------|
| Server startup (`src/server.rs`) | Loops all registered wikis, rebuilds stale ones |
| CLI `wiki search` (`src/main.rs`) | Checks before executing query |
| CLI `wiki list` (`src/main.rs`) | Same pattern |

**Manual rebuild** — `wiki index rebuild` does a full delete-all + re-index
via `rebuild_index` in `src/search.rs`:
- Calls `writer.delete_all_documents()` then re-walks all Markdown
- Writes fresh `state.toml` with current HEAD

**MCP tools** — `wiki_index_rebuild` and `wiki_index_status` expose the same
functions over MCP.

### What's missing

There is **no actual corruption detection**.

1. **No tantivy integrity check** — if the mmap files under `search-index/`
   are truncated, corrupted, or schema-mismatched, `Index::open()` in `search`
   and `list` will fail with an opaque tantivy error. There is no
   try-open → detect corruption → auto-rebuild path.

2. **No schema migration** — if `build_schema()` changes between versions
   (e.g. adding a field), the existing index becomes incompatible. There is no
   version marker in `state.toml` and no detection of schema mismatch.

3. **No graceful recovery** — when `search` or `list` fail because the index
   is broken, the error propagates as a generic `anyhow` error. The user gets
   a raw message, not a "index appears corrupt, rebuilding…" recovery.

4. **No `state.toml` validation** — if `state.toml` is malformed or partially
   written (crash during rebuild), `toml::from_str` fails and `index_status`
   returns an error rather than treating it as "needs rebuild".

5. **Server has no runtime recovery** — if the index breaks while the server
   is running (e.g. disk issue), search/list calls fail permanently until
   manual restart + rebuild.

---

## Improvements

Ordered by impact.

### 1. Try-open with auto-recovery

Wrap `Index::open()` in `search()` and `list()` with a fallback that rebuilds
on failure:

```rust
let index = match Index::open(dir) {
    Ok(idx) => idx,
    Err(e) => {
        eprintln!("warning: index corrupt ({e}), rebuilding...");
        rebuild_index(wiki_root, index_path, wiki_name, repo_root)?;
        let dir = MmapDirectory::open(&search_dir)?;
        Index::open(dir)?
    }
};
```

- [ ] Implement in `search()`
- [ ] Implement in `list()`
- [ ] Implement in MCP tool handlers (one retry before failing the call)

### 2. Schema version in `state.toml`

Add a `schema_version: u32` field to `IndexState`. Bump it when
`build_schema()` changes. Treat version mismatch as stale in `index_status`.

```toml
schema_version = 1
built = "2025-07-15T14:32:01Z"
pages = 142
sections = 8
commit = "a3f9c12"
```

- [ ] Add `schema_version` to `IndexState`
- [ ] Define `CURRENT_SCHEMA_VERSION` constant
- [ ] Write version on rebuild
- [ ] Check version in `index_status`, treat mismatch as stale

### 3. Resilient `state.toml` parsing

In `index_status`, catch deserialization errors and return `stale: true`
instead of propagating the error:

```rust
let state: IndexState = match toml::from_str(&content) {
    Ok(s) => s,
    Err(_) => return Ok(IndexStatus {
        wiki: wiki_name.to_string(),
        path: index_path.join("search-index").to_string_lossy().into(),
        built: None,
        pages: 0,
        sections: 0,
        stale: true,
    }),
};
```

- [ ] Catch `toml::from_str` errors in `index_status`

### 4. `wiki index check` subcommand

A dedicated integrity check that opens the index, runs a test query, and
reports health without modifying anything. Useful for monitoring and
diagnostics.

```
wiki index check [--wiki <name>]
```

Output:

```
wiki:       research
openable:   yes
queryable:  yes
schema:     v1 (current)
state.toml: valid
stale:      no
```

- [ ] Add `IndexAction::Check` variant to CLI
- [ ] Implement `index_check()` in `src/search.rs`
- [ ] Expose as MCP tool `wiki_index_check`

### 5. Server-side retry

In MCP tool handlers for search and list, catch index errors and attempt one
rebuild before failing the tool call.

- [ ] Wrap `handle_search` with retry-on-index-error
- [ ] Wrap `handle_list` with retry-on-index-error
