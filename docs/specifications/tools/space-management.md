---
title: "Space Management"
summary: "llm-wiki spaces — create, list, remove, and set-default."
read_when:
  - Setting up a new wiki
  - Managing registered wiki spaces
status: ready
last_updated: "2025-07-17"
---

# Space Management

All space operations live under `llm-wiki spaces`:

| Subcommand           | MCP tool                  | Description                       |
| -------------------- | ------------------------- | --------------------------------- |
| `spaces create`      | `wiki_spaces_create`      | Create a new wiki repo + register |
| `spaces list`        | `wiki_spaces_list`        | List all registered wikis         |
| `spaces remove`      | `wiki_spaces_remove`      | Remove a wiki from the registry   |
| `spaces set-default` | `wiki_spaces_set_default` | Set the default wiki              |

For configuration (`wiki_config`), see
[config-management.md](config-management.md).

## spaces create

MCP tool: `wiki_spaces_create`

```
llm-wiki spaces create <path>
          --name <name>              # required — used in wiki:// URIs
          [--description <text>]
          [--force]                  # update space entry if name differs
          [--set-default]            # set as default_wiki
```

Creates the following structure (see
[wiki-repository-layout.md](../model/wiki-repository-layout.md)):

```
<path>/
├── README.md
├── wiki.toml
├── schemas/
│   ├── base.json
│   ├── concept.json
│   ├── paper.json
│   ├── skill.json
│   ├── doc.json
│   └── section.json
├── inbox/
├── raw/
└── wiki/
```

Initial git commit: `create: <name>`.

On first run, the wiki becomes the default one. Also ensures
`~/.llm-wiki/` infrastructure exists (config.toml, indexes/, logs/).

### Re-run behavior

| Condition                               | Behavior                        |
| --------------------------------------- | ------------------------------- |
| Path does not exist                     | Create everything, register     |
| Path exists, not registered             | Register in config.toml         |
| Path exists, registered, same name      | Skip silently                   |
| Path exists, registered, different name | Error (use `--force` to rename) |

## spaces list

MCP tool: `wiki_spaces_list`

```
llm-wiki spaces list
             [<name>]             # omit for all, provide to filter
             [--format <fmt>]     # text | json (default: text)
```

When `<name>` is omitted, lists all registered wikis.
When `<name>` is provided, returns a list with only that wiki's info.
If the name is not found, returns an empty list.

Text (default):

```
* research    /Users/geronimo/wikis/research    ML research knowledge base
  work        /Users/geronimo/wikis/work        —
```

`*` marks the current default.

JSON (`--format json`):

```json
[
  {
    "name": "research",
    "path": "/Users/geronimo/wikis/research",
    "description": "ML research knowledge base",
    "default": true
  },
  {
    "name": "work",
    "path": "/Users/geronimo/wikis/work",
    "description": null,
    "default": false
  }
]
```

## spaces remove

MCP tool: `wiki_spaces_remove`

```
llm-wiki spaces remove <name>
                   [--delete]     # also delete local directory
```

Refuses if the wiki is the current default — set a new default first.

## spaces set-default

MCP tool: `wiki_spaces_set_default`

```
llm-wiki spaces set-default <name>
```

Alias for `wiki_config set global.default_wiki <name>`.
