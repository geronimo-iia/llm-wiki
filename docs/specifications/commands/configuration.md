---
title: "Configuration"
summary: "Two-level config — global config at ~/.llm-wiki/config.toml and per-wiki config at wiki.toml in the repo root. llm-wiki config command for reading and writing both levels."
read_when:
  - Implementing or extending the configuration system
  - Adding a new configurable default (search, page creation, etc.)
  - Understanding the difference between global and per-wiki config
  - Using llm-wiki config to inspect or change settings
status: draft
last_updated: "2025-07-15"
---

# Configuration

Two config files, two scopes:

- `~/.llm-wiki/config.toml` — global: llm-wiki spaces, global defaults
- `wiki.toml` — per-wiki: identity, overrides, wiki-specific defaults

Per-wiki values take precedence over global values. CLI flags take precedence
over both.

---

## 1. Global Config — `~/.llm-wiki/config.toml`

```toml
# ── Global settings ────────────────────────────────────────────────────────────

[global]
default_wiki = "research"   # wiki used when no --wiki flag or wiki:// name given

# ── Wiki spaces ────────────────────────────────────────────────────────────────

[[wikis]]
name = "research"
path = "/Users/geronimo/wikis/research"

[[wikis]]
name   = "work"
path   = "/Users/geronimo/wikis/work"
remote = "git@github.com:org/work-wiki.git"

# ── Global defaults (apply to all wikis unless overridden) ─────────────────────

[defaults]
search_top_k      = 10     # default --top-k for llm-wiki search
search_excerpt    = true   # false to default to --no-excerpt behavior
search_sections   = false  # true to include section index pages in results
page_mode         = "flat" # default page creation mode: flat | bundle
list_page_size    = 20     # default page size for llm-wiki list pagination

[read]
no_frontmatter = false  # true to strip frontmatter from llm-wiki read output by default

[index]
auto_rebuild  = false  # true to rebuild stale index automatically before search/list
auto_recovery = true   # true to rebuild corrupt index on open failure

[graph]
format  = "mermaid"  # mermaid | dot
depth   = 3          # default hop limit (used when --root is set)
type    = []         # empty = all types; e.g. ["concept", "source"]
output  = ""         # empty = stdout; path or wiki:// URI

[serve]
sse              = false    # enable SSE transport by default
sse_port         = 8080     # SSE port
acp              = false    # enable ACP transport by default
max_restarts     = 10       # max transport restarts before exit; 0 = no restart
restart_backoff  = 1        # initial backoff seconds; doubles each restart, cap 30s
heartbeat_secs   = 60       # heartbeat log interval; 0 = disabled

[ingest]
auto_commit = true  # commit after ingest; false = validate + index only

[validation]
type_strictness = "loose"  # strict | loose

[logging]
log_path      = "~/.llm-wiki/logs"  # log file directory; empty = disable file logging
log_rotation  = "daily"             # daily | hourly | never
log_max_files = 7                   # max rotated files; 0 = unlimited
log_format    = "text"              # text | json

[lint]
fix_missing_stubs   = true   # auto-create scaffold pages for missing stubs
fix_empty_sections  = true   # auto-create index.md for empty sections
```

---

## 2. Per-Wiki Config — `wiki.toml`

Lives at the wiki repository root. Committed and versioned with the wiki —
shared across all users of the same wiki.

```toml
# ── Wiki identity ──────────────────────────────────────────────────────────────

name        = "research"
description = "ML research knowledge base"

# ── Per-wiki overrides ─────────────────────────────────────────────────────────

[defaults]
search_top_k    = 15     # this wiki has more pages, raise the default
search_excerpt  = false  # refs only for this wiki
search_sections = true   # this wiki uses sections as navigation, include them
page_mode       = "flat"

[ingest]
auto_commit = false  # human reviews before committing

[validation]
type_strictness = "strict"  # override global loose default for this wiki

[lint]
fix_missing_stubs  = false  # do not auto-create stubs for this wiki
```

---

## 3. Config Keys Reference

| Key | Scope | Default | Description |
|-----|-------|---------|-------------|
| `global.default_wiki` | global only | — | Wiki name used when no `--wiki` flag or `wiki://` name is given |
| `defaults.search_top_k` | global / per-wiki | `10` | Default result count for `llm-wiki search` |
| `defaults.search_excerpt` | global / per-wiki | `true` | Include excerpts by default; `false` behaves like `--no-excerpt` |
| `defaults.search_sections` | global / per-wiki | `false` | Include section index pages in results; `true` behaves like `--include-sections` |
| `defaults.page_mode` | global / per-wiki | `flat` | Default page creation mode: `flat` or `bundle` |
| `defaults.list_page_size` | global / per-wiki | `20` | Default page size for `llm-wiki list` pagination |
| `ingest.auto_commit` | global / per-wiki | `true` | Commit automatically after ingest; `false` requires explicit `llm-wiki commit` |
| `read.no_frontmatter` | global / per-wiki | `false` | Strip frontmatter from `llm-wiki read` output by default |
| `index.auto_rebuild` | global only | `false` | Automatically rebuild stale index before search/list |
| `index.auto_recovery` | global only | `true` | Automatically rebuild corrupt index on open failure |
| `graph.format` | global / per-wiki | `mermaid` | Default output format: `mermaid` or `dot` |
| `graph.depth` | global / per-wiki | `3` | Default hop limit when `--root` is set |
| `graph.type` | global / per-wiki | `[]` | Page types to include; empty = all types |
| `graph.output` | global / per-wiki | `""` | Default output path; empty = stdout |
| `validation.type_strictness` | global / per-wiki | `loose` | `strict` — unknown type is an error; `loose` — unknown type is a warning, ingest proceeds |
| `serve.sse` | global only | `false` | Enable SSE transport by default |
| `serve.sse_port` | global only | `8080` | SSE port |
| `serve.acp` | global only | `false` | Enable ACP transport by default |
| `serve.max_restarts` | global only | `10` | Max transport restarts before exit. `0` = no restart (crash exits). |
| `serve.restart_backoff` | global only | `1` | Initial backoff in seconds. Doubles on each restart, capped at 30s. |
| `serve.heartbeat_secs` | global only | `60` | Heartbeat log interval in seconds. `0` = disabled. |
| `logging.log_path` | global only | `~/.llm-wiki/logs` | Log file directory. Empty string disables file logging. |
| `logging.log_rotation` | global only | `daily` | Rotation schedule: `daily`, `hourly`, `never` |
| `logging.log_max_files` | global only | `7` | Max rotated log files. `0` = unlimited. |
| `logging.log_format` | global only | `text` | Output format: `text` or `json` |
| `lint.fix_missing_stubs` | global / per-wiki | `true` | Auto-create scaffold pages for missing stubs on `llm-wiki lint fix` |
| `lint.fix_empty_sections` | global / per-wiki | `true` | Auto-create `index.md` for empty sections on `llm-wiki lint fix` |

---

## 4. CLI — `llm-wiki config`

```
llm-wiki config get <key>                  # print a config value
llm-wiki config set <key> <value>          # set a config value
llm-wiki config list                       # print all resolved config (global + per-wiki merged)
             [--global]                # global config only
             [--wiki <name>]           # per-wiki config only
```

### Examples

```bash
# Read resolved value (per-wiki overrides global)
llm-wiki config get defaults.search_top_k

# Set a global default
llm-wiki config set defaults.search_top_k 15 --global

# Set a per-wiki override
llm-wiki config set defaults.page_mode bundle --wiki research

# Inspect everything
llm-wiki config list
llm-wiki config list --global
llm-wiki config list --wiki research
```

`llm-wiki config set` without `--global` writes to the per-wiki `wiki.toml` of
the default wiki (or `--wiki <name>` target). With `--global` it writes to
`~/.llm-wiki/config.toml`.

---

## 5. Resolution Order

For any config key, the resolved value is the first match in this order:

```
1. CLI flag          (e.g. --top-k 20)
2. Per-wiki config   (wiki.toml)
3. Global config     (~/.llm-wiki/config.toml)
4. Built-in default  (hardcoded in config.rs)
```

---

## 6. MCP Tool

```rust
#[tool(description = "Get or set llm-wiki configuration values")]
async fn wiki_config(
    &self,
    #[tool(param)] action: String,       // "get" | "set" | "list"
    #[tool(param)] key: Option<String>,
    #[tool(param)] value: Option<String>,
    #[tool(param)] global: Option<bool>,
    #[tool(param)] wiki: Option<String>,
) -> String { ... }
```
