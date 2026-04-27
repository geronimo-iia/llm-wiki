---
title: "Search"
summary: "Full-text search with optional type filter and facets."
read_when:
  - Searching the wiki
status: ready
last_updated: "2026-04-27"
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
            [--format <fmt>]          # text | json | llms (default: text)
            [--wiki <name>]
```

BM25 ranks across `title`, `summary`, `read_when`, `tldr`, `tags`, and
body text. `--type` adds a keyword filter on the `type` field.

Results are ranked by a combined score applied **inside** the tantivy
collector (via `tweak_score`), so the top-k returned are the true
top-k — not just the top-k by raw BM25:

```
final_score = bm25 × status_multiplier × confidence
```

| `status` | Default multiplier |
|---|---|
| `active` | ×1.0 |
| `draft` | ×0.8 |
| `archived` | ×0.3 |
| absent / not in map | ×0.9 (the `unknown` entry) |
| any custom status | configurable |

`confidence` is the page-level float (default `0.5` when absent).
Multipliers are configurable via `[search.status]` in `config.toml` / `wiki.toml`.
Custom statuses (`verified`, `stub`, `deprecated`, …) can be added to the map
alongside the built-ins — no code change required.

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
      "confidence": 0.9,
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

LLM (`--format llms`):

One line per result: `- [title](uri): summary`. No score, no excerpt
block. When `format: "llms"` is set, excerpts are suppressed.

```markdown
- [Mixture of Experts](wiki://research/concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Switch Transformer](wiki://research/sources/switch-transformer-2021): Scales to trillion parameters using sparse MoE routing.
```

### PageRef fields

Each result object (`PageRef`) contains:

| Field        | Type   | Description                                   |
| ------------ | ------ | --------------------------------------------- |
| `slug`       | string | Page slug                                     |
| `uri`        | string | `wiki://<name>/<slug>`                        |
| `title`      | string | Page title                                    |
| `score`      | float  | Combined final score (`bm25 × status × conf`) |
| `confidence` | float  | Page `confidence` value (default `0.5`)       |
| `excerpt`    | string | Highlighted body excerpt (omitted with `--no-excerpt`) |
| `summary`    | string | Page summary (omitted when empty)             |

### `[search.status]` config

Multipliers live in a flat map under `[search.status]` in `config.toml`
(global) or `wiki.toml` (per-wiki). Built-in entries and custom entries
are written identically:

```toml
[search.status]
active     = 1.0
draft      = 0.8
archived   = 0.3
unknown    = 0.9   # reserved fallback — used when status absent or not in map
# custom entries
verified   = 1.0
stub       = 0.6
deprecated = 0.1
review     = 0.85
```

`unknown` is the reserved fallback key. A page whose status is absent or
does not appear in the map uses the `unknown` multiplier.

**Per-wiki resolution is key-by-key** — a `wiki.toml` `[search.status]`
block overrides or extends individual entries; it does not replace the
entire global map. A `wiki.toml` only needs to declare the keys that differ
from the global baseline.
