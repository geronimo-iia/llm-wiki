---
title: "wiki.toml"
summary: "wiki.toml reference — identity, type registry, per-wiki settings."
read_when:
  - Understanding what wiki.toml contains
  - Adding or modifying types in a wiki
  - Overriding engine defaults for a specific wiki
status: ready
last_updated: "2025-07-17"
---

# wiki.toml

`wiki.toml` is the single configuration file for a wiki repository. It
lives at the repo root, is committed to git, and is shared across all
users of the wiki.


## Complete Example

```toml
# ── Identity ───────────────────────────────────────────────────────────────────

[wiki]
name        = "research"
description = "ML research knowledge base"

# ── Type registry ──────────────────────────────────────────────────────────────

[types.default]
schema      = "schemas/base.json"
description = "Fallback for unrecognized types"

[types.concept]
schema      = "schemas/concept.json"
description = "Synthesized knowledge — one concept per page"

[types.paper]
schema      = "schemas/paper.json"
description = "Academic source — research papers, preprints"

[types.section]
schema      = "schemas/section.json"
description = "Section index grouping related pages"

[types.skill]
schema      = "schemas/skill.json"
description = "Agent skill with workflow instructions"

# ... more types — see type-system.md for the full default list

# ── Per-wiki settings (override global defaults) ──────────────────────────────

[ingest]
auto_commit = true

[search]
top_k = 10

[validation]
type_strictness = "loose"
```


## Sections

### `[wiki]` — Identity

| Field         | Required | Description                                               |
| ------------- | -------- | --------------------------------------------------------- |
| `name`        | yes      | Wiki name — used in `wiki://` URIs and the space registry |
| `description` | no       | One-line description — shown in `wiki_spaces_list`        |

### `[types.*]` — Type Registry

Each `[types.<name>]` entry registers a page type. The engine uses it
on ingest to find the right JSON Schema for validation.

| Field         | Required | Description                                     |
| ------------- | -------- | ----------------------------------------------- |
| `schema`      | yes      | Path to JSON Schema file, relative to repo root |
| `description` | yes      | What this type is — readable by LLM and human   |

`[types.default]` is the fallback for pages with an unrecognized or
missing `type` field.

For the full type system — schemas, field aliasing, graph edges — see
[type-system.md](type-system.md).

### Per-wiki settings

Any key from the global config that is not global-only can be overridden
here. See [global-config.md](global-config.md) for the full key
reference.


