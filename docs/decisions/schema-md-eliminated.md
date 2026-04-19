# schema.md Eliminated

## Decision

`schema.md` is removed. The type registry moves to `wiki.toml`
`[types.*]`. Wiki conventions move to skills.

## Context

`schema.md` was doing two jobs:

1. Type registry — type names, descriptions, validation rules
2. Wiki conventions — category structure, ingest rules, linking policy

The engine had to parse a Markdown file to get structured data. The
LLM had to read it at every session start for conventions.

## Rationale

The type registry is structured data. It belongs in the engine's config
file (`wiki.toml`), not in a Markdown file the engine has to parse.

Wiki conventions (how to organize pages, when to link, how to write
frontmatter) are authoring guidance. They belong in skills that teach
the LLM how to work, not in engine configuration.

## Where Content Moved

| Content            | New home                                    |
| ------------------ | ------------------------------------------- |
| Type registry      | `wiki.toml` `[types.*]`                     |
| Type descriptions  | `wiki.toml` `[types.*]` `description` field |
| Category structure | `README.md` or wiki owner's choice          |
| Ingest conventions | `ingest` skill in `llm-wiki-skills`         |
| Linking policy     | `frontmatter` skill in `llm-wiki-skills`    |
| Domain patterns    | `README.md` or wiki-specific skills         |

## Consequences

- `wiki.toml` is the single source of truth for engine configuration
- `wiki_config list` returns type names + descriptions
- No Markdown-as-config parsing
- LLM reads conventions from skills, not from a file the engine manages

> **Note:** The "type registry in `wiki.toml`" part of this decision
> has been superseded by
> [schema-driven-types](schema-driven-types.md) — types are now
> discovered from `schemas/*.json` via `x-wiki-types`, with
> `wiki.toml` `[types.*]` as optional overrides.
