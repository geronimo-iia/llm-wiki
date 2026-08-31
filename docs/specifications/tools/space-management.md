---
title: "Space Management"
summary: "llm-wiki spaces — create, list, remove, and set-default."
read_when:
  - Setting up a new wiki
  - Managing registered wiki spaces
status: ready
last_updated: "2026-08-20"
---

# Space Management

All space operations live under `llm-wiki spaces`. When called from a
running server, create, remove, and set-default take effect immediately
— no restart needed. See [server.md](../engine/server.md#hot-reload).

| Subcommand           | MCP tool                  | Description                                  |
| -------------------- | ------------------------- | -------------------------------------------- |
| `spaces create`      | `wiki_spaces_create`      | Create a new wiki repo + register            |
| `spaces register`    | `wiki_spaces_register`    | Register an existing repo without creating files |
| `spaces list`        | `wiki_spaces_list`        | List all registered wikis                    |
| `spaces remove`      | `wiki_spaces_remove`      | Remove a wiki from the registry              |
| `spaces set-default` | `wiki_spaces_set_default` | Set the default wiki                         |

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
          [--wiki-root <dir>]        # content dir name (default: wiki)
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
└── wiki/              ← or the value of --wiki-root
```

Initial git commit: `create: <name>`.

On first run, the wiki becomes the default one. Also ensures
`~/.llm-wiki/` infrastructure exists (config.toml, indexes/, logs/).

When called from a running server, the new wiki is mounted
immediately — searchable and indexable without restart.

### Re-run behavior

| Condition                               | Behavior                        |
| --------------------------------------- | ------------------------------- |
| Path does not exist                     | Create everything, register     |
| Path exists, not registered             | Register in config.toml         |
| Path exists, registered, same name      | Skip silently                   |
| Path exists, registered, different name | Error (use `--force` to rename) |

## spaces register

MCP tool: `wiki_spaces_register`

```
llm-wiki spaces register <path>
          --name <name>              # required
          [--description <text>]
          [--wiki-root <dir>]        # override wiki_root (errors if conflicts with wiki.toml)
```

Registers an existing git repository without creating any files or
making any git commits. Use this to adopt a repo that already has
content (e.g. a `docs/` repo, a Hugo site, or any repo where pages
already exist in a subdirectory other than `wiki/`).

The command reads `wiki.toml` from `<path>` to determine the effective
`wiki_root`. If `--wiki-root` is given and `wiki.toml` already declares
a different value, the command errors — edit `wiki.toml` manually instead.

If the directory named by `wiki_root` does not exist, the command errors.

| Condition                                     | Behavior                                       |
| --------------------------------------------- | ---------------------------------------------- |
| `<path>` does not exist                       | Error                                          |
| `wiki.toml` absent, no `--wiki-root`          | `wiki_root` defaults to `"wiki"`               |
| `wiki.toml` has `wiki_root`, no flag          | Uses value from `wiki.toml`                    |
| `--wiki-root` matches `wiki.toml`             | OK                                             |
| `--wiki-root` conflicts with `wiki.toml`      | Error — edit `wiki.toml` first                 |
| Already registered under same name            | Skip silently                                  |

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

When called from a running server, the wiki is unmounted immediately.
In-flight requests complete normally.

## spaces set-default

MCP tool: `wiki_spaces_set_default`

```
llm-wiki spaces set-default <name>
```

Alias for `wiki_config set global.default_wiki <name>`.

When called from a running server, the default updates immediately.

## Atomicity guarantees

`wiki_spaces_create`, `wiki_spaces_register`, and `wiki_spaces_set_default`
are atomic with respect to disk and in-memory state:

- If the `wiki.toml` write fails after in-memory state has been updated,
  the engine rolls back automatically — no manual recovery needed.
- Concurrent space mutations are serialised by an internal config lock.
  A rebuild triggered by a file-watcher event on a newly mounted space
  runs only after the space is fully registered.

If a `wiki_spaces_create` or `wiki_spaces_register` call returns an
error, no partial state is left behind: the config entry is removed and
the space is not mounted.
