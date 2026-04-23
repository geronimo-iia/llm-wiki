# Custom Types

How to add a custom page type to your wiki. The engine validates
frontmatter against the type's JSON Schema on ingest, indexes fields
according to the schema, and includes declared edges in the concept
graph.

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
      "items": { "type": "string" }
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

This copies the schema into the wiki's `schemas/` directory and
validates index resolution. If the schema has `x-wiki-types`, the
type is discovered automatically.

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

## Reference

- [Type system spec](../specifications/model/type-system.md)
- [Frontmatter spec](../specifications/model/page-content.md)
- [Schema management tool](../specifications/tools/schema-management.md)
