---
title: "Engine State"
summary: "Where the engine stores its state — global config, space registry, and search indexes at ~/.llm-wiki/."
read_when:
  - Understanding where engine state lives on disk
  - Understanding the separation between wiki repo and engine state
  - Diagnosing index or config issues
status: ready
last_updated: "2025-07-17"
---

# Engine State

Engine state lives outside the wiki repository, at `~/.llm-wiki/`. It
is local to the machine — never committed, never shared.

```
~/.llm-wiki/
├── config.toml             ← global config + space registry
├── indexes/
│   └── <name>/             ← per-wiki index
│       ├── search-index/   ← tantivy files
│       └── state.toml      ← indexed commit, page count, built date
└── logs/                   ← rotating log files for llm-wiki serve
```


## Global Config

`~/.llm-wiki/config.toml` holds the space registry (which wikis are
registered and where they live) and global defaults. Created
automatically on the first `llm-wiki spaces create`.

See [global-config.md](../model/global-config.md) for the full
key reference.


## Search Indexes

One index per wiki at `~/.llm-wiki/indexes/<name>/`. The search index
is a derived artifact — rebuildable from committed files at any time
via `llm-wiki index rebuild`.

`state.toml` tracks the indexed commit, page count, and build date.
The engine uses it for staleness detection.

See [index-management.md](index-management.md) for staleness, schema
versioning, and auto-recovery.


## Logs

`~/.llm-wiki/logs/` holds rotating log files written by
`llm-wiki serve`. Created automatically on first use.

See [server.md](server.md) for logging configuration.
