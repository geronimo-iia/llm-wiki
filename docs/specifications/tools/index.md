---
title: "Index"
summary: "Rebuild and inspect the search index."
read_when:
  - Managing the search index
  - Diagnosing search issues
status: ready
last_updated: "2025-07-17"
---

# Index

| Command | MCP tool | Description |
|---------|----------|-------------|
| `index rebuild` | `wiki_index_rebuild` | Rebuild index from committed files |
| `index status` | `wiki_index_status` | Check index health |

## index rebuild

MCP tool: `wiki_index_rebuild`

```
llm-wiki index rebuild
              [--wiki <name>]
              [--format <fmt>]          # text | json (default: from config)
              [--dry-run]
```

Drops all documents and re-indexes the entire wiki tree. Use after a
fresh clone, manual file edits, index corruption, or engine upgrade.

See [index-management.md](../engine/index-management.md).

### Output

Text (default):

```
rebuild: research — 142 pages indexed in 320ms
rebuild: work — 87 pages indexed in 210ms
```

JSON (`--format json`):

```json
[
  {
    "wiki": "research",
    "pages_indexed": 142,
    "duration_ms": 320
  },
  {
    "wiki": "work",
    "pages_indexed": 87,
    "duration_ms": 210
  }
]
```

## index status

MCP tool: `wiki_index_status`

```
llm-wiki index status
              [--wiki <name>]
              [--format <fmt>]          # text | json (default: from config)
```

See [index-management.md](../engine/index-management.md).

### Output

Text (default):

```
research  142 pages  8 sections  built 2025-07-17T14:32:01Z  fresh
work       87 pages  3 sections  built 2025-07-17T12:10:00Z  stale
```

JSON (`--format json`):

```json
[
  {
    "wiki": "research",
    "pages": 142,
    "sections": 8,
    "built": "2025-07-17T14:32:01Z",
    "stale": false,
    "openable": true,
    "queryable": true,
    "schema_current": true
  },
  {
    "wiki": "work",
    "pages": 87,
    "sections": 3,
    "built": "2025-07-17T12:10:00Z",
    "stale": true,
    "openable": true,
    "queryable": true,
    "schema_current": true,
    "degraded_reason": "index is behind the current HEAD commit or schema — rebuild needed"
  }
]
```
