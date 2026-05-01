---
title: "config.toml"
summary: "Global engine config at ~/.llm-wiki/config.toml — space registry, defaults, and global-only settings."
read_when:
  - Understanding what config.toml contains
  - Looking up a config key and its default
  - Understanding which keys are global-only vs per-wiki
status: ready
last_updated: "2025-07-21"
---

# config.toml

The global engine configuration holds the space registry and all global
defaults. Local to the machine, never committed.

**Location resolution order:**
1. `--config <path>` CLI flag
2. `LLM_WIKI_CONFIG` environment variable
3. `~/.llm-wiki/config.toml` (default)

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
output_format   = "text"
facets_top_tags = 10

[read]
no_frontmatter = false

[ingest]
auto_commit = true

[validation]
type_strictness = "loose"

[graph]
format = "mermaid"
depth  = 3
min_nodes_for_communities   = 30  # suppress below this; default 30
community_suggestions_limit = 2   # extra suggest results from community strategy

[history]
follow        = true
default_limit = 10

[suggest]
default_limit = 5
min_score     = 0.1

[search.status]
active   = 1.0
draft    = 0.8
archived = 0.3
unknown  = 0.9

# ── Global-only settings ──────────────────────────────────────────────────────

[index]
auto_rebuild  = false
auto_recovery = true

[serve]
http             = false
http_port        = 8080
http_allowed_hosts = ["localhost", "127.0.0.1", "::1"]
acp             = false
max_restarts    = 10
restart_backoff = 1
heartbeat_secs  = 60
acp_max_sessions = 20

[logging]
log_path      = "~/.llm-wiki/logs"
log_rotation  = "daily"
log_max_files = 7
log_format    = "text"

[watch]
debounce_ms = 500
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
| `defaults.output_format`     | `text`    | Default output format: `text` or `json`           |
| `defaults.facets_top_tags`   | `10`      | Max tag facet entries returned; `0` = all          |
| `history.follow`             | `true`    | Track renames in `wiki_history`                    |
| `history.default_limit`      | `10`      | Default entry count for `wiki_history`             |
| `suggest.default_limit`      | `5`       | Max suggestions for `wiki_suggest`                 |
| `suggest.min_score`          | `0.1`     | Minimum score threshold for suggestions            |
| `lint.stale_days`            | `90`      | Days before a page's `last_updated` is considered old |
| `lint.stale_confidence_threshold` | `0.4` | Confidence below this threshold (AND old) = stale finding |
| `search.status`              | `{ active=1.0, draft=0.8, archived=0.3, unknown=0.9 }` | Status multiplier map. `unknown` is the reserved fallback for absent or unmapped statuses. Add custom entries (`stub`, `verified`, …) alongside built-ins. Per-wiki resolution merges key-by-key — a `wiki.toml` only needs to declare what differs. |
| `read.no_frontmatter`        | `false`   | Strip frontmatter from `wiki_content_read` output         |
| `ingest.auto_commit`         | `true`    | Commit after ingest                               |
| `validation.type_strictness` | `loose`   | `strict`: unknown type is error; `loose`: warning |
| `graph.format`               | `mermaid` | Default output format: `mermaid` or `dot`         |
| `graph.depth`                | `3`       | Default hop limit when `--root` is set            |
| `graph.type`                 | `[]`      | Page types to include; empty = all                |
| `graph.output`               | `""`      | Default output path; empty = stdout               |
| `graph.min_nodes_for_communities` | `30` | Suppress community detection below this node count |
| `graph.community_suggestions_limit` | `2` | Max extra results from community strategy in `wiki_suggest` |
| `index.memory_budget_mb`     | `50`      | Tantivy writer memory budget in MB                |
| `index.tokenizer`            | `en_stem` | Tantivy tokenizer for text fields                 |

### Global-only settings

These keys can only appear in `config.toml`. Setting them in `wiki.toml`
is rejected.

| Key                     | Default            | Description                               |
| ----------------------- | ------------------ | ----------------------------------------- |
| `index.auto_rebuild`    | `false`            | Rebuild stale index before search/list    |
| `index.auto_recovery`   | `true`             | Rebuild corrupt index on open failure     |
| `serve.http`             | `false`            | Enable HTTP transport by default          |
| `serve.http_port`        | `8080`             | HTTP port                                 |
| `serve.http_allowed_hosts` | `localhost,127.0.0.1,::1` | Allowed Host headers (DNS rebinding protection) |
| `serve.acp`             | `false`            | Enable ACP transport by default           |
| `serve.max_restarts`    | `10`               | Max transport restarts; `0` = no restart  |
| `serve.restart_backoff` | `1`                | Initial backoff seconds; doubles, cap 30s |
| `serve.heartbeat_secs`  | `60`               | Heartbeat interval; `0` = disabled        |
| `serve.acp_max_sessions` | `20`              | Max concurrent ACP sessions; `NewSession` returns an error when reached |
| `watch.debounce_ms`    | `500`              | Filesystem watcher debounce interval in ms |
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

Most sections resolve all-or-nothing: if a section is present in
`wiki.toml`, it replaces the global section entirely. The one exception
is `[search.status]`, which merges **key by key** — the global map
provides the baseline, and per-wiki entries override or extend
individual keys. A `wiki.toml` only needs to declare the entries it
wants to change or add.
