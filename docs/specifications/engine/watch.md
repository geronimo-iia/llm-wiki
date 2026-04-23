---
title: "Watch"
summary: "Filesystem watcher — auto-ingest on file save, schema rebuild on schema change."
read_when:
  - Understanding how wiki_watch works
  - Setting up live indexing
status: ready
last_updated: "2025-07-22"
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
| `<wiki>/schemas/*.json` | JSON Schema | Full index rebuild |

## What it ignores

- `inbox/`, `raw/` — not wiki content
- Non-`.md` files in `wiki/` — assets don't need ingesting
- `.git/` — internal git operations
- Non-`.json` files in `schemas/`

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
