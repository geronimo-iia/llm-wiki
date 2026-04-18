---
title: "Slug Implementation"
summary: "Slug and WikiUri types, resolution, parsing, flat vs bundle detection."
status: draft
last_updated: "2025-07-17"
---

# Slug Implementation

Implementation reference for slug and URI handling. Not a specification
-- see [page-content.md](../specifications/model/page-content.md) for
slug resolution rules.

## Why a Dedicated Module

Slug/URI logic is currently split across `markdown.rs` (slug resolution)
and `spaces.rs` (URI parsing). Both files mix this with unrelated
concerns (page I/O, space management). A dedicated `slug.rs` module
keeps it clean -- pure data transformation, no file reading or writing,
used everywhere.

## Types

```rust
/// A validated slug -- path relative to wiki root, no extension
struct Slug(String);

/// A parsed wiki:// URI
struct WikiUri {
    wiki: Option<String>,  // None = default wiki
    slug: Slug,
}
```

`Slug` wraps a `String` with validation at construction: no `../`, no
extension, no leading `/`. Once you have a `Slug`, it's valid -- no
re-checking downstream.

`WikiUri` is the parsed form of `wiki://research/concepts/moe`. Every
tool that accepts `<slug|uri>` parses input into a `WikiUri` first.

Conversions:

```rust
impl TryFrom<&str> for Slug { ... }      // validate bare slug
impl TryFrom<&str> for WikiUri { ... }   // parse wiki:// or bare slug
```

## Functions

| Function              | Input                          | Output                                       |
| --------------------- | ------------------------------ | -------------------------------------------- |
| `Slug::from_path`     | file path + wiki root          | `Slug`                                       |
| `Slug::resolve`       | `&self` + wiki root            | file path (flat or bundle)                   |
| `Slug::title`         | `&self`                        | title-cased display name                     |
| `WikiUri::parse`      | string (slug or `wiki://` URI) | `WikiUri`                                    |
| `WikiUri::resolve`    | `&self` + config               | (wiki entry, `Slug`)                         |
| `resolve_read_target` | `Slug` + wiki root             | page path or (parent `Slug`, asset filename) |

## Slug::from_path

Derives a `Slug` from a file path relative to wiki root:

- `concepts/moe.md` -> `concepts/moe`
- `concepts/moe/index.md` -> `concepts/moe`

Rule: if filename is `index.md`, slug is the parent directory. Otherwise
strip the `.md` extension.

## Slug::resolve

Resolves a `Slug` to a file path. Checks two forms in order:

1. `<wiki_root>/<slug>.md` -- flat file
2. `<wiki_root>/<slug>/index.md` -- bundle

Returns the first that exists. Error if neither found.

## Slug::title

Derives a display title from the last slug segment:

- `concepts/mixture-of-experts` -> `Mixture of Experts`

Splits on `-`, title-cases each word.

## WikiUri::parse

Parses a string into a `WikiUri`. Accepts both `wiki://` URIs and bare
slugs:

- `wiki://research/concepts/moe` -> `wiki: Some("research")`, `slug: "concepts/moe"`
- `wiki://concepts/moe` -> ambiguous (see below)
- `concepts/moe` -> `wiki: None`, `slug: "concepts/moe"`

The ambiguity: `wiki://foo/bar` -- is `foo` a wiki name or the first
slug segment? Resolution: the caller (WikiUri::resolve) checks if `foo`
is a registered wiki name. At parse time, store it as a candidate.

## WikiUri::resolve

Resolves a `WikiUri` against the global config:

1. If `wiki` is `Some(name)` and name is a registered wiki -> use it
2. If `wiki` is `Some(name)` but not registered -> treat as slug
   segment, use default wiki with `name/slug` as full slug
3. If `wiki` is `None` -> use default wiki from config

Returns `(WikiEntry, Slug)`. This is what every tool calls.

## resolve_read_target

Two-step resolution for `wiki_content_read`:

1. Try `Slug::resolve` -> page
2. If slug has a non-`.md` extension in the last segment, split into
   parent `Slug` + filename -> asset

## Existing Code

Currently in `markdown.rs` and `spaces.rs`. Reusable with extraction:

| Function              | Current location | Becomes                               |
| --------------------- | ---------------- | ------------------------------------- |
| `slug_for`            | `markdown.rs`    | `Slug::from_path`                     |
| `resolve_slug`        | `markdown.rs`    | `Slug::resolve`                       |
| `resolve_read_target` | `markdown.rs`    | `resolve_read_target`                 |
| `resolve_uri`         | `spaces.rs`      | `WikiUri::parse` + `WikiUri::resolve` |
| `title_case`          | `markdown.rs`    | `Slug::title`                         |

After extraction, `markdown.rs` becomes page I/O only (read, write,
create). `spaces.rs` becomes space management only (register, remove,
set_default).

## No I/O in This Module

`Slug::resolve` and `resolve_read_target` check file existence
(`is_file`, `is_dir`). That's the only I/O -- no file reading, no
writing. Consider accepting a trait or closure for file existence
checks to keep it testable without temp directories.
