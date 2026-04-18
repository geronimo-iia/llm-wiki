---
title: "Frontmatter Parser Implementation"
summary: "YAML parsing, frontmatter extraction, body splitting, and how validation moves to the type registry."
status: draft
last_updated: "2025-07-17"
---

# Frontmatter Parser Implementation

Implementation reference for frontmatter parsing. Not a specification
— see [page-content.md](../specifications/model/page-content.md) for
the page format and [types/base.md](../specifications/model/types/base.md)
for base fields.

## Responsibilities

The frontmatter parser does two things:

1. **Split** a markdown file into frontmatter (YAML) and body
2. **Parse** the YAML into a usable structure

It does NOT:
- Validate against JSON Schema (that's `SpaceTypeRegistry`)
- Resolve aliases (that's `SpaceTypeRegistry`)
- Modify the file on disk (the engine never touches files)

## Parsing

### Split

```
---
title: "Mixture of Experts"
type: concept
...
---

## Overview
...
```

1. Strip BOM if present
2. Find opening `---` (must be first line)
3. Find closing `---`
4. YAML = content between the two markers
5. Body = everything after closing `---` + newline

### No frontmatter

If the file has no `---` opening, return empty frontmatter and the
entire content as body. The caller decides what to do (generate
minimal defaults at index time, or reject).

## Data Structure

The current code uses a typed `PageFrontmatter` struct with all known
fields. This doesn't work with the dynamic type system — a skill page
has `name`/`description`, a custom type has arbitrary fields.

### New approach: untyped frontmatter

Parse YAML into a generic `serde_yaml::Value` (or `BTreeMap<String, Value>`),
not a fixed struct:

```rust
struct ParsedPage {
    frontmatter: BTreeMap<String, serde_yaml::Value>,
    body: String,
}
```

This gives the type registry full control over what fields exist and
how they're validated. The parser doesn't need to know about types.

### Typed access

Convenience methods for common fields that every page has:

```rust
impl ParsedPage {
    fn title(&self) -> Option<&str>;
    fn page_type(&self) -> Option<&str>;
    fn status(&self) -> Option<&str>;
    fn tags(&self) -> Vec<&str>;
}
```

These read from the `BTreeMap` — no separate struct needed.

## Writing

Serialize a `BTreeMap<String, Value>` back to YAML and wrap with `---`
markers:

```rust
fn write_page(frontmatter: &BTreeMap<String, Value>, body: &str) -> String
```

Preserves field order if using `BTreeMap` (sorted alphabetically) or
`IndexMap` (insertion order — may be preferable for readability).

## Scaffolding

For `wiki_content_new`, generate minimal frontmatter:

```rust
fn scaffold(slug: &Slug, section: bool) -> BTreeMap<String, Value>
```

Returns `{ title: <from slug>, type: "page" | "section", status: "draft" }`.
The `--name` and `--type` CLI flags override these.

## Title Extraction

For files without frontmatter, extract title from body:

1. First `# Heading` in the body
2. Fall back to filename stem, title-cased

## Crate

Use the `frontmatter` crate for YAML extraction:

```toml
frontmatter = "0.4"
```

It handles BOM, `---` detection, edge cases. We parse the extracted
YAML string with `serde_yaml` into `BTreeMap<String, Value>`.

Reference: https://crates.io/crates/frontmatter

## Existing Code

The current `src/frontmatter.rs` needs significant rework:

| Component                         | Reusable | Notes                                          |
| --------------------------------- | -------- | ---------------------------------------------- |
| `parse_frontmatter` (split logic) | yes      | BOM handling, `---` detection, body extraction |
| `PageFrontmatter` struct          | remove   | Replace with `BTreeMap<String, Value>`         |
| `Claim` struct                    | remove   | Validation moves to type registry              |
| `BUILT_IN_TYPES`                  | remove   | Type registry handles this                     |
| `validate_frontmatter`            | remove   | JSON Schema validation in type registry        |
| `write_frontmatter`               | yes      | Adapt to `BTreeMap`                            |
| `scaffold_frontmatter`            | yes      | Adapt to `BTreeMap`, use `Slug::title`         |
| `generate_minimal_frontmatter`    | yes      | Adapt to `BTreeMap`                            |
| `title_from_body_or_filename`     | yes      | Move to `slug.rs` or keep here                 |

### Key change

The parser becomes dumb — split + generic YAML parse. All intelligence
(which fields exist, which are required, aliases, validation) moves to
`SpaceTypeRegistry`. This matches the design: the parser doesn't know
about types, the registry does.
