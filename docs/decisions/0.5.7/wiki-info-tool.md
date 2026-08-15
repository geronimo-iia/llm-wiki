# wiki_info Tool

## Decision

Add a `wiki_info` MCP tool that takes no arguments and returns a JSON object with five fields:
`version`, `config_path`, `spaces`, `default_wiki`, and `index_status`.

## Context

Clients connecting to the MCP server have no way to discover the running server version,
the config file in use, which wikis are registered, or whether the index is healthy — without
calling a sequence of other tools or inspecting out-of-band files. A single zero-argument tool
covers the common use cases: version discovery, health checks, and session bootstrap.

## Alternatives considered

**Per-space index detail instead of aggregate status.** `wiki_index_status` already exposes
per-space detail (openable, queryable, stale, document count). `wiki_info` is for quick health
checks and version discovery, not diagnosis. A single `"ok"` / `"degraded"` string is sufficient
for the bootstrap case and avoids duplicating `wiki_index_status` output.

**`engine.config.wikis` vs `engine.spaces.keys()` for the spaces list.** `config.wikis` is an
ordered `Vec<WikiEntry>` that reflects registration order; `spaces.keys()` is a `HashMap`,
unordered and non-deterministic. The `spaces` field uses `config.wikis` to produce stable,
deterministic output.

**`engine.spaces.keys()` for the health loop.** Only loaded spaces can be queried; config entries
whose directories do not exist would fail `ops::index_status`. Iterating `engine.spaces.keys()`
restricts the health check to spaces that are actually open.

**Arguments for wiki filtering.** The tool reports server-level state, not per-wiki state.
Callers have no context to provide. Adding a `wiki` argument would imply filtering behavior
that belongs to `wiki_index_status`.

## Consequences

- `handle_info` added to `src/mcp/handlers.rs`; reads `EngineState` in one lock acquisition.
- `"wiki_info"` descriptor and dispatch arm added to `src/mcp/tools.rs`.
- `index_status` aggregated via `ops::index_status` per loaded space — no direct `index_manager` calls.
- No new dependencies.
