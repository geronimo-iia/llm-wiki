---
title: "Writing Content"
summary: "How to create and update wiki pages: direct write pattern, wiki_resolve, wiki_content_write, and backlinks."
status: active
last_updated: "2026-04-28"
---

# Writing Content

There are two main patterns for writing wiki content: the **direct write
pattern** (recommended for new pages) and the **read-modify-write pattern**
(for updates to existing pages).

## Creating a new page

The recommended flow for creating a page avoids an extra MCP round-trip by
writing the file directly to disk using the path returned by
`wiki_content_new`.

```
1. wiki_content_new(uri: "concepts/my-topic", name: "My Topic")
   → returns { uri, slug, path, wiki_root, bundle }

2. Write content directly to `path`
   (Bash write, Edit, or any file write tool)

3. wiki_ingest(path: "concepts/my-topic.md", wiki: "research")
   → validates frontmatter, updates index, commits
```

Why not `wiki_content_write`? `wiki_content_new` scaffolds the frontmatter
and returns the exact filesystem path. Writing directly to that path and then
calling `wiki_ingest` keeps the page canonical without an extra MCP read
round-trip to confirm the path.

### `wiki_content_new` response

```json
{
  "uri": "wiki://research/concepts/my-topic",
  "slug": "concepts/my-topic",
  "path": "/home/user/wikis/research/wiki/concepts/my-topic.md",
  "wiki_root": "/home/user/wikis/research/wiki",
  "bundle": false
}
```

For a bundle page (`bundle: true`), `path` points to the `index.md` inside
the created directory.

## Resolving an existing page path

When you want to write directly to an existing page without reading its
content first, use `wiki_resolve`:

```
wiki_resolve(uri: "concepts/scaling-laws", wiki: "research")
→ {
    "slug": "concepts/scaling-laws",
    "wiki": "research",
    "wiki_root": "/home/user/wikis/research/wiki",
    "path": "/home/user/wikis/research/wiki/concepts/scaling-laws.md",
    "exists": true,
    "bundle": false
  }
```

Then write directly to `path` and call `wiki_ingest` to re-validate and
re-index.

`wiki_resolve` also works for non-existent slugs — `exists: false` — so you
can check whether a page exists before creating it.

## Updating an existing page

For updates where you need to read the current content first:

```
1. wiki_content_read(uri: "concepts/scaling-laws")
   → current content as string

2. Modify content in session

3. wiki_content_write(uri: "concepts/scaling-laws", content: "...")
   → writes to wiki tree

4. wiki_ingest(path: "concepts/scaling-laws.md")
   → validates, indexes, commits
```

`wiki_content_write` resolves the slug to a path internally — it's equivalent
to resolving and writing directly, but returns a minimal confirmation rather
than path information.

## Using wiki_content_write

`wiki_content_write` is the simplest write tool: it takes a slug/URI and
content, writes the file, and returns a confirmation. Use it when you already
have the full replacement content in session and don't need the path for
further operations.

```json
// Parameters
{
  "uri": "concepts/scaling-laws",
  "content": "---\ntype: concept\n...\n---\n\n# Scaling Laws\n...",
  "wiki": "research"   // optional, uses default
}
```

The response is a plain confirmation string. For path information, prefer
`wiki_content_new` (new pages) or `wiki_resolve` (existing pages).

## Creating sections

Sections are structural pages (folders in the wiki tree with their own
`index.md`). Create them with `wiki_content_new(section: true)`:

```
wiki_content_new(uri: "concepts/transformers", section: true)
```

The section page is scaffolded at `concepts/transformers/index.md`. Child
pages live at `concepts/transformers/<slug>.md`.

## Backlinks

`wiki_content_read` can return incoming links — all pages that reference the
target slug in their body or frontmatter edge fields:

```
wiki_content_read(uri: "concepts/scaling-laws", backlinks: true)
→ {
    "content": "...",
    "backlinks": [
      { "slug": "sources/chinchilla-2022", "title": "Chinchilla" },
      { "slug": "concepts/mixture-of-experts", "title": "Mixture of Experts" }
    ]
  }
```

Backlinks are useful for understanding what depends on a page before updating
or deleting it.

## Linking to pages

Two syntaxes create graph edges from a page body:

| Syntax | When to use |
|--------|-------------|
| `[[slug]]` | Wiki-native shorthand; parsed by the engine only |
| `[text](slug)` | Standard Markdown; rendered by Hugo CMS and indexed by the engine |
| `[text](wiki://name/slug)` | Cross-wiki reference; rendered by Hugo CMS as a local link, indexed as a `CrossWiki` edge |

All three create entries in `body_links`, appear in `wiki_graph`, and are
checked by the `broken-link` lint rule.

**Choosing a syntax** — `[text](wiki://name/slug)` is the portable format:
it is indexed by the engine, rendered correctly by Hugo CMS, and readable
as plain Markdown in any editor. Use `[[slug]]` when you want the shortest
possible inline reference in a wiki-only context.

External URLs (`https://…`), anchors (`#section`), and image links
(`![alt](path)`) are not indexed.

## Choosing the right tool

| Goal | Tool |
|------|------|
| Create a new page with scaffolded frontmatter | `wiki_content_new` → write to `path` → `wiki_ingest` |
| Get the path of an existing page without reading content | `wiki_resolve` |
| Update an existing page (need to read first) | `wiki_content_read` → modify → `wiki_content_write` |
| Simple overwrite of known content | `wiki_content_write` |
| Check what links to a page | `wiki_content_read(backlinks: true)` |
| Commit staged changes | `wiki_content_commit` |

## After writing

Always call `wiki_ingest` after writing content directly to disk (via `path`).
If you used `wiki_content_write`, the file is written but not yet indexed or
committed — call `wiki_ingest` to complete the pipeline.

```
wiki_ingest(path: "concepts/my-topic.md", wiki: "research")
```

Run `wiki_lint(rules: "broken-link,orphan")` after creating multiple pages in
a session to catch dead references introduced by new content.
