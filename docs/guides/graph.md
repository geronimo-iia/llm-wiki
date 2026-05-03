---
title: "Graph Guide"
summary: "Community detection, cross-cluster suggestions, and graph tuning."
status: ready
last_updated: "2026-04-28"
---

# Graph Guide

## Community detection

When a wiki reaches `min_nodes_for_communities` pages (default: 30),
`wiki_stats()` returns a `communities` field:

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

Below `min_nodes_for_communities`, community detection is suppressed — clusters at
small scale produce noise. Tune per wiki in `wiki.toml`:

```toml
[graph]
min_nodes_for_communities   = 50  # raise for large, dense wikis
community_suggestions_limit = 3   # more cross-cluster suggestions per call
```

The Louvain phase-1 pass is capped at `n × 10` iterations to prevent oscillation
on small or cyclic graphs. This has no effect on convergence for normal wikis.

## Structural health

Three `wiki_lint` rules report structural fragility:

| Rule | What it means | How to fix |
|------|---------------|------------|
| `articulation-point` | Removing this page disconnects the graph | Add alternative link paths that bypass this page |
| `bridge` | Removing this link disconnects the graph | Create at least one parallel path between the two connected components |
| `periphery` | Most structurally isolated page | Link it to more central pages |

```bash
# Run only structural rules
llm-wiki lint --rules articulation-point,bridge,periphery

# Include in full lint run (default)
llm-wiki lint
```

`wiki_stats` also reports aggregate structural metrics when the graph is below
`graph.max_nodes_for_diameter` (default 2000):

| Field | Meaning |
|-------|---------|
| `diameter` | Longest shortest path — how far apart the most distant pages are |
| `radius` | Shortest eccentricity — minimum distance from any page to all others |
| `center` | Slugs with eccentricity equal to `radius` — hub pages |

```bash
llm-wiki stats --format json | jq '{diameter, radius, center}'
```
