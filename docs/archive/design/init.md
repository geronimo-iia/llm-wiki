---
title: "Init"
summary: "Initialize a new wiki — create directory structure, git repo, and register in ~/.wiki/config.toml. Safe to re-run."
read_when:
  - Implementing or extending the init command
  - Understanding what wiki init creates and registers
  - Setting up a new wiki from scratch
status: draft
last_updated: "2025-07-15"
---

# Init

`wiki init` creates a new wiki at a given path, initializes a git repository,
creates the default directory structure, writes an initial commit, and
registers the wiki in `~/.wiki/config.toml`. Safe to re-run.

---

## 1. What Init Creates

```
<path>/
├── concepts/
├── sources/
├── queries/
├── raw/
└── .wiki/
    └── config.toml     ← per-wiki config with name and description
```

`.gitignore` is written at the wiki root:

```
.wiki/search-index/
```

Initial git commit: `init: <name>`

---

## 2. CLI Interface

```
wiki init <path>
          --name <name>              # wiki name — required, used in wiki:// URIs
          [--description <text>]     # optional one-line description
          [--force]                  # update registry if name differs from existing
          [--set-default]            # set as default_wiki in ~/.wiki/config.toml
```

### Examples

```bash
wiki init ~/wikis/research --name research
wiki init ~/wikis/research --name research --description "ML research knowledge base"
wiki init ~/wikis/research --name research --set-default
wiki init ~/wikis/research --name research-v2 --force   # rename in registry
```

---

## 3. Re-run Behavior

| Condition | Behavior |
|-----------|----------|
| Path does not exist | Create directory, git repo, structure, commit, register |
| Path exists, not a git repo | `git init`, create missing dirs, commit, register |
| Path exists, git repo, not registered | Register in `~/.wiki/config.toml` |
| Path exists, registered, same name | Skip silently — already initialized |
| Path exists, registered, different name | Error: `wiki already registered as "<old-name>". Use --force to rename.` |
| `--force` with different name | Update registry entry with new name |
| Description changed | Always update silently |

---

## 4. Registry Entry Written

Appended to `~/.wiki/config.toml`:

```toml
[[wikis]]
name        = "research"
path        = "/Users/geronimo/wikis/research"
description = "ML research knowledge base"   # omitted if not provided
```

If `--set-default`:

```toml
[global]
default_wiki = "research"
```

If `[global]` already exists, only `default_wiki` is updated.

---

## 5. Per-Wiki Config Written

`.wiki/config.toml` inside the new wiki:

```toml
name        = "research"
description = "ML research knowledge base"
```

---

## 6. MCP Tool

```rust
#[tool(description = "Initialize a new wiki and register it")]
async fn wiki_init(
    &self,
    #[tool(param)] path: String,
    #[tool(param)] name: String,
    #[tool(param)] description: Option<String>,
    #[tool(param)] force: Option<bool>,
    #[tool(param)] set_default: Option<bool>,
) -> InitReport { ... }

pub struct InitReport {
    pub path:       String,
    pub name:       String,
    pub created:    bool,    // false if already existed
    pub registered: bool,    // false if already registered with same name
    pub committed:  bool,
}
```

---

## 7. Rust Module Changes

| Module | Change |
|--------|--------|
| `cli.rs` | Add `init` subcommand with `<path>`, `--name`, `--description`, `--force`, `--set-default` |
| `registry.rs` | Add `register(entry, force)` — append or update `~/.wiki/config.toml` |
| `git.rs` | Add `init_repo(path)` — `git init` + initial commit |
| `integrate.rs` | Add `init_structure(wiki_root)` — create dirs, `.gitignore`, per-wiki config |
| `server.rs` | Add `wiki_init` MCP tool |

---

## 8. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki init <path> --name` | **not implemented** |
| Directory structure creation | **not implemented** |
| `.gitignore` creation | **not implemented** |
| Initial git commit | **not implemented** |
| Registry registration | **not implemented** |
| `--set-default` | **not implemented** |
| `--force` rename | **not implemented** |
| `wiki_init` MCP tool | **not implemented** |
