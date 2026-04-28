# Local Path in Content Tools

## Decision

Add a `wiki_resolve` tool and expose filesystem paths in `wiki_content_new`
and `wiki_lint` responses. Drop the proposed `wiki_ingest` pages array.

For `wiki_content_new`: change `ops::content_new` to return a
`ContentNewResult` struct (`uri`, `slug`, `path`, `wiki_root`, `bundle`)
instead of a bare URI string.

For `wiki_lint`: add `pub path: String` to `LintFinding`; thread
`wiki_root: &Path` through all 5 rule function signatures; resolve slug → path
at each of the 6 construction sites.

## Context

Every `wiki_content_read` / `wiki_content_write` round-trip sends full page
content through MCP. Claude Code has direct filesystem access (`Write`, `Edit`,
`Read`). If the engine exposes the resolved local path, the LLM can write
content directly to disk and call `wiki_ingest` once to validate — removing
the content round-trip entirely.

Three changes were evaluated. A fourth (`wiki_ingest` pages array) was proposed
in the initial spec but was dropped after reviewing actual skill workflows.

## Rationale

### `wiki_resolve` — implement as specced

`WikiUri::resolve` and `resolve_read_target` are already public and used
identically in every content handler. The new tool is a thin wrapper with no
novel logic. For a not-yet-existing slug, the would-be flat path is returned
(`<wiki_root>/<slug>.md`) — consistent with `wiki_content_new` default behaviour.

### `wiki_content_new` — return struct from ops layer (Option A)

Three options were considered:

- **Option A**: change `ops::content_new` to return `ContentNewResult`
- **Option B**: keep ops returning `String`, compute path in the MCP handler
- **Option C**: return `(String, bool)` tuple from ops

Option A was chosen. All path data (`wiki_root`, `slug`, `bundle`) is in scope
inside `content_new` — the ops layer is the natural owner of this computation.
Option B would call `WikiUri::resolve` twice (inside `content_new` and again in
the handler). Option C uses an unreadable tuple. API breakage is acceptable;
the sole caller is the MCP handler.

### `wiki_lint` — thread `wiki_root` into rule functions (Option A)

Three options were considered:

- **Option A**: pass `wiki_root: &Path` into each rule function, compute path at construction
- **Option B**: annotate findings post-collection in `run_lint`
- **Option C**: compute path in the MCP handler

Option A was chosen. `LintFinding` should be self-contained — any consumer
(CLI, MCP, future transports) gets the path without extra context. The 2 stat
calls per finding (`Slug::resolve` probes flat then bundle) are acceptable for
a lint run. API breakage is acceptable.

### `wiki_ingest` pages array — dropped

The initial spec proposed adding `path` per entry in an ingest pages array.
This was dropped after reviewing actual skill workflows: in every skill
(`ingest`, `content`, `crystallize`), the LLM calls `wiki_ingest` *after*
writing files whose paths it already holds. Returning the paths in the response
is redundant — no follow-up action needs them. Furthermore, `IngestReport` has
no `pages` array today; adding one would require a non-trivial struct change for
zero practical gain.

### Tools not changed

`wiki_search`, `wiki_list`, `wiki_suggest`, `wiki_graph`, `wiki_stats`,
`wiki_history`: bulk results or no direct edit intent follows. Adding `path`
to every result row costs more tokens than it saves.

## Consequences

- `ops::content_new` return type changes from `Result<String>` to
  `Result<ContentNewResult>`; the MCP handler serialises the struct as JSON.
- `LintFinding` gains a `path: String` field; all 5 rule function signatures
  gain a `wiki_root: &Path` parameter; 6 construction sites updated.
- `wiki_resolve` raises the tool count from 21 to 22.
- `wiki_ingest` response and `IngestReport` are unchanged.
- Skills (`content`, `ingest`) adopt the direct write pattern using
  `wiki_resolve` + native file tools.
