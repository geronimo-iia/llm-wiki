---
title: "Page Creation"
summary: "How to create new wiki pages and sections — llm-wiki new takes a wiki:// URI as target and scaffolds frontmatter."
read_when:
  - Adding a new page, bundle, or section to the wiki
  - Implementing the llm-wiki new subcommand
  - Understanding the difference between page creation and ingest
status: draft
last_updated: "2025-07-15"
---

# Page Creation

`llm-wiki new` creates pages and sections inside a target wiki. The target is
always a `wiki://` URI — it encodes both the wiki name and the slug in one
argument.

---

## 1. Two Primitives

### Page

A Markdown file with frontmatter. Either a flat file or a bundle (page + assets).

```bash
llm-wiki new page wiki://research/concepts/mixture-of-experts          # flat page
llm-wiki new page wiki://research/concepts/mixture-of-experts --bundle # bundle
```

Generates a minimal frontmatter scaffold:

```yaml
---
title: "Mixture of Experts"
summary: ""
status: draft
last_updated: "2025-07-15"
type: page
tags: []
read_when: []
---
```

Title is derived from the last slug segment (`mixture-of-experts` →
`Mixture of Experts`). Type defaults to `page` — the author sets the
appropriate type after creation.

### Section

A directory that groups related pages, always with an `index.md`.

```bash
llm-wiki new section wiki://research/skills
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

---

## 2. Default Wiki

When the wiki name is omitted from the URI, the default wiki
(`global.default_wiki`) is used:

```bash
llm-wiki new page wiki://concepts/mixture-of-experts     # default wiki
llm-wiki new section wiki://skills                        # default wiki
```

---

## 3. Auto-creating Parent Sections

If a parent section does not exist, it is created automatically with its
`index.md`:

```bash
llm-wiki new page wiki://research/a/b/c
# wiki://research/a       does not exist → create a/index.md
# wiki://research/a/b     does not exist → create a/b/index.md
# create a/b/c.md
```

All created files are ready to commit via `llm-wiki commit`.

---

## 4. Flat Page vs Bundle

| | Flat page | Bundle |
|---|---|---|
| Form | `{slug}.md` | `{slug}/index.md` |
| Assets | None | Co-located beside `index.md` |
| When to use | Text-only content | Page has diagrams, configs, scripts |

A flat page can be promoted to a bundle later — `llm-wiki ingest` handles the
`{slug}.md` → `{slug}/index.md` promotion automatically when the first asset
is co-located. See [asset-ingest.md](asset-ingest.md).

---

## 5. CLI Interface

```
llm-wiki new page <wiki:// URI>     # flat page with minimal frontmatter
             [--bundle]         # bundle folder + index.md instead
             [--dry-run]

llm-wiki new section <wiki:// URI>  # directory + index.md with frontmatter
                [--dry-run]
```

Errors:
- URI already exists → error, no overwrite
- Unknown wiki name in URI → error

Neither `new page` nor `new section` commits. Use `llm-wiki commit` after
writing content.

---

## 6. MCP Tools

```rust
#[tool(description = "Create a new empty wiki page with minimal frontmatter")]
async fn wiki_new_page(
    &self,
    #[tool(param)] uri: String,         // wiki:// URI
    #[tool(param)] bundle: Option<bool>,
) -> String { ... }  // returns wiki:// URI of created page

#[tool(description = "Create a new wiki section with an index page")]
async fn wiki_new_section(
    &self,
    #[tool(param)] uri: String,         // wiki:// URI
) -> String { ... }  // returns wiki:// URI of created section
```

MCP tools return the `wiki://` URI of the created resource — the LLM uses
it directly for subsequent `wiki_read` or `wiki_ingest` calls.

---

## 7. Relationship to Ingest

| | `llm-wiki new` | `llm-wiki ingest` |
|---|---|---|
| Purpose | Create an empty page or section | Validate and index files in the wiki tree |
| Input | A `wiki://` URI | A path relative to wiki root |
| Frontmatter | Generated scaffold | Preserved if present, generated if absent |
| Use when | Starting from scratch | Committing content already written into the wiki tree |

Typical authoring flow:

```bash
llm-wiki new page wiki://research/concepts/mixture-of-experts
llm-wiki read wiki://research/concepts/mixture-of-experts   # LLM reads scaffold
# LLM writes updated content via wiki_write
llm-wiki ingest concepts/mixture-of-experts.md
```
