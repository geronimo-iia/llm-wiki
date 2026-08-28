# Custom Types

How to add a custom page type to your wiki. The engine validates
frontmatter against the type's JSON Schema on ingest, indexes fields
according to the schema, and includes declared edges in the concept
graph.

## Overlay Model

Built-in schemas (base, concept, doc, paper, section, skill) are embedded in
the engine binary. `spaces create` does not copy them into `schemas/` — that
directory starts empty. On every mount, `space_builder` merges the embedded
defaults with any `.json` files found in `schemas/`. On-disk files overlay the
defaults; absent files mean "use the engine default".

To override a built-in type, create a file with the same name as the built-in
schema in your wiki's `schemas/` directory. Only the fields you define override
the defaults.

To add a brand-new type (no built-in default to override), follow the workflow
below.

## Quick Start

```
create schema → register → write page → ingest → search/list/graph
     │              │            │          │           │
 meeting.json    schema add    .md file   validate    --type meeting
                              frontmatter  index
                                           commit
```

1. Create a schema file
2. Register it with `llm-wiki schema add`
3. Pages with that type are validated and indexed automatically

## Example: Meeting Notes

### 1. Create the schema

Create `schemas/meeting.json`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Meeting notes",
  "type": "object",
  "required": ["title", "type"],
  "properties": {
    "title": {
      "type": "string",
      "description": "Meeting title"
    },
    "type": {
      "type": "string"
    },
    "date": {
      "type": "string",
      "description": "ISO 8601 date"
    },
    "attendees": {
      "type": "array",
      "items": { "type": "string" },
      "description": "List of attendees"
    },
    "action_items": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Action items from the meeting"
    },
    "status": {
      "type": "string",
      "enum": ["active", "draft", "stub", "generated"]
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "x-keyword": true
    },
    "concepts": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Slugs of concept pages discussed"
    }
  },
  "x-wiki-types": {
    "meeting": "Meeting notes with attendees and action items"
  },
  "x-graph-edges": {
    "concepts": {
      "relation": "discussed-in",
      "direction": "outgoing",
      "target_types": ["concept"]
    }
  },
  "additionalProperties": true
}
```

Key parts:
- `x-wiki-types` declares the type name — the engine discovers it
  automatically by scanning `schemas/*.json`
- `x-graph-edges` declares outgoing edges — meeting pages link to
  concept pages with a `discussed-in` relation
- `required` must include at least `title` and `type`
- `additionalProperties: true` allows extra fields without validation
  errors

### 2. Register the schema

```bash
llm-wiki schema add meeting schemas/meeting.json
```

This copies your schema file into the wiki's `schemas/` directory and
validates index resolution. If the schema has `x-wiki-types`, the
type is discovered automatically on the next mount.

Verify:

```bash
llm-wiki schema list
llm-wiki schema validate meeting
```

### 3. Write a page

```markdown
---
title: "Sprint Planning 2025-07-21"
type: meeting
date: "2025-07-21"
attendees:
  - Alice
  - Bob
action_items:
  - "Review MoE scaling results"
  - "Update wiki with new findings"
concepts:
  - concepts/mixture-of-experts
status: active
tags: [sprint, planning]
---

## Notes

Discussed MoE scaling results from the latest paper...
```

### 4. Ingest

```bash
llm-wiki ingest wiki/meetings/sprint-2025-07-21.md
```

The engine validates against `meeting.json`, indexes all fields, and
commits.

### 5. Search and list

```bash
llm-wiki search "sprint planning" --type meeting
llm-wiki list --type meeting
```

## Field Aliasing

If your type uses different field names for the same role as the base
schema, declare aliases with `x-index-aliases`:

```json
{
  "x-index-aliases": {
    "subject": "title",
    "notes": "summary"
  }
}
```

The index sees `title` and `summary` regardless of what the frontmatter
calls them. Search works uniformly across types.

## Graph Edges

Declare outgoing edges with `x-graph-edges`:

```json
{
  "x-graph-edges": {
    "concepts": {
      "relation": "discussed-in",
      "direction": "outgoing",
      "target_types": ["concept"]
    }
  }
}
```

This creates `discussed-in` edges from meeting pages to concept pages
in the graph. `wiki_graph --relation discussed-in` filters to those
edges.

Fields declared in `x-graph-edges` are automatically indexed as
keywords (slug lists).

To index a plain-string array as one keyword value per entry (instead
of joining all values into one text string), add `"x-keyword": true`
to the field definition. Values are lowercased at index time. Use this
for tag-like fields where each value is a discrete term, not prose:

```json
"tags": {
  "type": "array",
  "items": { "type": "string" },
  "x-keyword": true
}
```

## Override via wiki.toml

Normally, dropping a schema into `schemas/` is enough. Use `wiki.toml`
only to remap a type to a different schema file:

```toml
[types.meeting]
schema = "schemas/my-custom-meeting.json"
description = "Custom meeting schema"
```

## Validate

Check that your schema is valid and the index resolves correctly:

```bash
llm-wiki schema validate meeting
```

## Inspect

```bash
# List all registered types
llm-wiki schema list

# Show the JSON Schema for a type
llm-wiki schema show meeting

# Get a frontmatter template
llm-wiki schema show meeting --template
```

## Body Template

Add a body template at `schemas/meeting.md` to scaffold page structure
when creating pages with `wiki_content_new`:

```markdown
## Attendees



## Agenda



## Action Items

```

The template is plain Markdown (no frontmatter). The engine prepends
the scaffolded frontmatter automatically.

## Override a Built-in Type

To change the behavior of a built-in type (e.g., add a required field to
`concept`), create a file with the same name in `schemas/`:

```bash
# Show the current built-in schema
llm-wiki schema show concept --format json > schemas/concept.json

# Edit schemas/concept.json — add your fields, constraints, or edges
# The engine merges your file on top of the embedded default on next mount
```

Only the fields and extensions you define override the defaults. You do not
need to reproduce the entire schema — but in practice copying the full output
of `schema show` and editing it avoids surprises.

## Migrate Existing Wikis

Wikis created before version 1.0.0 may have stock schema copies in `schemas/`
left over from the old copy-on-create model. Use `wiki migrate` to remove them:

```bash
llm-wiki migrate --wiki <name>    # migrate a specific wiki
llm-wiki migrate --all            # migrate all registered wikis
llm-wiki migrate --wiki <name> --dry-run  # preview what would be removed
```

`wiki migrate` detects stock copies by comparing on-disk content to a JSON
equality archive of every known historical schema version (current and
archived releases). Files whose parsed content matches any known stock version
are removed. Genuine user customizations (any edit, field addition, or
structural change) are left untouched. After a successful non-dry-run
migration, deleted files are auto-committed per wiki.

## Reference

- [Type system spec](../specifications/model/type-system.md)
- [Frontmatter spec](../specifications/model/page-content.md)
- [Schema management tool](../specifications/tools/schema-management.md)
