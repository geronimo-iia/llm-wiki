# Study: Cross-wiki links — `wiki://` URIs resolved in graph and search

Make `wiki://` URIs first-class link targets so that pages in one wiki
can reference pages in another wiki, with those links resolved in the
graph.

## Problem

Today, links between pages are bare slugs — `sources: [concepts/moe]`,
`[[concepts/moe]]`. These resolve within a single wiki. There is no
way for a page in wiki A to link to a page in wiki B.

`wiki://` URIs exist (`wiki://research/concepts/moe`) but are only
used for display and content-read resolution. They are not recognized
as link targets by the graph builder or the index. A page that writes
`sources: [wiki://other/concepts/foo]` gets a broken edge — the graph
sees it as a literal slug, finds no matching node, and silently drops
it.

## Current architecture

### Link extraction (`links.rs`)

`extract_links` collects slugs from frontmatter fields (`sources`,
`concepts`) and body `[[wikilinks]]`. All values are treated as bare
slugs — no URI parsing.

### Indexing (`index_manager.rs`)

Frontmatter edge fields are indexed as keyword values. Body
`[[wikilinks]]` are indexed under `body_links`. Both store raw
strings — no URI normalization.

### Graph (`graph.rs`)

`build_graph` reads from a single wiki's tantivy index. It builds a
`slug → NodeIndex` map and resolves edges by looking up target slugs
in that map. Cross-wiki URIs don't match any slug and are silently
dropped.

### URI resolution (`slug.rs`)

`WikiUri::parse` handles `wiki://name/slug` and `WikiUri::resolve`
looks up the wiki name in the global config. This works for
`wiki_content_read` but is not used by the link or graph pipelines.

## Decisions

- **Graph-time resolution** — cross-wiki URIs are resolved at graph
  build time, not at index time. No index schema change, no re-index
  needed. The index stores raw strings as-is.
- **`--cross-wiki` flag** — consistent with `wiki_search`. Single-wiki
  graph is the default, `--cross-wiki` builds a unified graph across
  all mounted wikis.
- **Same relation labels** — `fed-by` is `fed-by` whether the target
  is local or cross-wiki. The rendering distinguishes them (external
  node styling), not the relation.
- **Edge direction is irrelevant** — the graph is built dynamically
  from all mounted indexes. If A links to B, the edge exists in the
  unified graph regardless of which side you start from.
- **Lint, not ingest** — cross-wiki link validation (unmounted target
  wiki) is a lint concern, not an ingest concern. Ingest validates
  frontmatter structure, lint checks semantic quality.
- **Broken cross-wiki links** — if the target wiki is not mounted,
  the edge is silently dropped (same as current behavior for missing
  slugs). Lint reports them.

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

Single-wiki graph (`wiki_graph`): cross-wiki URIs are shown as
external nodes (visually distinct) with edges, but the target node
has no metadata (title, type) since it's not in the local index.

Unified graph (`wiki_graph --cross-wiki`): all mounted wikis
contribute nodes. Cross-wiki edges are fully resolved. Nodes are
prefixed with wiki name for disambiguation.

```
graph LR
  research/concepts/moe["MoE"]:::concept
  notes/concepts/foo["foo"]:::concept_external
  research/concepts/moe -->|fed-by| notes/concepts/foo
```

### Content read

`wiki_content_read(uri: "wiki://other/concepts/foo")` already works.
No change needed.

### Search

Cross-wiki links affect graph topology but not BM25 ranking. No
change needed for search.

## Interaction with existing features

### Hot reload

When a wiki is mounted/unmounted, cross-wiki edges to/from it become
resolvable/unresolvable. The graph reflects the current set of mounted
wikis — no persistent state needed.

### Cross-wiki search

`wiki_search(cross_wiki: true)` already merges results. No change.

### Facets

No impact — facets count within the result set, not across links.

## Open questions

- Should external nodes in single-wiki graph show a placeholder title
  (e.g. the slug) or be omitted entirely?
- Performance: building a unified graph holds multiple searchers open.
  Fine for 2-3 wikis — worth noting as a constraint.

## Tasks

### 1. Update specifications

- [ ] `docs/specifications/engine/graph.md` — add "Cross-wiki edges"
  section: URI parsing at graph build time, external node rendering,
  `--cross-wiki` flag
- [ ] `docs/specifications/tools/graph.md` — add `--cross-wiki` flag
  for unified multi-wiki graph, document external node styling
- [ ] `docs/specifications/model/page-content.md` — document
  `wiki://` URI as valid link target in frontmatter edge fields and
  body wikilinks

### 2. Link extraction

- [ ] `src/links.rs` — update `extract_links` and `extract_wikilinks`
  to recognize `wiki://name/slug` and preserve the full URI
- [ ] `src/links.rs` — add `ParsedLink` enum: `Local(slug)` vs
  `CrossWiki { wiki, slug }` for downstream consumers

### 3. Graph: multi-wiki build

- [ ] `src/graph.rs` — add `build_graph_cross_wiki` that takes
  multiple `(wiki_name, Searcher, IndexSchema, TypeRegistry)` tuples
- [ ] `src/graph.rs` — build a unified `wiki_name/slug → NodeIndex`
  map, resolve `wiki://name/slug` targets by parsing the URI
- [ ] `src/graph.rs` — tag nodes as local vs external for rendering

### 4. Graph: rendering

- [ ] `src/graph.rs` — Mermaid: add `classDef` for external nodes,
  prefix external node IDs with wiki name
- [ ] `src/graph.rs` — DOT: add `wiki` attribute to external nodes,
  optionally use `subgraph` clusters per wiki

### 5. CLI / MCP wiring

- [ ] `src/cli.rs` — add `--cross-wiki` flag to `Graph` command
- [ ] `src/mcp/tools.rs` — add `cross_wiki` param to `wiki_graph`
- [ ] `src/mcp/handlers.rs` — when `cross_wiki` is set, call
  `build_graph_cross_wiki` with all mounted wikis
- [ ] `src/ops/graph.rs` — thread the `cross_wiki` flag

### 6. Lint

- [ ] Update lint to detect `wiki://` URIs pointing to unmounted
  wikis and report them as broken cross-wiki links

### 7. Tests

- [ ] Local link resolves in single-wiki graph (no regression)
- [ ] `wiki://other/slug` in frontmatter creates edge in unified graph
- [ ] `[[wiki://other/slug]]` in body creates edge in unified graph
- [ ] Cross-wiki link to unmounted wiki is silently dropped
- [ ] Single-wiki graph handles `wiki://` URIs gracefully
- [ ] Existing test suite passes unchanged

### 8. Decision record

- [ ] `docs/decisions/cross-wiki-links.md`

### 9. Update skills

- [ ] `llm-wiki-skills/skills/graph/SKILL.md` — `--cross-wiki` flag,
  cross-wiki edge rendering
- [ ] `llm-wiki-skills/skills/content/SKILL.md` — `wiki://` URI as
  valid link target
- [ ] `llm-wiki-skills/skills/lint/SKILL.md` — broken cross-wiki
  link detection

### 10. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets -- -D warnings`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md`
- [ ] Remove this prompt

## Success criteria

- `wiki://other/slug` in frontmatter edge fields creates a graph edge
  when both wikis are mounted
- `[[wiki://other/slug]]` in body text creates a graph edge when both
  wikis are mounted
- `wiki_graph(cross_wiki: true)` renders a unified graph across all
  mounted wikis with cross-wiki edges resolved
- Single-wiki `wiki_graph` still works — cross-wiki targets shown as
  external nodes or dropped
- No regression in search, list, or content-read
- No index schema change required
