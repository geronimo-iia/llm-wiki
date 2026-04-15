---
title: "Registry"
summary: "Manage the wiki registry — list registered wikis, remove entries, and set the default wiki."
read_when:
  - Implementing or extending the registry command
  - Listing, removing, or changing the default wiki
  - Understanding how the registry relates to ~/.wiki/config.toml
status: draft
last_updated: "2025-07-15"
---

# Registry

`wiki registry` manages the wiki registry in `~/.wiki/config.toml`. It provides
subcommands to list registered wikis, remove entries, and set the default wiki.

---

## 1. Subcommands

### `wiki registry list`

Prints all registered wikis with their name, path, description, and whether
they are the current default.

```bash
wiki registry list
```

Output:

```
  name        path                              description
* research    /Users/geronimo/wikis/research    ML research knowledge base
  work        /Users/geronimo/wikis/work        —
  sp-theory   /Users/geronimo/build/sp_theory   SP theory knowledge base
```

`*` marks the current default wiki.

---

### `wiki registry remove <name>`

Removes a wiki entry from `~/.wiki/config.toml`. Refuses if the wiki is the
current default — set a new default first with `wiki registry set-default`.

```bash
wiki registry remove work
wiki registry remove work --delete   # also delete the local directory
```

Flags:

```
wiki registry remove <name>
                     [--delete]   # also delete the wiki directory from disk
```

Errors:

| Condition | Error |
|-----------|-------|
| Name not found | `error: wiki "work" is not registered` |
| Is current default | `error: "work" is the default wiki — set a new default first` |
| `--delete` but path does not exist | Warning only, registry entry still removed |

Git commit is not made — registry changes are local only.

---

### `wiki registry set-default <name>`

Sets the default wiki. Thin alias for `wiki config set global.default_wiki <name>`.

```bash
wiki registry set-default research
```

Errors:

| Condition | Error |
|-----------|-------|
| Name not found | `error: wiki "unknown" is not registered` |

---

## 2. MCP Tools

```rust
#[tool(description = "List all registered wikis")]
async fn wiki_registry_list(&self) -> Vec<RegistryEntry> { ... }

#[tool(description = "Remove a wiki from the registry")]
async fn wiki_registry_remove(
    &self,
    #[tool(param)] name: String,
    #[tool(param)] delete: Option<bool>,
) -> String { ... }

#[tool(description = "Set the default wiki — alias for wiki config set global.default_wiki")]
async fn wiki_registry_set_default(
    &self,
    #[tool(param)] name: String,
) -> String { ... }

pub struct RegistryEntry {
    pub name:        String,
    pub path:        String,
    pub description: Option<String>,
    pub default:     bool,
}
```

---

## 3. Rust Module Changes

| Module | Change |
|--------|--------|
| `registry.rs` | Add `list()`, `remove(name, delete)` — read/write `~/.wiki/config.toml` |
| `cli.rs` | Add `registry` subcommand with `list`, `remove`, `set-default` |
| `server.rs` | Add `wiki_registry_list`, `wiki_registry_remove`, `wiki_registry_set_default` MCP tools |

`set-default` delegates to `config::set("global.default_wiki", name)` — no
new logic in `registry.rs`.

---

## 4. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki registry list` | **not implemented** |
| `wiki registry remove <name>` | **not implemented** |
| `wiki registry remove --delete` | **not implemented** |
| `wiki registry set-default <name>` | **not implemented** |
| `wiki_registry_list` MCP tool | **not implemented** |
| `wiki_registry_remove` MCP tool | **not implemented** |
| `wiki_registry_set_default` MCP tool | **not implemented** |
