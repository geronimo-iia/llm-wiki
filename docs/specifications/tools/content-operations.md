---
title: "Content Operations"
summary: "read, write, new, commit."
read_when:
  - Reading or writing wiki pages
  - Creating new pages or sections
  - Committing changes to git
status: ready
last_updated: "2026-04-27"
---

# Content Operations

All content operations live under `llm-wiki content`:

| Subcommand | MCP tool | Description |
|------------|----------|-------------|
| `content read` | `wiki_content_read` | Read a page or asset by slug or `wiki://` URI |
| `content write` | `wiki_content_write` | Write a file into the wiki tree |
| `content new` | `wiki_content_new` | Create a page or section with scaffolded frontmatter |
| `content commit` | `wiki_content_commit` | Commit pending changes to git |

None of these tools validate or index — that's what `wiki_ingest` does.
Only `content commit` and `wiki_ingest` (when `auto_commit` is true)
write to git.

## content read

MCP tool: `wiki_content_read`

```
llm-wiki content read <slug|uri>
          [--no-frontmatter]        # strip frontmatter
          [--list-assets]           # list co-located assets of a bundle
          [--backlinks]             # include incoming links (pages that link to this page)
          [--format <fmt>]          # text | json (default: from config)
          [--wiki <name>]
```

Accepts a slug (`concepts/moe`), short URI (`wiki://concepts/moe`), or
full URI (`wiki://research/concepts/moe`). Also reads bundle assets
(`wiki://research/concepts/moe/diagram.png`).

When a page has `superseded_by` set, the output includes a notice
pointing to the replacement.

### Backlinks

When `--backlinks` is passed, the response is JSON instead of plain text:

```json
{
  "content": "# Page content ...",
  "backlinks": [
    { "slug": "concepts/alpha", "title": "Alpha" },
    { "slug": "concepts/beta",  "title": "Beta"  }
  ]
}
```

`backlinks` lists all pages whose body contains a `[[<target-slug>]]` wikilink.
The result is a term query on the `body_links` index field — no file writes,
no index mutation. Returns an empty array when no pages link to this page.

Without `--backlinks` (default), the response is the raw page content as plain text.

## content write

MCP tool: `wiki_content_write`

```
llm-wiki content write <slug|uri>        # read content from stdin
          [--file <source>]              # read content from a file
          [--wiki <name>]
```

Writes a file into the wiki tree. Does not validate, index, or commit.

Accepts a bare slug or `wiki://` URI. When a `wiki://` URI is used,
`--wiki` is ignored. Reads content from stdin by default, or from a
file with `--file`.

## content new

MCP tool: `wiki_content_new`

```
llm-wiki content new <slug|uri>
             [--section]            # create a section instead of a page
             [--bundle]             # bundle folder + index.md (pages only)
             [--name <title>]       # page title (default: derived from slug)
             [--type <type>]        # page type (default: page, or section with --section)
             [--dry-run]
             [--wiki <name>]
```

Creates a page by default, or a section with `--section`. Does not
commit.

Pages get scaffolded frontmatter (title derived from slug, type
defaults to `page`, status `draft`). If a body template exists at
`schemas/<type>.md`, it is appended after the frontmatter. Otherwise
the body is empty.

Template resolution order:
1. `schemas/<type>.md` in the wiki repo (owner-defined)
2. Embedded default template (shipped with engine)
3. Empty body (no template)

Sections get `type: section`.

`--bundle` creates a folder with `index.md` instead of a flat file.
Only valid for pages, not sections (sections are always directories).
`--type` is ignored with `--section` (sections are always
`type: section`).

Accepts a bare slug or `wiki://` URI. When a `wiki://` URI is used,
`--wiki` is ignored.

Missing parent sections are created automatically with their `index.md`.

## content commit

MCP tool: `wiki_content_commit`

```
llm-wiki content commit [<slug>...]      # commit specific pages
            --all                        # commit all pending changes
            [-m, --message <msg>]
            [--wiki <name>]
```

No slugs and no `--all` → error. Slugs can be bare (`concepts/moe`)
or `wiki://` URIs (`wiki://research/concepts/moe`). When a slug is a
`wiki://` URI, `--wiki` is ignored.

When committing by slug, the engine resolves what to stage:

| Slug resolves to | What gets staged |
|------------------|-----------------|
| Flat page | That single `.md` file |
| Bundle (`index.md`) | Entire bundle folder recursively |
| Section (`index.md`) | Entire section folder recursively |

Default message: `commit: <slug>, <slug>` or `commit: all`.
`--message` overrides.
