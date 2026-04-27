---
title: "Base Type"
summary: "The base schema — shared fields for all page types, and the default fallback."
read_when:
  - Understanding the base frontmatter fields
  - Understanding what happens with unrecognized types
status: ready
last_updated: "2025-07-17"
---

# Base Type

Schema: `schemas/base.json`

The base schema defines the minimal fields shared by all page types.
It is also the fallback for pages with an unrecognized or missing `type`
field (registered as `[types.default]` in `wiki.toml`).

## Required Fields

| Field   | Type   | Description             |
| ------- | ------ | ----------------------- |
| `title` | string | Display name            |
| `type`  | string | Page type from registry |

## Optional Fields

| Field           | Type         | Description                                      |
| --------------- | ------------ | ------------------------------------------------ |
| `summary`       | string       | One-line scope                                   |
| `status`        | string       | Lifecycle state (e.g. `active`, `draft`, `stub`) |
| `last_updated`  | string       | ISO 8601 date                                    |
| `tags`          | list[string] | Lowercase hyphenated search terms                |
| `owner`         | string       | Person, team, or agent responsible               |
| `superseded_by` | string       | Slug of replacement page                         |
| `confidence`    | float 0.0–1.0 | Certainty of this page's content. Default `0.5`. `0.0–0.3` = speculative, `0.4–0.6` = partial, `0.7–0.9` = reviewed, `1.0` = verified. Used as a search ranking multiplier. |

Uses `additionalProperties: true` — unrecognized fields are preserved
on disk and indexed as text.

## Aliasing

The base schema has no `x-index-aliases`. Type schemas that use
different field names (like `skill.json` with `name`/`description`)
define their own aliases.

