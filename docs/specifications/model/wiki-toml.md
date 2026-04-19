---
title: "wiki.toml"
summary: "wiki.toml reference — identity, type overrides, per-wiki settings."
read_when:
  - Understanding what wiki.toml contains
  - Overriding a type's schema mapping
  - Overriding engine defaults for a specific wiki
status: ready
last_updated: "2025-07-18"
---

# wiki.toml

`wiki.toml` is the configuration file for a wiki repository. It lives
at the repo root, is committed to git, and is shared across all users
of the wiki.


## Complete Example

```toml
# Identity
name        = "research"
description = "ML research knowledge base"

# Type overrides (optional)
# Only needed to remap a type to a different schema file.
# Types are normally discovered from schemas/*.json via x-wiki-types.

# [types.paper]
# schema      = "schemas/my-custom-paper.json"
# description = "Custom paper schema with extra fields"

# Per-wiki settings (override global defaults)

[ingest]
auto_commit = true

[validation]
type_strictness = "loose"
```


## Sections

### Identity

| Field         | Required | Description                                               |
| ------------- | -------- | --------------------------------------------------------- |
| `name`        | yes      | Wiki name — used in `wiki://` URIs and the space registry |
| `description` | no       | One-line description — shown in `wiki_spaces_list`        |

### `[types.*]` — Type Overrides (optional)

Types are discovered automatically from `schemas/*.json` via
`x-wiki-types` (see [type-system.md](type-system.md)). Most wikis
need no `[types.*]` entries at all.

Use `[types.*]` only to override the discovered mapping — for example,
to point a type at a different schema file:

```toml
[types.paper]
schema      = "schemas/my-custom-paper.json"
description = "Custom paper schema with extra fields"
```

| Field         | Required | Description                                     |
| ------------- | -------- | ----------------------------------------------- |
| `schema`      | yes      | Path to JSON Schema file, relative to repo root |
| `description` | yes      | What this type is — readable by LLM and human   |

A `[types.*]` entry takes precedence over the same type discovered
from `x-wiki-types` in a schema file.

### Per-wiki settings

Any key from the global config that is not global-only can be overridden
here. See [global-config.md](global-config.md) for the full key
reference.
