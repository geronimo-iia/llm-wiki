---
title: "config.toml"
summary: "Global engine config at ~/.llm-wiki/config.toml — space registry, defaults, and global-only settings."
read_when:
  - Understanding what config.toml contains
  - Looking up a config key and its default
  - Understanding which keys are global-only vs per-wiki
status: ready
last_updated: "2025-07-17"
---

# config.toml

`~/.llm-wiki/config.toml` is the global engine configuration. It holds
the space registry and all global defaults. Local to the machine, never
committed.

## Example

```toml
# ── Spaces ─────────────────────────────────────────────────────────────────────

[global]
default_wiki = "research"

[[wikis]]
name        = "research"
path        = "/Users/geronimo/wikis/research"
description = "ML research knowledge base"

[[wikis]]
name = "work"
path = "/Users/geronimo/wikis/work"

# ── Defaults (overridable per wiki in wiki.toml) ──────────────────────────────

[defaults]
search_top_k    = 10
search_excerpt  = true
search_sections = false
page_mode       = "flat"
list_page_size  = 20

[read]
no_frontmatter = false

[ingest]
auto_commit = true

[validation]
type_strictness = "loose"

[graph]
format = "mermaid"
depth  = 3

# ── Global-only settings ──────────────────────────────────────────────────────

[index]
auto_rebuild  = false
auto_recovery = true

[serve]
sse             = false
sse_port        = 8080
acp             = false
max_restarts    = 10
restart_backoff = 1
heartbeat_secs  = 60

[logging]
log_path      = "~/.llm-wiki/logs"
log_rotation  = "daily"
log_max_files = 7
log_format    = "text"
```


## Sections

### `[global]` — Identity

| Key            | Default | Description                                |
| -------------- | ------- | ------------------------------------------ |
| `default_wiki` | —       | Wiki name used when no `--wiki` flag given |

### `[[wikis]]` — Space Registry

Each entry registers a wiki. Created by `llm-wiki spaces create`.

| Field         | Required | Description                          |
| ------------- | -------- | ------------------------------------ |
| `name`        | yes      | Wiki name                            |
| `path`        | yes      | Absolute path to the wiki repository |
| `description` | no       | One-line description                 |

### Overridable defaults

These keys can appear in both `config.toml` (global) and `wiki.toml`
(per-wiki). Per-wiki wins.

| Key                          | Default   | Description                                       |
| ---------------------------- | --------- | ------------------------------------------------- |
| `defaults.search_top_k`      | `10`      | Default result count for `wiki_search`            |
| `defaults.search_excerpt`    | `true`    | Include excerpts; `false` = `--no-excerpt`        |
| `defaults.search_sections`   | `false`   | Include section pages                             |
| `defaults.page_mode`         | `flat`    | Default page creation mode: `flat` or `bundle`    |
| `defaults.list_page_size`    | `20`      | Default page size for `wiki_list`                 |
| `read.no_frontmatter`        | `false`   | Strip frontmatter from `wiki_read` output         |
| `ingest.auto_commit`         | `true`    | Commit after ingest                               |
| `validation.type_strictness` | `loose`   | `strict`: unknown type is error; `loose`: warning |
| `graph.format`               | `mermaid` | Default output format: `mermaid` or `dot`         |
| `graph.depth`                | `3`       | Default hop limit when `--root` is set            |
| `graph.type`                 | `[]`      | Page types to include; empty = all                |
| `graph.output`               | `""`      | Default output path; empty = stdout               |

### Global-only settings

These keys can only appear in `config.toml`. Setting them in `wiki.toml`
is rejected.

| Key                     | Default            | Description                               |
| ----------------------- | ------------------ | ----------------------------------------- |
| `index.auto_rebuild`    | `false`            | Rebuild stale index before search/list    |
| `index.auto_recovery`   | `true`             | Rebuild corrupt index on open failure     |
| `serve.sse`             | `false`            | Enable SSE transport by default           |
| `serve.sse_port`        | `8080`             | SSE port                                  |
| `serve.acp`             | `false`            | Enable ACP transport by default           |
| `serve.max_restarts`    | `10`               | Max transport restarts; `0` = no restart  |
| `serve.restart_backoff` | `1`                | Initial backoff seconds; doubles, cap 30s |
| `serve.heartbeat_secs`  | `60`               | Heartbeat interval; `0` = disabled        |
| `logging.log_path`      | `~/.llm-wiki/logs` | Log file directory; empty = stderr only   |
| `logging.log_rotation`  | `daily`            | `daily`, `hourly`, `never`                |
| `logging.log_max_files` | `7`                | Max rotated files; `0` = unlimited        |
| `logging.log_format`    | `text`             | `text` or `json`                          |


## Resolution Order

```
1. CLI flag
2. Per-wiki config   (wiki.toml)
3. Global config     (config.toml)
4. Built-in default
```
