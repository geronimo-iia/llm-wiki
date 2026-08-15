# CommonMark Link Normalization

## Decision

Normalize CommonMark relative link destinations before storing them in the
Tantivy `body_links` field. `.md` is stripped; `./` and `../` prefixes are
resolved against the source page's containing directory (`source_dir`).
Normalization is pure — `source_dir` is computed at the three `index_page` call
sites and threaded as `Option<&str>` through `index_page` →
`extract_body_wikilinks` → `extract_wikilinks` → `extract_commonmark_links`.
The `[[wikilink]]` path and all callers that pass `None` are unchanged.

Extends [0.2.0/commonmark-body-links](../0.2.0/commonmark-body-links.md).

## Context

`[text](./glossary.md)` and `[text](../other/page.md)` were written verbatim
into `body_links`. The lint checker, orphan rule, and graph builder all compare
stored links against slugs (no `.md`, no relative prefix), so nothing matched.
The effect was silent: pages with relative CommonMark links appeared unlinked to
every engine consumer.

The bug was masked by the fact that absolute CommonMark slugs (`[text](slug)`)
and `[[wikilinks]]` worked correctly — only relative-path destinations were
affected.

Two layout variants complicate normalization:

- **Flat page** `technology/concurrency.md` — slug is `technology/concurrency`,
  containing directory is `technology/`.
- **Bundle page** `technology/concurrency/index.md` — slug is
  `technology/concurrency`, containing directory is `technology/concurrency/`.

The slug alone is ambiguous: `technology/concurrency` could be either layout.
The containing directory must be derived from the filesystem path at index time.

## Rationale

**Thread `source_dir`, not `source_slug`.** Passing the pre-computed directory
string (`technology/` vs `technology/concurrency/`) eliminates the flat/bundle
ambiguity in the normalization helper. A slug-based approach would require the
helper to guess the layout.

**Compute at call sites, not inside the helper.** The three `index_page` call
sites in `index_manager.rs` are the only places where both `path` (for
`file_name() == "index.md"` bundle detection) and `slug` (for directory
derivation) are simultaneously in scope. Computing `source_dir` there and passing
it down keeps the helper pure and testable without I/O.

**Normalize at index time, not at query time.** Normalizing during ingest means
all downstream consumers (graph, lint, backlinks) receive clean slugs with no
changes. Normalizing at query time would require every consumer to handle raw
relative destinations — a larger blast radius and harder to test.

**`None` for callers that don't need normalization.** `extract_links`,
`extract_parsed_links`, and `extract_parsed_wikilinks` all pass `None`.
Frontmatter `sources`/`concepts` fields are already absolute slugs;
`[[wikilinks]]` are already absolute slugs. Only CommonMark destinations
originate as relative paths. The `Option` makes the behavior explicit at every
call site.

## Consequences

- `normalize_commonmark_dest(dest, source_dir)` added to `src/links.rs` (private).
- `extract_commonmark_links`, `extract_wikilinks`, `extract_body_wikilinks`, and
  `index_page` each gain a `source_dir: Option<&str>` parameter.
- Three `index_page` call sites in `index_manager.rs` compute `source_dir` from
  `path` + `slug` before calling.
- Existing callers that passed no `source_dir` context update to `None` — no
  behavior change.
- Pages must be re-indexed for stored `body_links` to reflect the fix. A full
  `wiki_rebuild` is recommended after upgrading to 0.5.7.
- The stale "No callers change" statement in
  [0.2.0/commonmark-body-links](../0.2.0/commonmark-body-links.md) Consequences
  is superseded by this decision.
