---
title: "Design: Remote Wiki Registration and Version Management"
summary: "Add remote URL support to spaces register, version lifecycle subcommands (install, use, update, uninstall), and default_repo_root in global config."
read_when:
  - Implementing remote wiki registration in the engine
  - Implementing spaces install / use / update / uninstall
  - Understanding how managed spaces differ from local-path spaces
status: proposal
last_updated: "2026-05-01"
---

# Design: Remote Wiki Registration and Version Management

**Scope:** (A) `spaces register` accepting a git remote URL with cloning,
(B) version lifecycle subcommands under `spaces` (install, use, update, uninstall),
(C) `default_repo_root` in global config.

## Problem

`spaces register` today requires a local path. Users who want to share a wiki
repo (e.g. `llm-wiki-skills`) must clone it manually before registering.
There is no way to pin a specific version, update to a newer tag, or switch
between versions — the repo is just a directory with no managed lifecycle.

| Gap | Impact |
|-----|--------|
| No remote registration | Manual `git clone` + `spaces register` — two steps, error-prone path management |
| No version lifecycle | Can't pin a tag, can't `update`, can't rollback |

## Global Config Addition

Add `default_repo_root` to `[global]` in `config.toml` — global-only:

```toml
[global]
default_wiki      = "research"
default_repo_root = "~/.llm-wiki/repos"   # default value
```

| Key | Default | Description |
|-----|---------|-------------|
| `global.default_repo_root` | `~/.llm-wiki/repos` | Base directory where remote wikis are cloned. Tilde-expanded. |

All `spaces` commands that clone a remote use this as the parent directory
unless `--install-dir` overrides it. Global-only — cannot be set in `wiki.toml`.

`specifications/model/global-config.md` must be updated.

## Feature A — Remote URL in `spaces register`

### Proposed CLI

```
llm-wiki spaces register <path-or-url>
          --name <name>
          [--wiki-root <dir>]
          [--tag <tag>]           # pin to a specific git tag
          [--branch <branch>]     # track a branch instead of default
          [--install-dir <dir>]   # override default_repo_root/<name>
```

### Behavior

When `<path-or-url>` is detected as a remote URL (`https://`, `git@`,
`git://`, `ssh://`):

1. Resolve install dir: `--install-dir` ?? `default_repo_root/<name>`
2. `git clone` the URL into the install dir (full clone)
3. If `--tag`: `git checkout <tag>` after clone
4. If `--branch`: clone with `--branch <branch>` and track it
5. Read `wiki.toml` from clone — extract `wiki_root` if present
6. Apply `--wiki-root` if given (conflict check against `wiki.toml`)
7. Register the local clone path in `config.toml`

If clone fails: hard error, nothing registered.
If install dir already exists and is a git repo: error — use `spaces update` instead.

### MCP tool

`wiki_spaces_register` gains optional parameters:

```json
{
  "url":         "https://github.com/geronimo-iia/llm-wiki-skills",
  "name":        "llm-wiki-skills",
  "tag":         "v1.2.0",
  "wiki_root":   "skills",
  "install_dir": null
}
```

When `url` is set, `path` is ignored.

## Feature B — Version Lifecycle Subcommands under `spaces`

All version management is folded into `spaces`.

### New subcommands

```
llm-wiki spaces list-remote    <name-or-url>         # list available tags from remote
llm-wiki spaces install        <name-or-url> [<tag>] # clone and register
llm-wiki spaces use            <name> <tag>          # switch to a different tag
llm-wiki spaces update         <name>                # fetch and switch to latest tag
llm-wiki spaces uninstall      <name>                # unregister and delete clone
```

### Directory layout

```
<default_repo_root>/
└── llm-wiki-skills/      ← single git clone, HEAD at active tag
    └── <cloned repo files>
```

The registered space `path` points directly to the clone directory.
Switching versions = `git checkout <tag>` + hot-remount. No multiple
local copies. No separate metadata file — all managed space state lives
in the `[[wikis]]` entry in `config.toml`.

### `config.toml` — extended space entry

Managed spaces carry one additional field in their `[[wikis]]` entry:

```toml
[[wikis]]
name        = "llm-wiki-skills"
path        = "~/.llm-wiki/repos/llm-wiki-skills"
description = "Skills for the llm-wiki engine"
wiki_root   = "skills"
```

`url` and `active_tag` are not stored — derived on demand from the clone:

```bash
git -C <path> remote get-url origin              # → url
git -C <path> describe --tags --exact-match HEAD # → active tag
```

`wiki_root` is the only new field, valid for both managed and unmanaged spaces.

`specifications/model/global-config.md` must document the new `wiki_root`
field on `[[wikis]]`.

### `spaces list-remote` — tag discovery

```
$ llm-wiki spaces list-remote llm-wiki-skills
v1.2.0  (latest)
v1.1.0
v1.0.0
```

Accepts either a registered space name (reads URL via `git remote get-url origin`
from the clone) or a raw URL. Calls `git ls-remote --tags <url>`, strips
`^{}` derefs, sorts semver descending. Shells out to `git` — no clone needed.

Error if space has no remote: `"<name>: no git remote configured — cannot list remote tags"`

### `spaces install` — clone a repo

```
llm-wiki spaces install <name-or-url> [<tag-or-branch>]
                        [--branch]    # treat argument as branch, not tag
```

```
llm-wiki spaces install llm-wiki-skills v1.2.0
llm-wiki spaces install https://github.com/geronimo-iia/llm-wiki-skills v1.2.0
llm-wiki spaces install https://github.com/geronimo-iia/llm-wiki-skills main --branch
```

1. If URL given: resolve name from `wiki.toml` or `--name`
2. Clone into `<default_repo_root>/<name>/` (full clone — supports `git checkout` later)
3. If `--branch`: clone with `--branch <branch>` and track it
4. If tag: `git checkout <tag>` after clone
5. If neither: check out latest tag (calls `list-remote` first)
6. Register space with `wiki_root` in `config.toml`

Error if clone directory already exists — use `spaces use` or `spaces update`.
Error if space has no git remote: `"<name>: no git remote configured — version commands unavailable"`

### `spaces use` — switch active version

```
llm-wiki spaces use llm-wiki-skills v1.1.0
```

1. If remote exists: `git fetch --tags`, then verify tag locally — error if not found
2. If no remote: check local tags only — error if not found
3. `git checkout <tag>`
4. If server running: unmount + remount (hot-reload)
5. Print: `llm-wiki-skills → v1.1.0`

### `spaces update` — fetch and switch to latest

```
llm-wiki spaces update llm-wiki-skills
```

1. Check remote exists — error: `"<name>: no git remote configured — use 'spaces use' to switch manually"`
2. `list-remote` to find latest tag
3. Read current tag via `git describe --tags --exact-match HEAD`
4. If already at latest: print "already at v1.2.0", exit 0
5. `git fetch --tags` in clone directory
6. `git checkout <latest-tag>`
7. If server running: unmount + remount (hot-reload) — auto, no prompt
8. Print: `llm-wiki-skills: v1.1.0 → v1.2.0`

### `spaces uninstall` — remove a managed space

```
llm-wiki spaces uninstall llm-wiki-skills
```

1. Unregister the space from `config.toml` (remove `[[wikis]]` entry)
2. Delete `<default_repo_root>/llm-wiki-skills/` (entire clone directory)

Requires confirmation prompt — destructive. Add `--yes` to skip.

### `spaces list` — show managed metadata

`spaces list` output gains two optional fields for managed spaces:

Text:
```
* llm-wiki-skills  ~/.llm-wiki/repos/llm-wiki-skills  Skills for the engine  [managed v1.2.0]
```

JSON:
```json
{
  "name": "llm-wiki-skills",
  "path": "~/.llm-wiki/repos/llm-wiki-skills",
  "managed": true,
  "active_version": "v1.2.0"
}
```

Unmanaged (local-path) spaces: `"managed": false`, `"active_version": null`.

## `spaces register <url>` as shorthand

`spaces register <url>` without `--tag` is equivalent to:

```
spaces install <url> <latest-tag>
```

With `--tag <t>`: installs and activates that tag.

Repos registered via `spaces register <local-path>` remain unmanaged —
no git remote, version switching not available. The two modes are independent.

## Impact Analysis

| Area | Impact |
|------|--------|
| `config.toml` / global config spec | Add `global.default_repo_root` (global-only) |
| `spaces register` | URL detection + clone logic before existing registration path |
| `wiki_spaces_register` MCP tool | Add `url`, `tag`, `branch`, `install_dir` params |
| New MCP tools | `wiki_spaces_list_remote`, `wiki_spaces_install`, `wiki_spaces_use`, `wiki_spaces_update`, `wiki_spaces_uninstall` |
| `wiki_spaces_list` response | Add `managed` (bool), `active_version` (string\|null) fields — derived from git |
| `[[wikis]]` entry in `config.toml` | Add optional `wiki_root` field only — `url` and `active_tag` read from git |
| `SpaceContext` | No change — always resolves to a local path |
| `spaces uninstall` | Distinct from `spaces remove` — also deletes clone directory |
| Hot-reload | `spaces use` / `spaces update` reuse existing unmount+remount |
| git dependency | `list-remote`, clone, fetch, checkout shell out to `git` — must be on PATH |
| `~/.llm-wiki/` layout | Add `repos/<name>/` (single clone per managed space) |
| `specifications/tools/space-management.md` | Document all new subcommands + MCP tools |
| `specifications/model/global-config.md` | Add `global.default_repo_root` + `wiki_root` field on `[[wikis]]` |
