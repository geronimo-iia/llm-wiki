---
title: "Ingest Pipeline"
summary: "Validate, alias, index, commit — how content enters the wiki."
read_when:
  - Understanding how content enters the wiki
  - Understanding the validation and indexing flow
status: ready
last_updated: "2025-07-17"
---

# Ingest Pipeline

`wiki_ingest` validates files already in the wiki tree, updates the
search index, and optionally commits to git. It is the only write path
into the tantivy index.

For the CLI and MCP tool, see [ingest.md](../tools/ingest.md).

## Page Discovery

The engine walks `wiki/` recursively. No exclusions needed — `raw/` and
`inbox/` are outside the wiki root.

- `.md` file named `index.md` → page at slug = parent directory path
- Any other `.md` file → page at slug = path without extension
- Non-`.md` file inside a bundle folder → asset of that page

## Pipeline Steps

```
1. Parse YAML frontmatter
2. Read `type` field (default: "page" if missing)
3. Look up type in discovered registry (schemas/*.json x-wiki-types,
   overridden by wiki.toml [types.<type>] if present)
4. Fall back to [types.default] / base.json if type not found
5. Load JSON Schema from schemas/
6. Validate frontmatter against schema → reject if invalid
7. Read x-index-aliases from schema → apply aliases
8. Read x-graph-edges from schema → index edges with relation labels
9. Index all canonical fields in tantivy
10. Index unrecognized string fields as body text
11. Store original frontmatter as-is (no rewriting)
12. Commit to git (if auto_commit)
```

For aliasing and graph edges, see
[type-system.md](../model/type-system.md). For the index schema, see
[index-management.md](index-management.md).

## Validation

| Check                        | On failure                                                                                  |
| ---------------------------- | ------------------------------------------------------------------------------------------- |
| Valid YAML frontmatter block | Error — file rejected                                                                       |
| `title` present (or aliased) | Error — file rejected                                                                       |
| `type` recognized            | Depends on `validation.type_strictness` (see [global-config.md](../model/global-config.md)) |
| JSON Schema validation       | Depends on `validation.type_strictness`                                                     |
| No path traversal (`../`)    | Error — file rejected                                                                       |

### Index defaults

When a field is missing from frontmatter, the engine applies defaults
at index time only. The file on disk is never modified.

| Field          | Default        |
| -------------- | -------------- |
| `status`       | `active`       |
| `type`         | `page`         |
| `last_updated` | Today's date   |

### What the engine never touches

The file on disk. The engine does not merge, rewrite, or reformat
frontmatter or body. What the author wrote is what stays on disk.

## Commit Behavior

When `ingest.auto_commit` is `true` (default), ingest produces a git
commit. When `false`, no commit — the user reviews and commits via
`wiki_content_commit`.

The tantivy index is always updated after validation, regardless of
`auto_commit`.
