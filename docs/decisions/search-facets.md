# Search Facets

## Decision

`wiki_search` and `wiki_list` always return facet counts for `type`,
`status`, and `tags` alongside results. No opt-in flag needed.

## Context

To understand the shape of search results — how many concepts vs
papers, which tags dominate, how many drafts — you previously needed
to paginate through everything or run multiple filtered queries.

## Key decisions

- **Always returned** — facets are cheap (same index scan) and always
  useful. No `--facets` flag.
- **Fixed fields** — always facet on `type`, `status`, `tags`. These
  are base schema fields with low cardinality (except tags). No
  `defaults.facets_fields` config.
- **Hybrid filtering** — `type` facet is always unfiltered (shows
  full distribution even when `--type` filter is active). `status`
  and `tags` facets are filtered (describe the current result set).
- **Top-N tags** — `defaults.facets_top_tags = 10` caps tag facets.
  `type` and `status` return all values (low cardinality).
- **`wiki_list`** — always includes facets, same as search.

## Why hybrid filtering

If you search `--type concept` and the type facet is filtered, it
just says `concept: 12` — useless. Unfiltered type facet shows
`concept: 12, paper: 8, article: 3` so the agent can suggest
"there are also 8 papers on this topic."

`status` and `tags` should be filtered because they describe the
current result set — "of the concepts, how many are draft?"

This is the same approach as Algolia's "disjunctive facets."

## Implementation

Facet collection uses `DocSetCollector` to get matching doc addresses,
then iterates stored field values. This is simple and correct for
wiki-scale indexes (<10K pages). For larger indexes, a FAST column
iterator would be more efficient.

Keyword fields (`type`, `status`, `tags`) have `FAST` enabled for
future optimization.

## Consequences

- Search and list responses are larger (facets object added)
- Bootstrap can report wiki composition from list facets without
  paginating
- Research skill can show result distribution to help narrow queries
- Lint skill can find draft count from status facet
