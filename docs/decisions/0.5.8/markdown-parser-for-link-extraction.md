# Markdown Parser for Link Extraction

## Decision

Adopt `pulldown-cmark` for body link extraction. Replace the manual
`[[wikilink]]` and CommonMark walkers in `src/links.rs` with an event-driven
pass over the parsed AST. Skip any `Event::Text` that falls inside a
`Tag::CodeBlock` or `Tag::Code` tag.

## Context

The 0.2.0 decision [commonmark-body-links](../0.2.0/commonmark-body-links.md)
explicitly deferred a Markdown parser with this note:

> This decision should be revisited if false-positive links from code blocks
> become a real problem.

Issue #127 is that problem. TOML `[[bench]]` and `[[pre-release-hooks]]`
array-of-tables headers inside fenced code blocks are extracted as wikilinks
by the current manual walker, producing false broken-link findings in the lint
checker and phantom edges in the graph.

The fix attempted in the 0.5.8 plan (`strip_code_content`) exposed the limits
of the manual approach:

- Byte-by-byte loop is UTF-8 unsafe — `bytes[i] as char` misinterprets
  multi-byte sequences
- Does not handle tilde fences (`~~~`), indented code blocks, or nested
  backticks in inline code
- Adds a non-trivial helper that is itself a partial Markdown parser — the
  wrong abstraction level

## Rationale

**`pulldown-cmark` is the right tool.** It is the de-facto standard Rust
CommonMark parser: zero transitive dependencies, actively maintained, used by
`mdBook`, `rustdoc`, and most of the Rust ecosystem. The dependency cost is
negligible.

**The scope is narrow.** Only `src/links.rs` changes. The extraction logic
(what counts as a link, filtering, normalization) is unchanged — only the
mechanism that feeds text to the extractor changes. All callers, all index
consumers, all tests remain valid.

**Correctness by construction.** `pulldown-cmark` handles all CommonMark code
fence variants (` ``` `, `~~~`, indented), inline code spans including nested
backticks, and all edge cases the manual walker cannot. No `strip_code_content`
helper is needed.

**`[[wikilink]]` syntax is not CommonMark.** `pulldown-cmark` does not parse
`[[...]]` natively — it surfaces them as plain `Event::Text`. The wikilink
extraction loop runs on the text content of non-code events, which is exactly
the right scope.

**Alternative considered: keep manual walker + `strip_code_content`.**
Rejected. The helper is a partial re-implementation of a Markdown parser with
known correctness gaps. Maintaining two parallel text scanners (one for code
stripping, one for link extraction) is worse than one parser pass.

## Implementation

Replace the body of `extract_wikilinks` and `extract_commonmark_links` with a
single `pulldown-cmark` event loop:

1. Parse `text` with `pulldown_cmark::Parser::new(text)`
2. Track depth inside `Tag::CodeBlock` and `Tag::Code` with a boolean flag
3. On `Event::Text(s)` outside code: run the existing `[[...]]` scan and
   CommonMark `](dest)` scan on `s`
4. All normalization (`source_dir`, `.md` stripping) is unchanged

`extract_body_wikilinks`, `extract_links`, `extract_parsed_links` signatures
are unchanged. No callers change.

## Consequences

- `pulldown-cmark` added to `Cargo.toml` dependencies
- `strip_code_content` is not written — the 0.5.8 plan Task 2 is replaced by
  this approach
- TOML `[[section]]` headers, inline code examples, and all fenced blocks are
  correctly excluded from link extraction
- Issue #127 is resolved permanently, not patched
- Supersedes the "manual walker, not a Markdown parser" rationale in
  [0.2.0/commonmark-body-links](../0.2.0/commonmark-body-links.md)
