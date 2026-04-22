# Study: Cross-wiki links — `wiki://` URIs resolved in graph and search

Explore making `wiki://` URIs first-class link targets so that pages
in one wiki can reference pages in another wiki, with those links
resolved in both the graph and search results.

## Problem

Today, links between pages are bare slugs — `sources: [concepts/moe]`,
`[[concepts/moe]]`. These resolve within a single wiki. There is no
way for a page in wiki A to link to a page in wiki B.

`wiki://` URIs exist (`wiki://research/concepts/moe`) but are only
used for display and content-read resolution. They are not recognized
as link targets by the graph builder or the index. A page that writes
`sources: [wiki://other/concepts/foo]` gets a broken edge — the graph
sees `wiki://other/concepts/foo` as a literal slug, finds no matching
node, and silently drops it.

Cross-wiki search (`--cross_wiki`) merges results from all wikis but
does not resolve cross-wiki edges.

## Current architecture

### Link extraction (`links.rs`)

`extract_links` collects slugs from frontmatter fields (`sources`,
`concepts`) and body `[[wikilinks]]`. All values are treated as bare
slugs — no URI parsing.

### Indexing (`index_manager.rs`)

Frontmatter edge fields (e.g. `sources`) are indexed as keyword values
under their field name. Body `[[wikilinks]]` are indexed under
`body_links`. Both store raw strings — no URI normalization.

### Graph (`graph.rs`)

`build_graph` reads from a single wiki's tantivy index. It builds a
`slug → NodeIndex` map and resolves edges by looking up target slugs
in that map. Cross-wiki URIs don't match any slug and are silently
dropped.

### URI resolution (`slug.rs`)

`WikiUri::parse` handles `wiki://name/slug` and `WikiUri::resolve`
looks up the wiki name in the global config. This works for
`wiki_content_read` but is not used by the link or graph pipelines.

## Proposed behavior

### Link syntax

Pages can reference other wikis using `wiki://` URIs in any link
position:

```yaml
sources:
  - concepts/local-concept          # same wiki (unchanged)
  - wiki://other/concepts/foo       # cross-wiki
```

```markdown
See [[wiki://other/concepts/foo]] for details.
```

### Graph

Cross-wiki edges appear in the graph when both wikis are mounted.
Nodes from different wikis are visually distinguishable (e.g. prefixed
slug, different style class).

```
graph LR
  concepts_moe["MoE"]:::concept
  other__concepts_foo["other/concepts/foo"]:::concept_external
  concepts_moe -->|fed-by| other__concepts_foo
```

### Search

`wiki_search` results already include `wiki://` URIs. No change
needed for result display. Cross-wiki links affect graph topology
but not BM25 ranking.

### Content read

`wiki_content_read(uri: "wiki://other/concepts/foo")` already works
via `WikiUri::resolve`. No change needed.

## Design decisions to make

### Where to resolve cross-wiki edges

Two approaches:

1. **At graph build time** — `build_graph` takes all mounted wikis,
   builds a unified slug map (`wiki_name/slug → NodeIndex`), resolves
   cross-wiki URIs against it.
2. **At index time** — normalize `wiki://name/slug` to `name/slug`
   in `body_links` and edge fields, so the index already contains
   resolvable keys.

Option 1 is simpler — no index schema change, no re-index needed.
Option 2 is more efficient for repeated graph builds but couples the
index to the multi-wiki topology.

Recommendation: option 1 (graph-time resolution).

### Canonical form for cross-wiki references

When a page in wiki `research` links to `wiki://notes/ideas/foo`:
- Stored in frontmatter as-is: `wiki://notes/ideas/foo`
- Indexed in `body_links` / edge fields as-is (raw string)
- Resolved at graph time by parsing the URI and looking up the target
  wiki

### Broken cross-wiki links

If the target wiki is not mounted, the edge is silently dropped (same
as current behavior for missing slugs). `wiki_graph` could optionally
report unresolved cross-wiki references.

### Graph scope

- `wiki_graph --wiki research` — single-wiki graph, cross-wiki edges
  shown as dangling or external nodes
- `wiki_graph --all` — unified graph across all mounted wikis,
  cross-wiki edges fully resolved

## Interaction with existing features

### Cross-wiki search

`wiki_search --cross_wiki` already merges results. Cross-wiki links
don't affect search ranking — they only affect graph topology.

### Ingest validation

`wiki_ingest` currently doesn't validate that link targets exist.
Cross-wiki links add another class of potentially broken references.
Consider a `--validate-links` flag or a lint rule.

### wiki_list

No impact — `wiki_list` doesn't deal with links.

### Hot reload

When a wiki is mounted/unmounted, cross-wiki edges to/from it become
resolvable/unresolvable. The graph reflects the current set of mounted
wikis — no persistent state needed.

## Open questions

- Should `wiki_graph` default to single-wiki or all-wiki scope?
- Should cross-wiki edges have a distinct relation label (e.g.
  `x-fed-by` vs `fed-by`) or use the same labels?
- Should `wiki_ingest` warn about cross-wiki links to unmounted wikis?
- Performance: building a unified graph across N wikis with M total
  pages — is this O(M) or worse?

## Tasks

### Spec updates

- [ ] `docs/specifications/engine/graph.md` — add "Cross-wiki edges"
  section: how `wiki://` URIs are parsed and resolved at graph build
  time, external node rendering, `--all` flag
- [ ] `docs/specifications/tools/search.md` — document that
  cross-wiki URIs in link fields are preserved as-is in the index
- [ ] `docs/specifications/tools/graph.md` — add `--all` flag for
  unified multi-wiki graph, document external node styling
- [ ] `docs/specifications/model/page-content.md` — document
  `wiki://` URI as valid link target in frontmatter edge fields and
  body wikilinks

### Link extraction

- [ ] `src/links.rs` — update `extract_links` and `extract_wikilinks`
  to recognize `wiki://name/slug` as a link target and preserve the
  full URI (not strip the prefix)
- [ ] `src/links.rs` — add `CrossWikiLink { wiki: String, slug: String }`
  struct or tag extracted links as local vs cross-wiki

### Graph: multi-wiki build

- [ ] `src/graph.rs` — add `build_graph_all` that takes multiple
  `(wiki_name, Searcher, IndexSchema, TypeRegistry)` tuples
- [ ] `src/graph.rs` — build a unified `qualified_slug → NodeIndex`
  map where qualified_slug = `wiki_name/slug` for cross-wiki and
  bare `slug` for local
- [ ] `src/graph.rs` — resolve `wiki://name/slug` edge targets by
  parsing the URI and looking up `name/slug` in the unified map
- [ ] `src/graph.rs` — tag nodes as local vs external for rendering
  (add `wiki: Option<String>` to `PageNode` or a separate field)

### Graph: rendering

- [ ] `src/graph.rs` — `render_mermaid`: add `classDef` for external
  nodes (e.g. `concept_external`), prefix external node IDs with
  wiki name
- [ ] `src/graph.rs` — `render_dot`: add `wiki` attribute to external
  nodes, optionally use `subgraph` clusters per wiki

### CLI / MCP wiring

- [ ] `src/cli.rs` — add `--all` flag to `Graph` command
- [ ] `src/mcp/tools.rs` — add `all` param to `wiki_graph` schema
- [ ] `src/mcp/handlers.rs` — when `all` is set, call
  `build_graph_all` with all mounted wikis
- [ ] `src/ops/graph.rs` — thread the `all` flag through to
  `build_graph_all`

### Tests

- [ ] Test: local link `concepts/moe` resolves in single-wiki graph
  (no regression)
- [ ] Test: `wiki://other/concepts/foo` in frontmatter creates an
  edge in unified graph when both wikis are mounted
- [ ] Test: `[[wiki://other/concepts/foo]]` in body creates an edge
  in unified graph
- [ ] Test: cross-wiki link to unmounted wiki is silently dropped
- [ ] Test: `build_graph` (single-wiki) ignores `wiki://` URIs
  gracefully (no panic, no phantom nodes)

## Success criteria

- `wiki://other/slug` in frontmatter edge fields creates a graph edge
  when both wikis are mounted
- `[[wiki://other/slug]]` in body text creates a graph edge when both
  wikis are mounted
- `wiki_graph --all` renders a unified graph across all mounted wikis
  with cross-wiki edges resolved
- Single-wiki `wiki_graph` still works — cross-wiki targets shown as
  external or dropped
- No regression in search, list, or content-read
- No index schema change required (graph-time resolution)
