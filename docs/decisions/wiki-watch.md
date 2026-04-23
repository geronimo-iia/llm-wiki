# wiki_watch — filesystem watcher

## Decision

Add a filesystem watcher using the `notify` crate that auto-ingests
pages on file save and smart-rebuilds the index on schema changes.

## Context

After editing a wiki page externally, the user must manually run
`wiki_ingest`. Forgetting this means stale search results.

## Key decisions

- **`notify` crate v7** — cross-platform, widely used.
- **CLI flag only** — `--watch` on serve, standalone `llm-wiki watch`.
  No config key for enabled/disabled.
- **Configurable debounce** — `watch.debounce_ms = 500` (global-only).
- **Schema watching** — `schemas/*.json` changes trigger smart rebuild
  (partial when possible, full when necessary) via `schema_rebuild`.
- **Serialized operations** — all index operations go through a single
  async channel. Rebuild takes priority over incremental ingest.
- **Index only, no git commit** — the watcher updates the tantivy
  index, not git. External edits are already on disk; the user manages
  git through their own workflow. `ingest.auto_commit` applies to
  `wiki_ingest`, not to the watcher.
- **No new MCP tool** — the watcher is a server/CLI feature, not a
  tool. It uses existing ingest and rebuild paths.

## Consequences

- External edits are indexed within ~500ms
- Schema changes are handled automatically
- `notify` adds a dependency (~13 transitive crates)
- No performance impact when `--watch` is not used
