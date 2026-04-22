# Study: wiki_watch — auto-ingest on file save

Add a filesystem watcher that automatically ingests pages when files
are saved in the wiki tree. Closes the gap between external editing
(IDE, text editor) and the search index.

## Problem

After editing a wiki page in an IDE, the user must manually run
`wiki_ingest` to update the index. Forgetting this means search and
list return stale results. The LLM doesn't know the page changed.

## Proposed behavior

### As a server mode

`wiki_watch` runs as part of `llm-wiki serve` (opt-in):

```
llm-wiki serve --watch
```

Or as a standalone command:

```
llm-wiki watch [--wiki <name>]
```

### What it does

1. Watch `<wiki>/wiki/` for `.md` file changes (create, modify, rename)
2. Debounce — wait 500ms after last change before ingesting
3. Run incremental `wiki_ingest` on changed files
4. Log: `watch: ingested concepts/moe.md`
5. Emit MCP `notifications/resources/updated` for changed pages

### What it ignores

- `inbox/`, `raw/`, `schemas/` — not wiki content
- Non-`.md` files in `wiki/` — assets don't need ingesting
- `.git/` changes — internal git operations

## Implementation considerations

### Watcher library

`notify` crate (v7) — cross-platform filesystem events. Already
widely used in the Rust ecosystem.

### Debouncing

Editors save files in multiple steps (write temp, rename, etc.).
Debounce by path — collect events for 500ms, then ingest the unique
set of changed paths.

### Integration with serve

If `--watch` is passed to `llm-wiki serve`:
- Start a watcher task alongside the transport tasks
- Watcher calls `engine.refresh_index()` on changes
- Shares the same `WikiEngine` — no separate process

### Standalone mode

`llm-wiki watch` runs the watcher without MCP transports. Useful for
background indexing while editing in an IDE.

## Interaction with existing features

- Hot reload: when a wiki is mounted/unmounted, the watcher should
  start/stop watching that wiki's directory
- `ingest.auto_commit`: watcher should respect this setting (commit
  on ingest or not)
- MCP notifications: watcher triggers `resources/updated` for
  changed pages

## Open questions

- Should the watcher also detect schema changes (`schemas/*.json`)
  and trigger a full rebuild?
- Should there be a config key `watch.enabled` or is the CLI flag
  sufficient?
- Debounce interval — configurable or hardcoded 500ms?

## Tasks

- [ ] Spec: `docs/specifications/engine/watch.md`
- [ ] Add `notify` dependency
- [ ] `src/watch.rs` — watcher with debouncing
- [ ] `src/server.rs` — integrate watcher task with `--watch` flag
- [ ] `src/cli.rs` — standalone `Watch` command
- [ ] Config: `watch.debounce_ms` (default: 500)
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
