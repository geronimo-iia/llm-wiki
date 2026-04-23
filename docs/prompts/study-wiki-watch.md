# Study: wiki_watch — auto-ingest on file save

Add a filesystem watcher that automatically ingests pages when files
are saved in the wiki tree. Closes the gap between external editing
(IDE, text editor) and the search index.

## Problem

After editing a wiki page in an IDE, the user must manually run
`wiki_ingest` to update the index. Forgetting this means search and
list return stale results. The LLM doesn't know the page changed.

## Decisions

- **`notify` crate (v7)** — cross-platform filesystem events, widely
  used in the Rust ecosystem.
- **CLI flag only** — `--watch` on `serve` and standalone
  `llm-wiki watch`. No `watch.enabled` config key — watching is a
  per-invocation decision, not a persistent setting.
- **Debounce configurable** — `watch.debounce_ms = 500` in config.
  Editors save differently (vim vs VS Code), power users may tune.
- **Schema watching** — watch `schemas/*.json` too. Any `.json`
  change in `schemas/` triggers a full index rebuild (not incremental
  ingest). Rare but critical — without it, schema changes silently
  break the index.
- **`ingest.auto_commit` respected** — watcher follows the existing
  config for whether to commit on ingest.
- **MCP notifications** — watcher emits `resources/updated` for
  changed pages after ingest.

## Proposed behavior

### As a server mode

```
llm-wiki serve --watch
```

Starts the watcher task alongside transport tasks. Shares the same
`WikiEngine` — no separate process.

### As a standalone command

```
llm-wiki watch [--wiki <name>]
```

Runs the watcher without MCP transports. Useful for background
indexing while editing in an IDE. Ctrl+C to stop.

### What it watches

| Path | File type | Action |
|------|-----------|--------|
| `<wiki>/wiki/**/*.md` | Markdown | Incremental ingest |
| `<wiki>/schemas/*.json` | JSON Schema | Full index rebuild |

### What it ignores

- `inbox/`, `raw/` — not wiki content
- Non-`.md` files in `wiki/` — assets don't need ingesting
- `.git/` — internal git operations
- Non-`.json` files in `schemas/` — not schema files

### Debouncing

Editors save files in multiple steps (write temp, rename, etc.).
Debounce by path — collect events for `watch.debounce_ms` (default
500ms), then process the unique set of changed paths in one batch.

### Logging

```
watch: ingested concepts/moe.md
watch: ingested concepts/scaling-laws.md (2 files, 12ms)
watch: schema changed, rebuilding index (42 pages, 180ms)
```

## Interaction with existing features

- **Hot reload** — when a wiki is mounted/unmounted, the watcher
  starts/stops watching that wiki's directory.
- **`ingest.auto_commit`** — watcher respects this setting.
- **MCP notifications** — watcher emits `resources/updated` for
  changed pages after successful ingest.
- **Index staleness** — watcher keeps the index fresh, so
  `index.auto_rebuild` is less relevant when watching is active.

## Open questions

- Should the standalone `llm-wiki watch` also watch multiple wikis
  (all registered), or only the specified/default wiki?

## Tasks

### 1. Update specifications

- [ ] Create `docs/specifications/engine/watch.md` — behavior,
  debouncing, schema watching, serve integration, standalone mode
- [ ] Update `docs/specifications/engine/server.md` — add `--watch`
  flag to serve, watcher task in startup sequence
- [ ] Update `docs/specifications/model/global-config.md` — add
  `watch.debounce_ms` (default: 500)
- [ ] Update `docs/specifications/tools/overview.md` — mention
  `llm-wiki watch` in CLI-only commands

### 2. Config

- [ ] `src/config.rs` — add `WatchConfig { debounce_ms: u32 }`
  with default 500
- [ ] Add to `GlobalConfig` (global-only, not per-wiki)
- [ ] Wire get/set for `watch.debounce_ms`

### 3. Add dependency

- [ ] `Cargo.toml` — add `notify = "7"`

### 4. Watcher core

- [ ] `src/watch.rs` — `WikiWatcher` struct
- [ ] Watch `wiki/**/*.md` for create/modify/rename events
- [ ] Watch `schemas/*.json` for create/modify/delete events
- [ ] Debounce by path using `tokio::time::sleep` + `HashSet`
- [ ] On `.md` change: call `engine.refresh_index()` for changed
  files (incremental ingest)
- [ ] On `.json` schema change: call `engine.rebuild_index()` (full
  rebuild)
- [ ] Respect `ingest.auto_commit` config
- [ ] Log each ingest/rebuild with file count and duration

### 5. Serve integration

- [ ] `src/server.rs` — add `--watch` flag
- [ ] `src/cli.rs` — add `watch` field to `Serve` command
- [ ] Start watcher task in serve startup when `--watch` is set
- [ ] Watcher task shares `Arc<WikiEngine>`, uses cancellation token
  for shutdown
- [ ] On hot reload (mount/unmount), start/stop watching the
  affected wiki directory

### 6. Standalone command

- [ ] `src/cli.rs` — add `Watch` command with `--wiki`
- [ ] `src/main.rs` — run watcher in a tokio runtime, block until
  ctrl+c

### 7. MCP notifications

- [ ] After successful ingest, emit `resources/updated` for each
  changed page URI
- [ ] After schema rebuild, emit `resources/list_changed`

### 8. Tests

- [ ] Watcher detects `.md` file creation and triggers ingest
- [ ] Watcher detects `.md` file modification and triggers ingest
- [ ] Watcher ignores non-`.md` files in `wiki/`
- [ ] Watcher ignores files outside `wiki/` and `schemas/`
- [ ] Schema change triggers full rebuild
- [ ] Debounce batches rapid saves into one ingest
- [ ] Existing test suite passes unchanged

### 9. Decision record

- [ ] `docs/decisions/wiki-watch.md`

### 10. Update skills

- [ ] `llm-wiki-skills/skills/setup/SKILL.md` — mention `--watch`
  flag for live indexing
- [ ] `llm-wiki-skills/skills/content/SKILL.md` — note that with
  `--watch`, manual ingest is not needed after external edits

### 11. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets -- -D warnings`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md`
- [ ] Remove this prompt

## Success criteria

- Editing a `.md` file in the wiki tree while `llm-wiki serve --watch`
  is running updates the search index within ~500ms
- Schema changes trigger a full rebuild automatically
- `llm-wiki watch` works standalone without MCP transports
- MCP notifications are emitted for changed pages
- No performance impact when `--watch` is not used
- Existing tests pass unchanged
