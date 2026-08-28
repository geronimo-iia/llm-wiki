---
title: "Schema Management"
summary: "wiki_schema MCP tool and llm-wiki schema CLI — list, show, add, remove, and validate type schemas."
read_when:
  - Understanding how to inspect type schemas
  - Adding or removing a custom type
  - Getting a frontmatter template for a type
  - Validating schema files before ingest
status: ready
last_updated: "2025-07-18"
---

# Schema Management

Introspect and manage the type schemas for a wiki. Available as both
a CLI command (`llm-wiki schema`) and an MCP tool (`wiki_schema`).

All operations target a specific wiki (`--wiki <name>` or the default).

```
llm-wiki schema list [--format text|json]                          List registered types
llm-wiki schema show <type> [--format text|json]                   Print JSON Schema
llm-wiki schema show <type> --template                             Print frontmatter template
llm-wiki schema add <type> <schema-path>                           Register a custom type
llm-wiki schema remove <type> [--delete] [--delete-pages] [--dry-run]  Unregister a type
llm-wiki schema validate [<type>]                                  Validate schemas + index resolution
llm-wiki migrate --wiki <name>|--all [--dry-run]                   Remove stock schema copies from existing wikis
```

## Overlay Merge Model

Built-in schemas (base, concept, doc, paper, section, skill) are embedded in
the engine binary. `spaces create` no longer copies them into `schemas/` — a
newly created wiki has an empty `schemas/` directory (`.gitkeep` only).

On every mount, `space_builder` loads embedded defaults first, then merges
any `.json` files found in `<wiki>/schemas/` on top. On-disk files are user
overrides; their absence means "use the engine default". The merge applies the
same logic as `schema validate`'s `build_space()` dry-run, so the overlay
result is always a valid tantivy schema.

To override a built-in type, create a file with the same name as the built-in
in `schemas/`. Only the fields you define take effect — the engine fills in
the rest from the embedded default.

## Operations

### list

List all registered types with descriptions.

**CLI:**
```
llm-wiki schema list [--wiki <name>] [--format text|json]
```

**MCP:**
```json
{
  "action": "list",
  "wiki": "<name>"
}
```

**Output (text):** sorted list of type entries:

```
default       Fallback for unrecognized types
article       Editorial source — blog posts, news, essays
concept       Synthesized knowledge — one concept per page
doc           Reference document — specifications, guides, standards
...
```

**Output (json):** array of `{ name, description, schema_path }`.

Sources: discovered from `schemas/*.json` via `x-wiki-types`, merged
with `wiki.toml` `[types.*]` overrides.

### show

Print the JSON Schema for a type.

**CLI:**
```
llm-wiki schema show <type> [--wiki <name>] [--format text|json]
```

**MCP:**
```json
{
  "action": "show",
  "type": "<type>",
  "wiki": "<name>"
}
```

**Output:** the full JSON Schema file content for the given type.

If the type is not registered, returns an error.

### show --template

Print a YAML frontmatter template for a type.

**CLI:**
```
llm-wiki schema show <type> --template [--wiki <name>]
```

**MCP:**
```json
{
  "action": "show",
  "type": "<type>",
  "template": true,
  "wiki": "<name>"
}
```

**Output:** a YAML frontmatter block with all required fields filled
with placeholder values and optional fields commented or omitted:

```yaml
---
title: ""
type: concept
read_when:
  - ""
summary: ""
status: active
last_updated: "2025-07-18"
tags: []
---
```

For skill type (aliased fields):

```yaml
---
name: ""
description: ""
type: skill
status: active
last_updated: "2025-07-18"
tags: []
---
```

Template generation reads `required` and `properties` from the JSON
Schema. Required fields are included with empty/default values.
Optional fields may be included as comments or omitted.

### add

Register a custom type by copying a user-provided schema file into the wiki's
`schemas/` directory and optionally adding a `[types.*]` override to
`wiki.toml`. This is for brand-new types only — to override a built-in type,
drop a file with the same name directly into `schemas/` (see overlay model
above).

**CLI:**
```
llm-wiki schema add <type> <schema-path> [--wiki <name>]
```

**MCP:**
```json
{
  "action": "add",
  "type": "<type>",
  "schema_path": "<path>",
  "wiki": "<name>"
}
```

**Behavior:**

1. Validate the schema file (valid JSON, valid JSON Schema)
2. Copy it to `<wiki>/schemas/<filename>`
3. If the schema has `x-wiki-types` declaring the type → done
   (auto-discovered on next mount via overlay merge)
4. If not → add a `[types.<type>]` entry to `wiki.toml` pointing
   to the copied schema file
5. Run `validate` on the result to confirm index resolution works

**Output:** confirmation of what was done.

### remove

Unregister a type, remove its pages from the index, and optionally
delete page files from disk.

**CLI:**
```
llm-wiki schema remove <type> [--delete] [--delete-pages] [--dry-run] [--wiki <name>]
```

**MCP:**
```json
{
  "action": "remove",
  "type": "<type>",
  "delete": true,
  "delete_pages": true,
  "dry_run": true,
  "wiki": "<name>"
}
```

**Behavior:**

1. Cannot remove the `default` type — error
2. Count pages of this type in the index
3. If `--dry-run` → report what would be done, stop
4. Remove pages of this type from the tantivy index
5. If `--delete-pages` → delete the `.md` files from disk
6. If `[types.<type>]` exists in `wiki.toml` → remove the entry
7. If `--delete` → remove the type from `x-wiki-types` in the
   schema file, or delete the schema file entirely if it declares
   only this type

**Output:**
- Pages removed from index: N
- Page files deleted from disk: N (if `--delete-pages`)
- `wiki.toml` entry removed: yes/no
- Schema file modified/deleted: yes/no (if `--delete`)

**Flags:**
- `--delete` — also modify/delete the schema file
- `--delete-pages` — also delete page `.md` files from disk
- `--dry-run` — show what would be done without doing it

### validate

Validate schema files and index resolution.

**CLI:**
```
llm-wiki schema validate [<type>] [--wiki <name>]
```

**MCP:**
```json
{
  "action": "validate",
  "type": "<type>",
  "wiki": "<name>"
}
```

**Behavior:**

- If `<type>` is given → validate that type's schema file only
- If omitted → validate all schema files in `schemas/`

**Checks:**

1. File is valid JSON
2. File is valid JSON Schema (Draft 2020-12)
3. `x-wiki-types` is present and non-empty (warning if missing)
4. Base schema invariant: `default` type requires `title` and `type`
5. `x-index-aliases` targets are valid (no cycles, targets exist
   as properties in some schema)
6. Index resolution: run `build_space()` as a dry-run — confirms
   that the full set of schemas produces a valid tantivy schema
   with no field conflicts

Check 6 is the key one — it catches problems that individual schema
validation misses:
- Two schemas classifying the same field differently (text vs keyword)
- Alias targets that don't resolve to any known field
- Missing `default` type after discovery

**Output:** ok or list of errors/warnings per schema file.

### migrate

Remove stock schema copies from wikis created before version 1.0.0, when
`spaces create` still copied built-in schemas into `schemas/`. Under the
overlay model those copies shadow the engine defaults and block automatic
schema upgrades.

**CLI:**
```
llm-wiki migrate --wiki <name> [--dry-run]
llm-wiki migrate --all [--dry-run]
```

**MCP:**
```json
{
  "action": "migrate",
  "wiki": "<name>|all",
  "dry_run": true
}
```

`--wiki <name>` or `--all` is required — no silent all-wikis default.

**Detection method:** JSON equality via `serde_json::Value` comparison.
A file is considered stock if its parsed content matches any known stock
schema version — current embedded schemas or any version in the source
archive (`schemas/archive/<version>/`). Whitespace, key ordering, and line
endings are all normalized by parsing, so the check is not sensitive to
platform differences.

**Behavior:**

1. For each wiki in scope, scan `schemas/*.json`
2. Parse each file; compare against all known stock versions
3. If `--dry-run` → report which files would be removed, stop
4. Remove files whose parsed content matches a stock version
5. Leave files with any user customization untouched (warn about each)
6. Auto-commit deleted files per wiki after a successful non-dry-run run

**Output per wiki:**
- Files removed: list of filenames
- Files skipped (customized): list of filenames with a warning
- Nothing to migrate: reported if `schemas/` has no stock copies

## MCP Tool Definition

```json
{
  "name": "wiki_schema",
  "description": "Inspect and manage type schemas",
  "parameters": {
    "action": "list | show | add | remove | validate | migrate",
    "type": "(for show/add/remove/validate) type name",
    "template": "(for show) return frontmatter template instead of schema",
    "schema_path": "(for add) path to schema file to copy",
    "delete": "(for remove) also delete/modify the schema file",
    "delete_pages": "(for remove) also delete page files from disk",
    "dry_run": "(for remove/migrate) show what would be done without doing it",
    "wiki": "target wiki name or 'all' (for migrate); uses default if omitted"
  }
}
```

## Relationship to Other Tools

- `wiki_config list` returns wiki identity and settings — not types.
  `wiki_schema list` returns the type registry.
- `wiki_content_new` scaffolds a page with minimal frontmatter.
  `wiki_schema show --template` returns the full type-specific
  template without creating a file.
- `wiki_ingest` validates against the schema. `wiki_schema show`
  lets you inspect what it validates against.
- `wiki_index_rebuild` rebuilds the full index.
  `wiki_schema remove` removes only pages of a specific type.
