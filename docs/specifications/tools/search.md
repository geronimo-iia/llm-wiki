---
title: "Search"
summary: "Full-text search with optional type filter and facets."
read_when:
  - Searching the wiki
status: ready
last_updated: "2025-07-22"
---

# Search

MCP tool: `wiki_search`

```
llm-wiki search "<query>"
            [--type <type>]           # filter by page type
            [--no-excerpt]            # refs only, no excerpt
            [--top-k <n>]             # default: from config
            [--include-sections]      # include section index pages
            [--all]                   # search across all registered wikis
            [--format <fmt>]          # text | json (default: text)
            [--wiki <name>]
```

BM25 ranks across `title`, `summary`, `read_when`, `tldr`, `tags`, and
body text. `--type` adds a keyword filter on the `type` field.

Facets (`type`, `status`, `tags` distributions) are always included in
the response. The `type` facet is unfiltered (shows full distribution
even when `--type` is active). `status` and `tags` facets are filtered
(reflect the current result set). Tag facets are capped to top N
(default from `defaults.facets_top_tags`).

### Examples

```bash
llm-wiki search "mixture of experts"
llm-wiki search --type concept "routing strategies"
llm-wiki search --type paper,article "transformer architecture"
llm-wiki search --type skill "process PDF files"
```

### Output

Text (default):

```
concepts/mixture-of-experts  0.94  Mixture of Experts
  Sparse routing of tokens to expert subnetworks, trading compute...
sources/switch-transformer-2021  0.81  Switch Transformer (2021)
  Switch Transformer scales to trillion parameters using sparse MoE...
```

JSON (`--format json`):

```json
{
  "results": [
    {
      "slug": "concepts/mixture-of-experts",
      "uri": "wiki://research/concepts/mixture-of-experts",
      "title": "Mixture of Experts",
      "score": 0.94,
      "excerpt": "Sparse routing of tokens to expert subnetworks, trading compute..."
    }
  ],
  "facets": {
    "type": {
      "concept": 12,
      "paper": 8,
      "article": 3
    },
    "status": {
      "active": 20,
      "draft": 3
    },
    "tags": {
      "mixture-of-experts": 15,
      "scaling": 9,
      "transformers": 7
    }
  }
}
```

The `type` facet is always unfiltered — it shows the full distribution
across all matching pages regardless of `--type` filter. This lets
agents suggest "there are also 8 papers on this topic".

`status` and `tags` facets are filtered — they describe the current
result set after type filtering.
