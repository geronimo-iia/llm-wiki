---
title: "Migrating to 1.0.0"
summary: "Remove redundant stock schema copies after upgrading from pre-1.0.0."
status: ready
last_updated: "2026-08-28"
---

# Migrating to 1.0.0

## What changed

Since 1.0.0, schemas are embedded in the engine binary. When you create a wiki,
`schemas/` is an empty extension point — stock schemas are served automatically
without any on-disk copy.

Wikis created before 1.0.0 have stock schema copies in `schemas/`. They still
work — on-disk files take precedence over embedded defaults — but identical copies
are now redundant. The `migrate` command removes them so the engine can serve
current defaults automatically on future releases.

Custom schemas (files that differ from any known stock version) are never touched.

## Steps

### 1. Dry-run — preview what will be deleted

```bash
llm-wiki migrate --wiki <name> --dry-run
```

Or via MCP:

```
wiki_migrate(wiki: "<name>", dry_run: true)
```

Review the output:

- `Would delete` — identical to a known stock schema; safe to remove
- `Kept` — diverges from all known stock versions; your customization, left untouched

### 2. Check kept custom schemas

For any file in `kept_custom`, verify it has `"x-keyword": true` on fields used
for fast filtering (`type`, `tags`, `last_updated`, `aliases`). Missing that flag
causes those fields to fall back to stored-doc reads, which is slower. Patch if
needed before the live run.

### 3. Live run

```bash
llm-wiki migrate --wiki <name>
```

Or via MCP:

```
wiki_migrate(wiki: "<name>", dry_run: false)
```

To migrate all registered wikis at once:

```bash
llm-wiki migrate --all
```

### 4. Commit the deletions

```bash
git -C <wiki-path> add schemas/
git -C <wiki-path> commit -m "chore: remove redundant stock schema copies"
```

If you used the `migrate` skill in Claude Code, this commit is created
automatically after a successful live run.

### 5. Rebuild the index

Required after any schema change:

```bash
llm-wiki index rebuild --wiki <name>
```

Or via MCP:

```
wiki_index_rebuild(wiki: "<name>")
```

## After migration

- `schemas/` contains only your custom type overrides (or is empty with a `.gitkeep`)
- The engine serves embedded defaults for all stock types
- Future schema updates in new releases apply automatically — no manual copy needed

## Using the migrate skill

The `llm-wiki-skills` plugin ships a `migrate` skill that runs the full sequence
(dry-run preview, live run, git commit, index rebuild) interactively. Invoke it
from Claude Code:

```
/migrate
```

## Reference

- Tool spec: [specifications/tools/migrate.md](../specifications/tools/migrate.md)
- Design rationale: [decisions/1.0.0/schema-overlay-model.md](../decisions/1.0.0/schema-overlay-model.md)
