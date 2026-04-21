---
title: "List"
summary: "Paginated page listing with type and status filters."
read_when:
  - Listing pages with filters
status: ready
last_updated: "2025-07-21"
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
  "page_size": 20
}
```
