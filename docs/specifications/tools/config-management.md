---
title: "Config Management Tool"
summary: "get, set, and list configuration values."
read_when:
  - Reading or writing configuration via CLI or MCP
status: ready
last_updated: "2026-07-23"
---

# Config Management

MCP tool: `wiki_config`

| Command                    | Description              |
| -------------------------- | ------------------------ |
| `config get <key>`         | Read a config value      |
| `config set <key> <value>` | Write a config value     |
| `config list`              | List all resolved config |

Operates on `wiki.toml` (per-wiki) or `~/.llm-wiki/config.toml`
(global). For the file formats and full key reference, see
[wiki-toml.md](../model/wiki-toml.md) and
[global-config.md](../model/global-config.md).


## CLI

```
llm-wiki config get <key>
llm-wiki config set <key> <value>
                [--global]          # write to ~/.llm-wiki/config.toml
                [--wiki <name>]     # write to per-wiki wiki.toml
llm-wiki config list
             [--global]
             [--wiki <name>]
             [--format <fmt>]    # text | json (default: text)
```

`set` without `--global` writes to the per-wiki `wiki.toml` of the
default wiki (or `--wiki <name>` target). With `--global` it writes to
`~/.llm-wiki/config.toml`.

Global-only keys (`index.*`, `serve.*`, `logging.*`) reject
`--wiki` with an error.

### Examples

```bash
llm-wiki config get defaults.search_top_k
llm-wiki config set defaults.search_top_k 15 --global
llm-wiki config set defaults.page_mode bundle --wiki research
llm-wiki config list
llm-wiki config list --global --format json

# search.status multipliers — <key> is any status string (built-in or custom)
llm-wiki config set search.status.archived 0.1 --global
llm-wiki config set search.status.superseded 0.05 --wiki research
llm-wiki config get search.status.archived
```

### config list output

Text (default):

```
defaults.search_top_k    = 10
defaults.page_mode       = flat
ingest.auto_commit       = true
validation.type_strictness = loose
```

JSON (`--format json`):

```json
{
  "defaults.search_top_k": 10,
  "defaults.page_mode": "flat",
  "ingest.auto_commit": true,
  "validation.type_strictness": "loose"
}
```
