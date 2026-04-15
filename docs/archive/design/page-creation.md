---
title: "Page Creation"
summary: "How to create new wiki pages and sections — directly via the filesystem or via wiki new for scaffolded frontmatter."
read_when:
  - Adding a new page, bundle, or section to the wiki
  - Implementing the wiki new subcommand
  - Understanding the difference between page creation and ingest
status: draft
last_updated: "2025-07-15"
---

# Page Creation

Creating a page or section is a filesystem operation. The wiki discovers pages
via `walkdir` — any `.md` file under the wiki root is a page. No pipeline required.

---

## 1. Two Primitives

### Page

A Markdown file with frontmatter. Either a flat file or a bundle (page + assets).

```bash
wiki new page concepts/mixture-of-experts          # flat page
wiki new page concepts/mixture-of-experts --bundle # bundle folder + index.md
```

Generates a minimal frontmatter scaffold and commits:

```yaml
---
title: "Mixture of Experts"
summary: ""
status: draft
last_updated: "2025-07-15"
type: concept
tags: []
read_when: []
---
```

### Section

A directory that groups related pages, always with an `index.md` describing it.

```bash
wiki new section skills
```

Creates `skills/index.md` with frontmatter:

```yaml
---
title: "Skills"
summary: ""
status: draft
last_updated: "2025-07-15"
type: section
---
```

The `index.md` makes the section discoverable by `wiki context` and the tantivy
index. A section without an `index.md` is just an opaque folder.

---

## 2. Auto-creating Parent Sections

If a parent section does not exist when creating a page, it is created
automatically with its `index.md`:

```bash
wiki new page a/b/c
# a/ does not exist → create a/index.md
# a/b/ does not exist → create a/b/index.md
# create a/b/c.md
# single git commit: new: a/b/c
```

All created files are included in the same commit.

---

## 3. Flat Page vs Bundle

| | Flat page | Bundle |
|---|---|---|
| Form | `{slug}.md` | `{slug}/index.md` |
| Assets | None | Co-located beside `index.md` |
| When to use | Text-only content | Page has diagrams, configs, scripts |

A flat page can be promoted to a bundle later — `wiki ingest` handles the
`{slug}.md` → `{slug}/index.md` promotion automatically when the first asset
is co-located. See [asset-ingest.md](asset-ingest.md).

---

## 4. CLI Interface

```
wiki new page <slug>        # flat page with minimal frontmatter
             [--bundle]     # bundle folder + index.md instead
             [--wiki <name>]
             [--dry-run]

wiki new section <slug>     # directory + index.md with frontmatter
                [--wiki <name>]
                [--dry-run]
```

Errors:
- Slug already exists → error, no overwrite

Git commit: `new: <slug>`

---

## 5. MCP Tools

```rust
#[tool(description = "Create a new empty wiki page with minimal frontmatter")]
async fn wiki_new_page(
    &self,
    #[tool(param)] slug: String,
    #[tool(param)] bundle: Option<bool>,
    #[tool(param)] wiki: Option<String>,
) -> String { ... }  // returns absolute path of created file

#[tool(description = "Create a new wiki section with an index page")]
async fn wiki_new_section(
    &self,
    #[tool(param)] slug: String,
    #[tool(param)] wiki: Option<String>,
) -> String { ... }  // returns absolute path of index.md
```

The LLM calls `wiki_new_page` to scaffold, writes content to the returned path,
then calls `wiki_ingest` to commit.

---

## 6. Relationship to Ingest

`wiki new` and `wiki ingest` are complementary, not overlapping:

| | `wiki new` | `wiki ingest` |
|---|---|---|
| Purpose | Create an empty page or section | Bring existing content into the wiki |
| Input | A slug | A file or folder path |
| Frontmatter | Generated scaffold | Preserved if present, generated if absent |
| Use when | Starting from scratch | Copying existing content into the wiki |

Typical authoring flow:

```
wiki new page concepts/mixture-of-experts
$EDITOR concepts/mixture-of-experts.md   # or LLM fills it in
# done — wiki discovers it automatically
```

With LLM enrichment:

```
wiki new page sources/switch-transformer-2021
# LLM reads the paper, fills in frontmatter + body
wiki ingest sources/switch-transformer-2021.md --analysis enrichment.json
```

---

## 7. Rust Module Changes

| Module | Change |
|--------|--------|
| `cli.rs` | Add `new` subcommand with `page` and `section` subcommands |
| `integrate.rs` | Add `create_page(slug, bundle, wiki_root)` and `create_section(slug, wiki_root)` |
| `markdown.rs` | Add `scaffold_frontmatter(slug, type)` — derive title from slug, fill defaults |
| `server.rs` | Add `wiki_new_page` and `wiki_new_section` MCP tools |

No changes to `ingest.rs`, `search.rs`, `context.rs`, `graph.rs`.

---

## 8. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki new page <slug>` | **not implemented** |
| `wiki new page <slug> --bundle` | **not implemented** |
| `wiki new section <slug>` | **not implemented** |
| `wiki_new_page` MCP tool | **not implemented** |
| `wiki_new_section` MCP tool | **not implemented** |
