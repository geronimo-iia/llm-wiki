# wiki_suggest — suggest related pages to link

## Decision

Add `wiki_suggest` as the 19th MCP tool. Given a page, suggests
related pages to link using three strategies.

## Context

After creating or updating a page, the author may not know what
other pages should be linked. The graph stays sparse.

## Key decisions

- **Suggest only** — never modifies pages.
- **Three strategies** — tag overlap, graph neighborhood (2 hops),
  BM25 similarity. Combined with dedup and max-score ranking.
- **Edge field suggestion** — uses `x-graph-edges` to suggest which
  frontmatter field to use (sources, concepts, etc.).
- **Score threshold** — `suggest.min_score = 0.1` suppresses noise.
- **Single-wiki** — no cross-wiki suggestions.
- **Semantic similarity deferred** — 4th strategy when hybrid search
  lands.

## Consequences

- 19 tools (was 18)
- LLM can auto-suggest links after ingest/crystallize
- Graph connectivity improves over time
- Lint can use suggest for under-linked page detection
