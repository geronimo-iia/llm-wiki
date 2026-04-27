---
title: "Suggest"
summary: "Suggest related pages to link — tag overlap, graph neighborhood, BM25 similarity."
read_when:
  - Finding related pages to link
  - Improving graph connectivity
status: ready
last_updated: "2025-07-23"
---

# Suggest

MCP tool: `wiki_suggest`

```
llm-wiki suggest <slug|uri>
            [--limit <n>]           # default: from config
            [--format <fmt>]        # text | json
            [--wiki <name>]
```

Given a page, suggests related pages the user might want to link.
Excludes pages already linked from the input page. Suggests which
frontmatter field to use based on `x-graph-edges` declarations.

### Strategies

Four strategies are combined:

1. **Tag overlap** — pages sharing tags. Score = shared / total.
2. **Graph neighborhood** — pages within 2 hops not directly linked.
   Score = 1 / hops.
3. **BM25 similarity** — input page's title + summary as query.
   Score = normalized BM25.
4. **Community peers** — pages in the same Louvain community not already linked.
   Score = 0.4, reason = "same knowledge cluster". Suppressed when
   `node_count < min_nodes_for_communities`.

Results are merged, deduplicated, ranked by max score, filtered by
`suggest.min_score`, and capped to limit.

### Edge field suggestion

Based on the page type and candidate type, the tool suggests which
frontmatter field to use. Read from `x-graph-edges` in the type
schema:

| Page type | Candidate type | Suggested field |
|-----------|---------------|-----------------|
| concept | source types | `sources` |
| concept | concept | `concepts` |
| source types | source types | `sources` |
| source types | concept | `concepts` |
| doc | source types | `sources` |
| skill | doc | `document_refs` |
| any | any (fallback) | body `[[wikilink]]` |

### Output

Text (default):

```
sources/switch-transformer-2021  0.85  Switch Transformer (2021)
  → sources  (shares tags: mixture-of-experts, scaling)
concepts/scaling-laws            0.72  Scaling Laws
  → concepts  (2 hops via concepts/transformer)
```

JSON (`--format json`):

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

### Configuration

| Key | Default | Description |
|-----|---------|-------------|
| `suggest.default_limit` | `5` | Max suggestions returned |
| `suggest.min_score` | `0.1` | Minimum score threshold |
| `graph.min_nodes_for_communities` | `30` | Suppress community strategy below this node count |
| `graph.community_suggestions_limit` | `2` | Max extra results from the community strategy |

Overridable per wiki.

## MCP Tool Definition

```json
{
  "name": "wiki_suggest",
  "description": "Suggest related pages to link",
  "parameters": {
    "slug": "(required) slug or wiki:// URI of the input page",
    "limit": "max suggestions (default: from config)",
    "wiki": "target wiki name (default: default wiki)"
  }
}
```
