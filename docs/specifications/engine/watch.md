---
title: "Watch"
summary: "Filesystem watcher — auto-ingest on file save, schema rebuild on schema change."
read_when:
  - Understanding how wiki_watch works
  - Setting up live indexing
status: ready
last_updated: "2026-08-28"
---

# Watch

`wiki_watch` monitors the wiki tree for file changes and automatically
updates the search index. Available as a `llm-wiki serve` flag or a
standalone command.

## Modes

### Server mode

```
llm-wiki serve --watch
```

Starts the watcher task alongside transport tasks. Shares the same
`WikiEngine`.

### Standalone mode

```
llm-wiki watch [--wiki <name>]
```

Runs the watcher without MCP transports. Ctrl+C to stop.

## What it watches

| Path | File type | Action |
|------|-----------|--------|
| `<wiki>/wiki/**/*.md` | Markdown | Incremental ingest |
| `<wiki>/schemas/*.json` | JSON Schema | Smart rebuild (partial or full) |

## What it ignores

- `inbox/`, `raw/` — not wiki content
- Non-`.md` files in `wiki/` — assets don't need ingesting
- `.git/` — internal git operations
- Non-`.json` files in `schemas/` — body templates (`.md`) and other
  files do not affect the index

## Debouncing

Editors save files in multiple steps (write temp, rename, etc.).
The watcher collects events for `watch.debounce_ms` (default 500ms),
then processes the unique set of changed paths in one batch.

## Concurrency

All index operations are serialized through a single async channel.
The watcher sends events, a single consumer processes them:

- If a schema rebuild is pending, skip queued `.md` ingests (the
  rebuild covers them)
- If `.md` ingests are pending and a schema change arrives, discard
  the pending ingests and do a full rebuild instead
- Only one index write operation runs at a time

Priority: rebuild > incremental ingest.

### Rebuild dispatch guard (`AtomicBool`)

Before dispatching a `RebuildIndex` event, the watcher checks
`SpaceContext.rebuilding: Arc<AtomicBool>` using
`compare_exchange(false, true, AcqRel, Acquire)`. Only one concurrent
caller wins; all others skip. The flag is cloned out from under a brief
read lock so the lock is not held during the `spawn_blocking` call or
the `.await`.

The flag is reset to `false` in all three outcome branches of the
rebuild task: `Ok(Ok(_))`, `Ok(Err(_))`, and `Err(_)` (panic). A
re-mount via `mount_wiki` constructs a new `SpaceContext` with
`rebuilding = false`, resetting any stuck flag automatically.

This guard is best-effort deduplication at task submission time. It does
not replace serialisation at execution time.

### Rebuild serialisation (`Mutex`)

`SpaceIndexManager` has a `rebuild_lock: Mutex<()>` that serialises
concurrent rebuild calls at execution time. It covers cases the
`AtomicBool` guard cannot:

- A direct `wiki_index_rebuild` MCP call concurrent with a
  watcher-triggered rebuild bypasses the watcher's flag check.
- A race between the watcher's `compare_exchange` and the flag being
  set — the window is tiny but non-zero.

The second rebuild caller blocks until the first finishes, then
proceeds with the already-updated index. See
`docs/implementation/lock-patterns.md` § Serialising Concurrent Rebuilds
for the full lock-ordering discussion.

## Git commits

The watcher updates the tantivy index only — it does not commit to
git. External edits are already on disk; the user manages git through
their own workflow (IDE, CLI). The `ingest.auto_commit` setting
applies to `wiki_ingest`, not to the watcher.

## MCP notifications

After successful ingest, the watcher emits
`notifications/resources/updated` for each changed page URI. After
a schema rebuild, it emits `notifications/resources/list_changed`.

## Hot reload interaction

When a wiki is mounted/unmounted via hot reload, the watcher
starts/stops watching that wiki's directory.

## Configuration

| Key | Default | Description |
|-----|---------|-------------|
| `watch.debounce_ms` | `500` | Debounce interval in milliseconds |

Global-only setting.
