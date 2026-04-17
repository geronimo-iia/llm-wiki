---
title: "Content Operations"
summary: "read, write, new page, new section, commit."
read_when:
  - Reading or writing wiki pages
  - Creating new pages or sections
  - Committing changes to git
status: ready
last_updated: "2025-07-17"
---

# Content Operations

| Command       | MCP tool           | Description                                   |
| ------------- | ------------------ | --------------------------------------------- |
| `read`        | `wiki_read`        | Read a page or asset by slug or `wiki://` URI |
| `write`       | `wiki_write`       | Write a file into the wiki tree               |
| `new page`    | `wiki_new_page`    | Create a page with scaffolded frontmatter     |
| `new section` | `wiki_new_section` | Create a section with `index.md`              |
| `commit`      | `wiki_commit`      | Commit pending changes to git                 |

None of these tools validate or index — that's what `wiki_ingest` does.
Only `wiki_commit` and `wiki_ingest` (when `auto_commit` is true) write
to git.

## read

MCP tool: `wiki_read`

```
llm-wiki read <slug|uri>
          [--no-frontmatter]        # strip frontmatter
          [--list-assets]           # list co-located assets of a bundle
          [--wiki <name>]
```

Accepts a slug (`concepts/moe`), short URI (`wiki://concepts/moe`), or
full URI (`wiki://research/concepts/moe`). Also reads bundle assets
(`wiki://research/concepts/moe/diagram.png`).

When a page has `superseded_by` set, the output includes a notice
pointing to the replacement.

## write

MCP tool: `wiki_write`

```
llm-wiki write <slug|uri>                # read content from stdin
          [--file <source>]             # read content from a file
          [--wiki <name>]
```

Writes a file into the wiki tree. Does not validate, index, or commit.

Accepts a bare slug or `wiki://` URI. When a `wiki://` URI is used,
`--wiki` is ignored. Reads content from stdin by default, or from a
file with `--file`.

## new page

MCP tool: `wiki_new_page`

```
llm-wiki new page <slug|uri>
             [--bundle]             # bundle folder + index.md
             [--dry-run]
             [--wiki <name>]
```

Creates a page with scaffolded frontmatter (title derived from slug,
type defaults to `page`, status `draft`). Does not commit.

Accepts a bare slug or `wiki://` URI. When a `wiki://` URI is used,
`--wiki` is ignored.

Missing parent sections are created automatically with their `index.md`.

## new section

MCP tool: `wiki_new_section`

```
llm-wiki new section <slug|uri>
                [--dry-run]
                [--wiki <name>]
```

Creates a section directory with `index.md` (`type: section`). Does not
commit.

Accepts a bare slug or `wiki://` URI. When a `wiki://` URI is used,
`--wiki` is ignored.

Missing parent sections are created automatically with their `index.md`.

## commit

MCP tool: `wiki_commit`

```
llm-wiki commit [<slug>...]             # commit specific pages
            --all                       # commit all pending changes
            [-m, --message <msg>]
            [--wiki <name>]
```

No slugs and no `--all` → error. Slugs can be bare (`concepts/moe`)
or `wiki://` URIs (`wiki://research/concepts/moe`). When a slug is a
`wiki://` URI, `--wiki` is ignored.

When committing by slug, the engine resolves what to stage:

| Slug resolves to     | What gets staged                  |
| -------------------- | --------------------------------- |
| Flat page            | That single `.md` file            |
| Bundle (`index.md`)  | Entire bundle folder recursively  |
| Section (`index.md`) | Entire section folder recursively |

Default message: `commit: <slug>, <slug>` or `commit: all`.
`--message` overrides.
