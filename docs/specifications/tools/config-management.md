---
title: "Config Management Tool"
summary: "get, set, and list configuration values."
read_when:
  - Reading or writing configuration via CLI or MCP
status: ready
last_updated: "2025-07-17"
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
llm-wiki config set ingest.auto_commit false --wiki research
llm-wiki config list
llm-wiki config list --global
llm-wiki config list --wiki research
```
