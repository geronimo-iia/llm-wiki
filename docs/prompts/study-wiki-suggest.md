# Study: wiki_suggest — suggest related pages to link

Given a page, suggest related pages the user might want to link.
Helps the LLM build better-connected knowledge graphs.

## Problem

After creating or updating a page, the author (human or LLM) may
not know what other pages exist that should be linked. The graph
stays sparse, search misses connections, and knowledge stays siloed.

Today the LLM must manually search for related content. A dedicated
tool would make this automatic and consistent.

## Decisions

- **Suggest only** — the tool never modifies pages. The LLM or user
  decides which suggestions to apply.
- **Single-wiki** — no cross-wiki suggestions for now. Keeps it
  simple.
- **Three strategies** — tag overlap, graph neighborhood, BM25
  similarity. Combined with deduplication and ranking.
- **Semantic similarity deferred** — added when hybrid search lands.
- **Edge field suggestion** — based on page type and candidate type,
  using `x-graph-edges` declarations.
- **Score threshold** — suppress suggestions below a minimum
  relevance score to avoid noise. Configurable via
  `suggest.min_score` (default: 0.1).
- **Default limit** — `suggest.default_limit = 5` in config.
- **19th MCP tool** — `wiki_suggest`.

## Proposed behavior

### CLI

```
llm-wiki suggest <slug|uri>
            [--limit <n>]           # default: from config
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

### Response (JSON)

```json
[
  {
    "slug": "sources/switch-transformer-2021",
    "uri": "wiki://research/sources/switch-transformer-2021",
    "title": "Switch Transformer (2021)",
    "type": "paper",
    "score": 0.85,
    "reason": "shares tags: mixture-of-experts, scaling",
    "field": "sources"
  }
]
```

### Response (text)

```
sources/switch-transformer-2021  0.85  Switch Transformer (2021)
  → sources  (shares tags: mixture-of-experts, scaling)
concepts/scaling-laws            0.72  Scaling Laws
  → concepts  (2 hops via concepts/transformer)
```

`field` suggests where to add the link. `reason` explains why.

## Suggestion strategies

### 1. Tag overlap

Pages sharing tags with the input page. Score = shared tags / total
tags on candidate. Cheap — tantivy keyword query per tag.

### 2. Graph neighborhood

Pages within 2 hops in the concept graph that aren't directly linked.
"Friends of friends" — if A links to B and B links to C, suggest C
for A. Score = 1/hops.

### 3. BM25 similarity

Use the page's `title + summary` as a search query. Pages that rank
high but aren't already linked are candidates. Score = normalized
BM25 score.

### Combined ranking

Run all three strategies, merge with deduplication (by slug), take
the max score per candidate, sort descending, apply threshold, cap
to limit.

## Edge field suggestion

Based on the page type and the candidate type, suggest which
frontmatter field to use. Read `x-graph-edges` from the page's
schema to determine valid target types per field:

| Page type | Candidate type | Suggested field |
|-----------|---------------|-----------------|
| concept | source types | `sources` |
| concept | concept | `concepts` |
| source types | source types | `sources` |
| source types | concept | `concepts` |
| doc | source types | `sources` |
| skill | doc | `document_refs` |
| any | any (fallback) | body `[[wikilink]]` |

## Interaction with existing features

- **Ingest** — after `wiki_ingest`, the LLM could auto-run
  `wiki_suggest` to propose links for the ingested page
- **Crystallize** — suggest links for newly created query-result pages
- **Lint** — `wiki_suggest` with high limit could power an
  "under-linked pages" audit
- **Semantic search (future)** — when available, adds a 4th strategy
  for terminology-independent suggestions

## Tasks

### 1. Update specifications

- [ ] Create `docs/specifications/tools/suggest.md` — CLI, MCP,
  response format, strategies, edge field suggestion
- [ ] Update `docs/specifications/tools/overview.md` — add
  `wiki_suggest` (19 tools)
- [ ] Update `docs/specifications/model/global-config.md` — add
  `suggest.default_limit` (default: 5) and `suggest.min_score`
  (default: 0.1)

### 2. Config

- [ ] `src/config.rs` — add `SuggestConfig { default_limit: u32,
  min_score: f32 }` with defaults
- [ ] Add to `GlobalConfig` (overridable per wiki)
- [ ] Wire get/set

### 3. Suggestion engine

- [ ] `src/ops/suggest.rs` — `Suggestion` struct (slug, uri, title,
  type, score, reason, field)
- [ ] `src/ops/suggest.rs` — `suggest()` function that:
  1. Reads the input page (frontmatter + existing links)
  2. Runs tag overlap strategy
  3. Runs graph neighborhood strategy (2 hops)
  4. Runs BM25 similarity strategy (title + summary as query)
  5. Merges, deduplicates, ranks, applies threshold, caps to limit
  6. Suggests edge field per candidate using `x-graph-edges`
- [ ] `src/ops/mod.rs` — export suggest

### 4. MCP

- [ ] `src/mcp/tools.rs` — add `wiki_suggest` tool schema (slug,
  limit, wiki)
- [ ] `src/mcp/handlers.rs` — `handle_suggest` handler

### 5. CLI

- [ ] `src/cli.rs` — add `Suggest` command with slug, `--limit`,
  `--format`, `--wiki`
- [ ] `src/main.rs` — render suggestions in text and JSON

### 6. Tests

- [ ] Suggest returns candidates for a page with shared tags
- [ ] Suggest excludes already-linked pages
- [ ] Suggest respects limit
- [ ] Suggest on isolated page returns BM25-based candidates
- [ ] Edge field suggestion matches type rules
- [ ] Empty wiki returns empty suggestions
- [ ] Existing test suite passes unchanged

### 7. Decision record

- [ ] `docs/decisions/wiki-suggest.md`

### 8. Update skills

- [ ] `llm-wiki-skills/skills/content/SKILL.md` — mention suggest
  after creating/updating pages
- [ ] `llm-wiki-skills/skills/ingest/SKILL.md` — suggest after
  ingest to propose links
- [ ] `llm-wiki-skills/skills/crystallize/SKILL.md` — suggest for
  newly created pages
- [ ] `llm-wiki-skills/skills/lint/SKILL.md` — reference suggest
  for under-linked page detection

### 9. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets -- -D warnings`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md`
- [ ] Remove this prompt

## Success criteria

- `wiki_suggest("concepts/moe")` returns related pages with scores,
  reasons, and suggested fields
- Already-linked pages are excluded
- Edge field suggestions match `x-graph-edges` declarations
- Score threshold suppresses low-relevance noise
- 19 MCP tools total
