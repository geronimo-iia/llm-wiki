---
title: "Search Ranking"
summary: "How llm-wiki ranks search results, and how to tune multipliers for status and custom page states."
---

# Search Ranking

llm-wiki ranks results using a combined score applied **inside** the
tantivy collector, so the top-k returned are the true top-k — not
just the highest raw BM25 scores:

```
final_score = bm25 × status_multiplier × confidence
```

- **bm25** — term frequency / inverse document frequency score across
  `title`, `summary`, `read_when`, `tldr`, `tags`, and body text.
- **status_multiplier** — a float from the `[search.status]` map
  (see below). Default `0.9` when status is absent or not in the map.
- **confidence** — the page-level `confidence` float (default `0.5`
  when the field is absent).

## The `[search.status]` map

All status multipliers live in a single flat map. Built-in entries and
custom entries are written identically — there is no distinction:

```toml
[search.status]
active     = 1.0
draft      = 0.8
archived   = 0.3
unknown    = 0.9   # reserved fallback
```

`unknown` is a reserved key used as the fallback when a page's status
is absent or does not appear in the map. A page with `status: unknown`
in its frontmatter also matches this entry, which is intentional.

### Built-in defaults

| Status | Default multiplier | Meaning |
|---|---|---|
| `active` | `1.0` | Full weight |
| `draft` | `0.8` | Slight demotion — useful but not final |
| `archived` | `0.3` | Strong demotion — retained for reference |
| `unknown` | `0.9` | Fallback for absent or unmapped status |

## Set globally in config.toml

```toml
# ~/.llm-wiki/config.toml
[search.status]
active   = 1.0
draft    = 0.8
archived = 0.3
unknown  = 0.9
```

The four entries above are the compiled-in defaults — you only need to
write them if you want different values.

## Override per-wiki in wiki.toml

A `wiki.toml` `[search.status]` block merges **key by key** over the
global map — it does not replace it. Only declare the entries that
differ:

```toml
# wiki.toml (per-wiki)
[search.status]
archived = 0.0   # suppress archived for this wiki
stub     = 0.6   # add a custom status
```

Resolved for that wiki:

| Key | Value | Source |
|---|---|---|
| `active` | `1.0` | inherited from global |
| `draft` | `0.8` | inherited from global |
| `archived` | `0.0` | overridden by wiki |
| `unknown` | `0.9` | inherited from global |
| `stub` | `0.6` | added by wiki |

## Add custom statuses

Any status value your pages use can be given a distinct multiplier.
No code change is required — add the entry to `config.toml` or
`wiki.toml`:

```toml
[search.status]
verified   = 1.0    # fully reviewed — same weight as active
stub       = 0.6    # demote stubs without suppressing them
deprecated = 0.1    # near-zero but still findable
review     = 0.85   # in-review — slightly below active
```

Pages whose status does not appear in the map fall back to the
`unknown` multiplier.

### Example: demoting stubs

A `stub` page should appear in results but below well-developed pages:

```toml
[search.status]
stub = 0.6
```

A `stub + confidence 0.5` page scores `bm25 × 0.6 × 0.5`.
An `active + confidence 0.5` page scores `bm25 × 1.0 × 0.5`.
The active page ranks higher for the same body text.

### Example: suppressing a status entirely

Set the multiplier to `0.0` to exclude pages with that status from
results:

```toml
# wiki.toml
[search.status]
archived = 0.0
```

Score becomes zero — the page will not appear in the top-k collector.

### Example: wiki-specific review workflow

A wiki where `review` pages should rank equally with `active` ones:

```toml
# wiki.toml (review wiki)
[search.status]
review = 1.0
```

## Note: config set cannot reach map entries

`llm-wiki config set` handles scalar keys only. Status multipliers must
be edited by hand in `config.toml` or `wiki.toml`:

```toml
# config.toml or wiki.toml
[search.status]
stub = 0.6
```
