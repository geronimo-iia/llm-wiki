# Study: wiki_suggest — suggest related pages to link

Given a page, suggest related pages the user might want to link.
Helps the LLM build better-connected knowledge graphs.

## Problem

After creating or updating a page, the author (human or LLM) may
not know what other pages exist that should be linked. The graph
stays sparse, search misses connections, and knowledge stays siloed.

Today the LLM must manually search for related content. A dedicated
tool would make this automatic and consistent.

## Proposed behavior

### CLI

```
llm-wiki suggest <slug|uri>
            [--limit <n>]           # default: 5
            [--format <fmt>]        # text | json
            [--wiki <name>]
```

### MCP

```json
{
  "slug": "concepts/moe",
  "limit": 5
}
```

### Response

```json
[
  {
    "slug": "sources/switch-transformer-2021",
    "uri": "wiki://research/sources/switch-transformer-2021",
    "title": "Switch Transformer (2021)",
    "type": "paper",
    "reason": "shares tags: mixture-of-experts, scaling",
    "field": "sources"
  }
]
```

`field` suggests where to add the link (`sources`, `concepts`, or
body wikilink). `reason` explains why the suggestion was made.

## Suggestion strategies

### 1. Tag overlap

Pages sharing tags with the input page. Weighted by number of shared
tags. Cheap — just a tantivy query.

### 2. Graph neighborhood

Pages within N hops in the concept graph that aren't directly linked.
"Friends of friends" — if A links to B and B links to C, suggest C
for A.

### 3. BM25 similarity

Use the page's title + summary as a search query. Pages that rank
high but aren't already linked are candidates.

### 4. Semantic similarity (future)

When hybrid search is available, use embedding similarity to find
conceptually related pages regardless of terminology.

### Combined

Run strategies 1-3, merge with deduplication, rank by combined
score. Strategy 4 added when semantic search lands.

## Edge field suggestion

Based on the page type and the candidate type, suggest which
frontmatter field to use:

| Page type | Candidate type | Suggested field |
|-----------|---------------|-----------------|
| concept | source types | `sources` |
| concept | concept | `concepts` |
| source | source types | `sources` |
| source | concept | `concepts` |
| doc | source types | `sources` |
| any | any | body `[[wikilink]]` |

Read `x-graph-edges` from the page's schema to determine valid
target types per field.

## Interaction with existing features

- Ingest: after `wiki_ingest`, the LLM could auto-run `wiki_suggest`
  to propose links
- Crystallize: suggest links for newly created query-result pages
- Lint: `wiki_suggest` with `--limit 0` could power an "under-linked
  pages" lint rule

## Open questions

- Should suggestions include cross-wiki pages when multiple wikis
  are mounted?
- Should there be a threshold below which suggestions are suppressed?
- Should the tool modify pages directly, or only suggest?
  (Recommendation: suggest only — the LLM or user decides)

## Tasks

- [ ] Spec: `docs/specifications/tools/suggest.md`
- [ ] `src/ops/suggest.rs` — suggestion engine
- [ ] `src/mcp/tools.rs` — add `wiki_suggest` tool
- [ ] `src/mcp/handlers.rs` — handler
- [ ] `src/cli.rs` — `Suggest` command
- [ ] Tests
- [ ] Decision record, changelog, roadmap, skills
