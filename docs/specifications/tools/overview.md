---
title: "Tool Surface Overview"
summary: "The MCP/ACP/CLI tools — design principle, grouping, and global flags."
read_when:
  - Getting an overview of all available tools
  - Understanding why a tool belongs in the engine vs a skill
status: ready
last_updated: "2025-07-18"
---

# Tool Surface Overview

The engine exposes 17 tools. Every tool is available via MCP
(stdio + HTTP), ACP, and CLI. Same tool surface, three transports.

## Design Principle

A tool belongs in the engine if and only if it requires **stateful
access** that a skill cannot replicate:

- Filesystem writes into the wiki tree
- Git operations (commit, history)
- Tantivy index queries (search, list, graph traversal)
- Space registry mutations

Everything else — workflow orchestration, LLM prompting, multi-step
procedures — belongs in skills (the `llm-wiki-skills` repository).

## The 16 Tools

### Space management (4 tools)

| Tool | Description |
|------|-------------|
| `wiki_spaces_create` | Create a new wiki repo + register (hot-reloaded if server running) |
| `wiki_spaces_list` | List all registered wikis |
| `wiki_spaces_remove` | Remove a wiki from the registry (unmounted if server running) |
| `wiki_spaces_set_default` | Set the default wiki (updated immediately if server running) |

References:
- [space-management.md](space-management.md)

### Configuration (1 tool)

`wiki_config` — get, set, or list configuration values (per-wiki or
global).

References:
- [config-management.md](config-management.md)

### Schema management (1 tool)

`wiki_schema` — list, show, or add type schemas.

References:
- [schema-management.md](schema-management.md)

### Content operations (4 tools)

| Tool | Description |
|------|-------------|
| `wiki_content_read` | Read full page content by slug or `wiki://` URI |
| `wiki_content_write` | Write a file into the wiki tree |
| `wiki_content_new` | Create a page or section with scaffolded frontmatter |
| `wiki_content_commit` | Commit pending changes to git |

References:
- [content-operations.md](content-operations.md)

### Search & index (7 tools)

| Tool | Description |
|------|-------------|
| `wiki_search` | Full-text BM25 search with optional `--type` filter |
| `wiki_list` | Paginated page listing with type/status filters |
| `wiki_ingest` | Validate frontmatter + update index + commit |
| `wiki_graph` | Generate concept graph (Mermaid/DOT) |
| `wiki_history` | Git commit history for a page |
| `wiki_index_rebuild` | Rebuild tantivy index from committed files |
| `wiki_index_status` | Check index health |

References:
- [search.md](search.md)
- [list.md](list.md)
- [ingest.md](ingest.md)
- [graph.md](graph.md)
- [history.md](history.md)
- [index.md](index.md)

## Global Flags

All CLI commands accept:

```
--wiki <name>    Target a specific wiki (default: global.default_wiki)
```

All MCP/ACP tools accept an optional `wiki` parameter with the same
semantics.

## CLI-Only Commands

These commands are available via CLI only (no MCP/ACP equivalent).

### Log management

| Command | Description |
|---------|-------------|
| `llm-wiki logs tail [--lines N]` | Show recent log entries (default: 50) |
| `llm-wiki logs list` | List log files |
| `llm-wiki logs clear` | Delete all log files |

### Filesystem watcher

| Command | Description |
|---------|-------------|
| `llm-wiki watch [--wiki <name>]` | Auto-ingest on file save (standalone mode) |

Operates on `~/.llm-wiki/logs/`. Only useful when `llm-wiki serve`
has file logging enabled (see [server.md](../engine/server.md)).
