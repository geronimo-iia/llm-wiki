---
title: "Migrate"
summary: "Remove redundant stock schema copies from wiki directories."
read_when:
  - Using the overlay schema model
  - Cleaning up stock schemas after upgrading from pre-1.0.0
  - Using wiki_migrate or wiki migrate CLI
status: ready
last_updated: "2026-08-22"
---

# Migrate

| Command | MCP tool | Description |
|---------|----------|-------------|
| `migrate` | `wiki_migrate` | Remove stock schema copies from `schemas/` |

## migrate

MCP tool: `wiki_migrate`

```
llm-wiki migrate
              [--wiki <name>]           # target one wiki (required unless --all)
              [--all]                   # run across every registered wiki
              [--dry-run]               # preview without modifying anything
              [--format <fmt>]          # text | json (default: from config)
```

Since 1.0.0 the engine uses an overlay model: embedded defaults are
always present, and on-disk `schemas/` files only extend or replace them.
Stock schema copies in `schemas/` are therefore redundant. `migrate`
removes them, leaving any custom schemas untouched.

**Stock detection** uses JSON value equality (whitespace/key-order
independent) against the current embedded schemas and the pre-1.0.0
archive. A file that differs from every known stock version is kept.

**`--all`** and **`--wiki`** are mutually exclusive; one is required.

### Example — dry run

```
$ llm-wiki migrate --wiki my-wiki --dry-run
Wiki: my-wiki
  Would delete: schemas/base.json (stock)
  Would delete: schemas/concept.json (stock)
  Kept:         schemas/mytype.json (custom)
  already_clean: false
```

### Example — live run

```
$ llm-wiki migrate --wiki my-wiki
Wiki: my-wiki
  Deleted: schemas/base.json
  Deleted: schemas/concept.json
  Kept:    schemas/mytype.json
```

### JSON output

```json
{
  "wikis": [
    {
      "name": "my-wiki",
      "deleted": ["schemas/base.json", "schemas/concept.json"],
      "kept_custom": ["schemas/mytype.json"],
      "already_clean": false
    }
  ]
}
```

Fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Wiki name |
| `deleted` | string[] | Relative paths of deleted (or would-delete) stock schemas |
| `kept_custom` | string[] | Relative paths of custom schemas left untouched |
| `already_clean` | bool | `true` when `deleted` is empty (nothing to do) |

### MCP parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `wiki` | string | no | Target wiki name |
| `dry_run` | boolean | no | Preview only, no deletions |
