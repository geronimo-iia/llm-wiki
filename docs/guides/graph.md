---
title: "Graph Guide"
summary: "Community detection, cross-cluster suggestions, and graph tuning."
status: ready
last_updated: "2026-04-27"
---

# Graph Guide

## Community detection

When a wiki reaches 30+ pages, `wiki_stats()` returns a `communities` field:

| Field | Description |
|---|---|
| `count` | Number of distinct knowledge clusters |
| `largest` | Size of the biggest cluster |
| `smallest` | Size of the smallest cluster |
| `isolated` | Slugs in communities of size ≤ 2 — candidates for new links or consolidation |

Use the `isolated` list as a prioritized review queue. These pages are structurally
disconnected from the main knowledge body. Run `wiki_suggest(slug: "<isolated-slug>")`
to find the best connection candidates.

## Cross-cluster suggestions

`wiki_suggest` includes a "same knowledge cluster" strategy (strategy 4). These results
identify thematically related pages that have no direct link path — the highest-value
link candidates for improving graph connectivity.

Look for suggestions with `reason: "same knowledge cluster"` in the output.

## Tuning

Below 30 nodes, community detection is suppressed — clusters at small scale produce
noise. Tune per wiki in `wiki.toml`:

```toml
[graph]
min_nodes_for_communities   = 50  # raise for large, dense wikis
community_suggestions_limit = 3   # more cross-cluster suggestions per call
```
