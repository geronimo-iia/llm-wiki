---
title: "List"
summary: "Paginated page listing with type, status, tags, confidence filters, sort, and facets."
read_when:
  - Listing pages with filters
status: ready
last_updated: "2026-09-24"
---

# List

MCP tool: `wiki_list`

```
llm-wiki list
         [--type <type>]
         [--status <status>]
         [--tags <tag>]...           # filter by tag (repeatable)
         [--tags-mode and|or]        # tag match mode (default: and)
         [--min-confidence <f>]      # minimum confidence threshold [0.0–1.0]
         [--sort slug|confidence|status]  # sort field (default: slug)
         [--order asc|desc]          # sort direction (default: asc)
         [--page <n>]               # 1-based (default: 1)
         [--page-size <n>]          # default: from config
         [--format <fmt>]           # text | json | llms (default: text)
         [--wiki <name>]
```

Results ordered by the `--sort` field via tantivy native fast-field collectors.
No search ranking. Only the requested page window is extracted from the index.

## Filter and sort parameters

| Parameter | Type | Default | Semantics |
|---|---|---|---|
| `tags` | array[string] | absent = no filter | Tag filter list |
| `tags_mode` | `and` \| `or` | `and` | AND = all tags must match; OR = any one tag matches |
| `min_confidence` | float 0.0–1.0 | absent = no threshold | Pages without a `confidence` field always pass |
| `sort` | `slug` \| `confidence` \| `status` | `slug` | Sort field |
| `order` | `asc` \| `desc` | `asc` | Sort direction |

Sort implementation:

| `sort` value | Tantivy collector |
|---|---|
| `slug` | `order_by_string_fast_field("slug", order)` |
| `status` | `order_by_string_fast_field("status", order)` — lexicographic |
| `confidence` | `order_by_fast_field::<f64>("confidence", order)` — pages without the field sort to tantivy's absent-value position (confirmed by `list_sort_confidence_desc` test) |

`min_confidence` is applied as a post-collection filter on the page window.
Pages without a `confidence` field are treated as `1.0` and always pass.

Each entry includes slug, `wiki://` URI, title, type, status, tags, and `confidence`.

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
      "tags": ["mixture-of-experts", "scaling"],
      "confidence": 0.9
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

LLM (`--format llms`):

Pages grouped by type (count desc), one line per page, with summary.
Archived pages rendered with `~~strikethrough~~`. Pagination footer
included when more than one page of results exists.

```markdown
## concept (25)

- [Mixture of Experts](wiki://research/concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Scaling Laws](wiki://research/concepts/scaling-laws): Empirical laws relating model size, data, and compute to performance.
- ~~[Old Concept](wiki://research/concepts/old-concept): Superseded by newer understanding.~~

## paper (10)

- [Switch Transformer](wiki://research/sources/switch-transformer-2021): Scales to trillion parameters using sparse MoE routing.
```

Within each type group, pages are ordered by `confidence` desc, then title asc.
Pagination is unchanged — call `wiki_list(format: "llms", page: 2)` etc.

### PageSummary fields

Each page object (`PageSummary`) contains:

| Field        | Type         | Description                             |
| ------------ | ------------ | --------------------------------------- |
| `slug`       | string       | Page slug                               |
| `uri`        | string       | `wiki://<name>/<slug>`                 |
| `title`      | string       | Page title                              |
| `type`       | string       | Page type                               |
| `status`     | string       | Lifecycle status                        |
| `tags`       | list[string] | Tags                                    |
| `confidence` | float        | Page `confidence` value (default `0.5`) |
| `summary`    | string       | Page summary (omitted when empty)       |
