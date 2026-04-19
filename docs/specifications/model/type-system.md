---
title: "Type System"
summary: "What a type is, how types are discovered from schemas, field aliasing, and graph edges."
read_when:
  - Understanding how per-type validation works
  - Adding a custom type to a wiki
  - Understanding field aliasing
  - Understanding typed graph edges
  - Understanding type discovery from schemas
status: ready
last_updated: "2025-07-18"
---

# Type System

Every page has a `type` field. The type determines which JSON Schema
validates the frontmatter and how fields are indexed.

Types are discovered automatically from `schemas/*.json` files in the
wiki repository. Each schema declares which types it serves via
`x-wiki-types`. Optional `[types.*]` entries in `wiki.toml` override
the discovered mapping.

For the epistemic rationale behind types, see
[epistemic-model.md](epistemic-model.md).

## Built-in Types

Shipped as default schema files by `llm-wiki spaces create`:

| Type                                      | Schema         | Description                                                  |
| ----------------------------------------- | -------------- | ------------------------------------------------------------ |
| [base](types/base.md)                     | `base.json`    | Default fallback for unrecognized types                      |
| [concept, query-result](types/concept.md) | `concept.json` | Synthesized knowledge and saved conclusions                  |
| [source types](types/source.md)           | `paper.json`   | What each source claims (paper, article, documentation, ...) |
| [skill](types/skill.md)                   | `skill.json`   | Agent skill with workflow instructions                       |
| [doc](types/doc.md)                       | `doc.json`     | Reference document with agent-foundation frontmatter         |
| [section](types/section.md)               | `section.json` | Section index grouping related pages                         |

## How It Works

1. The engine scans `schemas/*.json` in the wiki repository
2. For each schema, it reads `x-wiki-types` to discover which page
   types the schema serves
3. If `wiki.toml` has a `[types.<name>]` entry for a type, that
   override takes precedence over the discovered mapping
4. On `wiki_ingest`, the engine validates frontmatter against the
   type’s JSON Schema
5. Field aliases map type-specific names to canonical index roles

## Type Discovery — `x-wiki-types`

Each JSON Schema declares which page types it serves:

```json
"x-wiki-types": {
  "paper": "Academic source — research papers, preprints",
  "article": "Editorial source — blog posts, news, essays",
  "documentation": "Reference source — product docs, API references"
}
```

- **Key** = type name (used in frontmatter `type` field)
- **Value** = human-readable description

One schema can serve multiple types (e.g., `paper.json` serves all 9
source types). The engine iterates all `schemas/*.json` files, collects
`x-wiki-types` entries, and builds the type registry.

The type named `default` (from `base.json`) is the fallback for pages
with an unrecognized or missing `type` field.

### Resolution order

1. Scan `schemas/*.json` → collect all `x-wiki-types` entries
2. Read `[types.*]` from `wiki.toml` (if any)
3. For each type: `wiki.toml` entry wins over discovered entry
4. Result = merged type registry

This means:
- **Common case**: no `[types.*]` in `wiki.toml` — types are fully
  discovered from schema files. `wiki.toml` stays clean.
- **Override case**: a `[types.*]` entry remaps a type to a different
  schema (e.g., `paper` → `schemas/my-paper.json`).
- **Custom type**: drop a schema with `x-wiki-types` into `schemas/`
  — the engine discovers it automatically. Or add a `[types.*]` entry
  in `wiki.toml` pointing to any schema file.

## Field Aliasing — `x-index-aliases`

Different types use different field names for the same role. The engine
maps them to canonical index fields via `x-index-aliases` in the JSON
Schema:

```json
"x-index-aliases": {
  "name": "title",
  "description": "summary",
  "when_to_use": "read_when"
}
```

- **Key** = source field name in frontmatter (e.g., `name`)
- **Value** = canonical index field (e.g., `title`)
- At ingest: if source field exists and canonical field does not, index
  source value under the canonical name
- If both exist, the canonical field wins
- Aliases affect indexing only — the file on disk is never rewritten

Fields not aliased to a canonical field are indexed as generic text.
For the full list of canonical index fields, see
[index-management.md](../engine/index-management.md).

## Typed Graph Edges — `x-graph-edges`

Each type schema declares its outgoing edges:

```json
"x-graph-edges": {
  "sources": {
    "relation": "fed-by",
    "direction": "outgoing",
    "target_types": ["paper", "article", "documentation"]
  },
  "concepts": {
    "relation": "depends-on",
    "direction": "outgoing",
    "target_types": ["concept"]
  }
}
```

| Field          | Required | Description                               |
| -------------- | -------- | ----------------------------------------- |
| `relation`     | Yes      | Edge label (e.g., `fed-by`, `depends-on`) |
| `direction`    | Yes      | `outgoing` (this page → target)           |
| `target_types` | No       | Valid target types. Omitted = any type.   |

Body `[[wiki-links]]` get a generic `links-to` relation.

See [graph.md](../engine/graph.md) for how the engine builds and renders
the graph.

> **Note:** Typed graph edges are subject to change (Phase 3).

## Custom Types

Drop a schema file into `schemas/` with `x-wiki-types` — the engine
discovers it automatically:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "x-wiki-types": {
    "meeting-notes": "Meeting notes with attendees and action items"
  },
  "type": "object",
  "required": ["title", "type"],
  "properties": {
    "title": { "type": "string" },
    "type": { "type": "string" },
    "attendees": { "type": "array", "items": { "type": "string" } },
    "action_items": { "type": "array", "items": { "type": "string" } }
  },
  "additionalProperties": true
}
```

Alternatively, add a `[types.*]` entry in `wiki.toml` pointing to any
schema file:

```toml
[types.meeting-notes]
schema = "schemas/meeting-notes.json"
description = "Meeting notes with attendees and action items"
```

The engine doesn’t need to know what “meeting-notes” means. It validates
against the schema and indexes using the alias mapping.

## Backward Compatibility

- Pages without a `type` field default to `type: page`, validated
  against `[types.default]`
- Pages with an unregistered type are validated against `[types.default]`
- Wikis with no `schemas/` directory use a built-in base schema
- No frontmatter rewriting — existing files are untouched
