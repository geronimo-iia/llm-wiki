# CommonMark Body Links

## Decision

Support both `[[slug]]` wikilink syntax and CommonMark `[text](slug)` inline
link syntax as first-class body links. Both syntaxes populate `body_links`,
appear in the graph, backlinks, and are checked by the `broken-link` lint rule.

## Context

The engine originally extracted body links from `[[slug]]` wikilink syntax only.
`[text](slug)` inline links — the standard Markdown syntax — were ignored by the
index, the graph, and all lint rules.

This gap mattered for two reasons:

1. `llm-wiki-hugo-cms` renders `[text](wiki://name/slug)` as local links via a
   render hook. Authors using that syntax had links that looked correct in Hugo
   but were invisible to the engine (no graph edge, no backlink, no lint coverage).

2. CommonMark is the standard portable link format. Wiki pages are plain Markdown
   files readable by any editor; `[[wikilinks]]` are a wiki-specific extension that
   renders as literal text in most Markdown renderers.

The question was whether to keep `[[slug]]` as the single canonical link syntax or
support both. A third option — a pre-processing Markdown parser with a full AST —
was also considered.

## Rationale

**Both syntaxes, not one.** Dropping `[[slug]]` would break all existing pages
and the wikilink convention that many authors already use. Dropping CommonMark
support would leave Hugo CMS links invisible to the engine. Supporting both
preserves backwards compatibility and makes the engine semantics match what Hugo
renders.

**Manual walker, not a Markdown parser.** A full CommonMark parser (e.g.
`pulldown-cmark`) would correctly skip fenced code blocks and handle edge cases
like reference-style links. However, it adds a dependency and significantly more
complexity for marginal benefit in practice — wiki pages rarely contain inline
link syntax inside fenced code blocks, and the existing `[[wikilink]]` extractor
has the same code-block limitation with no reported issues. The manual walker
is consistent with the existing approach. This decision should be revisited if
false-positive links from code blocks become a real problem.

**Known limitation: code blocks.** Both the `[[wikilink]]` extractor and the new
CommonMark extractor will match link syntax inside fenced code blocks. This is
a known, documented limitation, not a regression.

**Filter conservatively at extraction.** External URLs (`http://`, `https://`,
`mailto:`), anchor-only links (`#section`), and image links (`![alt](path)`) are
filtered at extraction time. `#anchor` suffixes are stripped. This matches author
intent: these are not wiki page references.

**`[text](wiki://name/slug)` as the portable cross-wiki format.** `[[wiki://name/slug]]`
works in the engine but renders as literal text in standard Markdown renderers.
`[text](wiki://name/slug)` is indexed by the engine, rendered correctly by Hugo
CMS, and readable as a plain Markdown link everywhere. Skills and docs recommend
this format for cross-wiki references.

## Consequences

- `extract_commonmark_links` added to `src/links.rs`; called by both
  `extract_parsed_wikilinks` and `extract_wikilinks`. No callers change.
- All downstream consumers (graph, backlinks, broken-link lint) automatically
  cover CommonMark links — they read from the `body_links` tantivy field, which
  is populated during ingest.
- Existing pages are not affected until re-indexed. `wiki_ingest` on any page
  picks up CommonMark links on that page.
- `[text](wiki://name/slug)` is documented as the preferred portable link format
  in skills and guides.
- Code-block false positives are a known limitation, shared with `[[wikilinks]]`.
