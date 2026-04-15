---
title: "Index"
summary: "Index maintenance commands — rebuild the tantivy index from committed Markdown and inspect index health."
read_when:
  - Implementing or extending index management
  - Understanding when and how to rebuild the tantivy index
  - Diagnosing search issues after a fresh clone or failed ingest
status: draft
last_updated: "2025-07-15"
---

# Index

The tantivy search index lives at `.wiki/search-index/` and is gitignored —
it is a local artifact rebuilt from committed Markdown. `.wiki/index-status.toml`
is committed on every rebuild and serves as the stable record of index state.
`wiki index` provides commands to manage it explicitly.

---

## 1. `index-status.toml`

Written to `.wiki/index-status.toml` and committed on every `wiki index rebuild`:

```toml
built    = "2025-07-15T14:32:01Z"   # ISO datetime of last rebuild
pages    = 142
sections = 8
commit   = "a3f9c12"               # git HEAD at time of rebuild
```

Git commit: `index: rebuild — 142 pages, 8 sections`

Staleness is determined by comparing `commit` against the current `git HEAD`.
If HEAD has moved since the last rebuild, the index is stale. This is reliable
across clones and filesystems — mtime is not used.

---

## 1. Subcommands

### `wiki index rebuild`

Walks all committed Markdown files, indexes all frontmatter fields and body
content, writes the tantivy index to `.wiki/search-index/`, writes
`.wiki/index-status.toml`, and commits both. Required after:

- Fresh clone
- Manual file edits outside of `wiki ingest` or `wiki new`
- Index corruption

```bash
wiki index rebuild
wiki index rebuild --wiki research
```

### `wiki index status`

Reports the current state of the index without modifying it.

```bash
wiki index status
```

Output:

```
wiki:     research
path:     /Users/geronimo/wikis/research/.wiki/search-index/
built:    2025-07-15T14:32:01Z
commit:   a3f9c12
pages:    142
sections: 8
stale:    no
```

`stale: yes` means `commit` in `index-status.toml` does not match the current
`git HEAD` — a rebuild is recommended.

---

## 2. Return Types

```rust
pub struct IndexStatus {
    pub wiki:     String,
    pub path:     String,
    pub built:    Option<String>,   // ISO datetime, None if index does not exist
    pub pages:    usize,
    pub sections: usize,
    pub stale:    bool,
}

pub struct IndexReport {
    pub wiki:          String,
    pub pages_indexed: usize,
    pub duration_ms:   u64,
}
```

---

## 3. CLI Interface

```
wiki index rebuild              # rebuild index from committed Markdown
              [--wiki <name>]
              [--dry-run]       # walk and count pages, no write

wiki index status               # inspect index health
              [--wiki <name>]
```

---

## 4. Staleness Detection

Staleness is determined by comparing the `commit` field in
`.wiki/index-status.toml` against the current `git HEAD`:

- `commit == HEAD` → index is fresh
- `commit != HEAD` → index is stale, rebuild recommended
- `index-status.toml` missing → index has never been built

This is reliable across clones and filesystems. mtime is not used.

---

## 5. Automatic Rebuild

`wiki search`, `wiki list`, and `wiki contradict` check index staleness at
startup via `index-status.toml`. Behavior depends on the `index.auto_rebuild`
config flag:

- `auto_rebuild = false` (default) — print a warning, continue with stale index
- `auto_rebuild = true` — rebuild silently before executing the command

Users who want seamless behavior after a pull opt in via config. The default
is explicit — auto-rebuild adds latency to the first command after any commit.

---

## 6. MCP Tools

```rust
#[tool(description = "Rebuild the tantivy search index from committed Markdown")]
async fn wiki_index_rebuild(
    &self,
    #[tool(param)] wiki: Option<String>,
) -> IndexReport { ... }

#[tool(description = "Inspect the current state of the search index")]
async fn wiki_index_status(
    &self,
    #[tool(param)] wiki: Option<String>,
) -> IndexStatus { ... }
```

---

## 7. Rust Module Changes

| Module | Change |
|--------|--------|
| `search.rs` | Extract `rebuild_index(wiki_root)` as a standalone function; add `index_status(wiki_root)` reading `index-status.toml` |
| `git.rs` | Add `current_head(wiki_root) -> String` for staleness check |
| `cli.rs` | Replace `--rebuild-index` flag on `search` with `index` subcommand (`rebuild`, `status`) |
| `server.rs` | Add `wiki_index_rebuild` and `wiki_index_status` MCP tools |

---

## 8. Implementation Status

| Feature | Status |
|---------|--------|
| Index rebuild (via `wiki search --rebuild-index`) | implemented (to be moved) |
| `wiki index rebuild` | **not implemented** |
| `wiki index status` | **not implemented** |
| Staleness detection | **not implemented** |
| `wiki_index_rebuild` MCP tool | **not implemented** |
| `wiki_index_status` MCP tool | **not implemented** |
