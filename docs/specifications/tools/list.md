---
title: "List"
summary: "Paginated page listing with type and status filters and facets."
read_when:
  - Listing pages with filters
status: ready
last_updated: "2025-07-22"
---

# List

MCP tool: `wiki_list`

```
llm-wiki list
         [--type <type>]
         [--status <status>]
         [--page <n>]               # 1-based (default: 1)
         [--page-size <n>]          # default: from config
         [--format <fmt>]           # text | json (default: text)
         [--wiki <name>]
```

Results ordered alphabetically by slug via `order_by_string_fast_field`
on the `slug` FAST field. No search ranking. Only the requested page
window is extracted from the index.

Each entry includes slug, `wiki://` URI, title, type, status, and tags.

Facets (`type`, `status`, `tags` distributions) are always included.
Same hybrid filtering as `wiki_search` — `type` facet is unfiltered,
`status` and `tags` are filtered. Tag facets capped to top N (from
`defaults.facets_top_tags`).

### Output

Text (default):

```
concepts/mixture-of-experts  concept  active  Mixture of Experts
concepts/scaling-laws        concept  active  Scaling Laws
sources/switch-transformer   paper    active  Switch Transformer (2021)

Page 1/3 (42 pages)
```

JSON (`--format json`):

```json
{
  "pages": [
    {
      "slug": "concepts/mixture-of-experts",
      "uri": "wiki://research/concepts/mixture-of-experts",
      "title": "Mixture of Experts",
      "type": "concept",
      "status": "active",
      "tags": ["mixture-of-experts", "scaling"]
    }
  ],
  "total": 42,
  "page": 1,
  "page_size": 20,
  "facets": {
    "type": {
      "concept": 25,
      "paper": 10,
      "article": 5,
      "section": 2
    },
    "status": {
      "active": 40,
      "draft": 2
    },
    "tags": {
      "mixture-of-experts": 8,
      "scaling": 6,
      "transformers": 5
    }
  }
}
```
