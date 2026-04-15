---
title: "List"
summary: "Paginated enumeration of wiki pages via the tantivy index, optionally filtered by type. Defines the PageSummary return type."
read_when:
  - Implementing or extending the list command
  - Understanding the PageSummary type
  - Getting a full inventory of wiki pages for an LLM workflow
status: draft
last_updated: "2025-07-15"
---

# List

`wiki list` enumerates pages from the tantivy index with optional type
filtering and offset-based pagination. No search ranking — results are ordered
by slug alphabetically.

---

## 1. Return Type — `PageSummary`

```rust
pub struct PageSummary {
    pub slug:   String,
    pub uri:    String,    // wiki://<wiki-name>/<slug>
    pub path:   String,    // absolute path on disk
    pub title:  String,
    pub r#type: String,    // concept | source | contradiction | query | section | ...
    pub status: String,    // active | draft | stub | archived
    pub tags:   Vec<String>,
}
```

`PageSummary` is lighter than `PageRef` — no score, no excerpt. It is an
inventory entry, not a search result.

---

## 2. Pagination

Offset-based. The index is a static snapshot of committed files — no
concurrent writes during a list call, so offset pagination is stable and
sufficient.

```rust
pub struct PageList {
    pub pages:   Vec<PageSummary>,
    pub total:   usize,   // total pages matching the filter
    pub page:    usize,   // current page (1-based)
    pub page_size: usize,
}
```

---

## 3. CLI Interface

```
wiki list
         [--type <type>]      # filter by frontmatter type field
         [--status <status>]  # filter by frontmatter status field
         [--page <n>]         # page number, 1-based (default: 1)
         [--page-size <n>]    # results per page (default: from config)
         [--wiki <name>]
```

### Examples

```bash
wiki list                              # all pages, page 1
wiki list --type concept               # concept pages only
wiki list --type contradiction --status active
wiki list --page 2 --page-size 20
```

---

## 4. MCP Tool

```rust
#[tool(description = "List wiki pages from the index, with optional type/status filter and pagination")]
async fn wiki_list(
    &self,
    #[tool(param)] r#type: Option<String>,
    #[tool(param)] status: Option<String>,
    #[tool(param)] page: Option<usize>,
    #[tool(param)] page_size: Option<usize>,
    #[tool(param)] wiki: Option<String>,
) -> PageList { ... }
```

---

## 5. Rust Module Changes

| Module | Change |
|--------|--------|
| `search.rs` | Add `list(filter, page, page_size)` querying tantivy with no search term; add `PageSummary`, `PageList` |
| `cli.rs` | Add `--type`, `--status`, `--page`, `--page-size` to `list` subcommand |
| `server.rs` | Update `wiki_list` return type to `PageList` |

---

## 6. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki list` basic enumeration | implemented (partial) |
| `PageSummary` struct | **not implemented** |
| `PageList` pagination struct | **not implemented** |
| `--type` filter | implemented |
| `--status` filter | **not implemented** |
| `--page` / `--page-size` pagination | **not implemented** |
| `wiki_list` returning `PageList` | **not implemented** |
