---
title: "Content Operations"
summary: "read, write, new, commit, resolve."
read_when:
  - Reading or writing wiki pages
  - Creating new pages or sections
  - Committing changes to git
  - Resolving a slug to its local filesystem path
status: ready
last_updated: "2026-04-28"
---

# Content Operations

All content operations live under `llm-wiki content`:

| Subcommand | MCP tool | Description |
|------------|----------|-------------|
| `content read` | `wiki_content_read` | Read a page or asset by slug or `wiki://` URI |
| `content write` | `wiki_content_write` | Write a file into the wiki tree |
| `content new` | `wiki_content_new` | Create a page or section with scaffolded frontmatter |
| `content commit` | `wiki_content_commit` | Commit pending changes to git |
| — | `wiki_resolve` | Resolve a slug or URI to its local filesystem path |

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

The MCP response is JSON (not plain text):

```json
{
  "uri":       "wiki://research/concepts/new-page",
  "slug":      "concepts/new-page",
  "path":      "/path/to/wiki/concepts/new-page.md",
  "wiki_root": "/path/to/wiki",
  "bundle":    false
}
```

`path` is the resolved filesystem path of the created file. Use it to write
content directly with native file tools before calling `wiki_ingest`.

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

## wiki_resolve

MCP tool only (no CLI equivalent).

```json
wiki_resolve(uri: "concepts/mixture-of-experts", wiki?: "research")
→ {
    "slug":      "concepts/mixture-of-experts",
    "wiki":      "research",
    "wiki_root": "/path/to/wiki",
    "path":      "/path/to/wiki/concepts/mixture-of-experts.md",
    "exists":    true,
    "bundle":    false
  }
```

Resolves a slug or `wiki://` URI to its local filesystem path without reading
the file. Use before writing content directly to disk.

| Field | Description |
|-------|-------------|
| `slug` | Canonical slug (no extension) |
| `wiki` | Wiki name |
| `wiki_root` | Absolute path to the wiki directory |
| `path` | Absolute path to the file (flat `.md` or `index.md` for bundles) |
| `exists` | `true` if the file exists on disk |
| `bundle` | `true` if the page is a bundle (`path` ends with `index.md`) |

For a not-yet-existing slug, `exists` is `false` and `path` is the would-be
flat path (`<wiki_root>/<slug>.md`). The direct write pattern:

```
1. wiki_resolve(uri)                      → get local path + wiki_root
2. Write / Edit file at path directly     → no MCP content round-trip
3. wiki_ingest(path: "<slug>", dry_run: true)  → validate frontmatter
4. wiki_ingest(path: "<slug>")            → commit
```

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
