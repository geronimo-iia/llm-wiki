# Decision: WikiPage Struct — Not Needed

## Context

Design review flagged that `ParsedPage` doesn't carry a slug, and
multiple functions pass `(slug, page)` pairs.

→ [analysis prompt](../prompts/wiki-page-struct.md)

## Decision

**Leave as-is.** Do not introduce a `WikiPage` struct.

## Rationale

| Factor | Assessment |
|--------|-----------|
| Call sites | 3, all in `index_manager.rs` (rebuild, update, rebuild_types) |
| External consumers | None — no other module passes (slug, page) pairs |
| Construction timing | Slug comes from path, URI from slug + wiki name, page from file content — computed at different points in the loop |
| What it saves | One line per call site (fewer `build_document` args) |
| What it costs | A new public struct that only `index_manager.rs` uses |

The real duplication is the loop body (read → parse → slug → uri →
build_document), not the lack of a combined type. If that loop body
is extracted into a helper (see `build-document-refactor.md`), the
(slug, page) pair becomes local to the helper and never escapes.

## What stays

- `ParsedPage` remains the parse-only type (no slug, no URI)
- `build_document` keeps its 5-param signature
- If a future module needs slug + page together, revisit then
