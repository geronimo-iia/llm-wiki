---
title: "Roadmap"
summary: "Roadmap planning for llm-wiki."
status: ready
last_updated: "2026-05-03"
---

# Roadmap

## v0.4.0 (in progress)

| Area  | What |
|-------|------|
| Graph | petgraph-live Phase 1: replace bespoke `CachedGraph` with `GenerationCache` ✓ implemented |
| Graph | petgraph-live Phase 2: snapshot warm-start via `GraphState` — see [docs/improvements/2026-05-03-petgraph-live-snapshot.md](improvements/2026-05-03-petgraph-live-snapshot.md) |
| Graph | petgraph-live Phase 3: `wiki_health` MCP tool + structural algorithms — see [docs/improvements/2026-05-03-petgraph-live-algorithms.md](improvements/2026-05-03-petgraph-live-algorithms.md) |

## Future

| Area                                            | What                                                     |
| ----------------------------------------------- | -------------------------------------------------------- |
| IDE                                             | Zed agent panel validation; Cursor MCP config validation |
| Remote Wiki Registration and Version Management | see docs/improvements/design-spaces-register-remote.md   |
| REST / OpenAPI API                              | see docs/improvements/2026-05-03-rest-api-design.md      |

