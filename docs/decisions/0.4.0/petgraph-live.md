---
title: "Adopt petgraph-live for graph cache"
summary: "Replace bespoke CachedGraph + RwLock<Option<CachedGraph>> with petgraph_live::cache::GenerationCache. Enables snapshot warm-start and algorithm suite in subsequent phases."
status: accepted
date: "2026-05-03"
---

# Adopt petgraph-live for graph cache

## Decision

Replace the bespoke `CachedGraph` struct and `RwLock<Option<CachedGraph>>` in `SpaceContext` with `petgraph_live::cache::GenerationCache<WikiGraph>` and a separate `GenerationCache<CommunityData>`. Delete all manual lock/generation logic in `graph.rs`.

## Context

v0.3.0 introduced a hand-rolled generation-keyed cache (see [decisions/0.3.0/graph-cache.md](../0.3.0/graph-cache.md)). It worked, but the implementation was bespoke: manual `RwLock` management, generation comparison inside `get_or_build_graph`, community data co-located with the graph in one lock. `petgraph-live` provides exactly this pattern as a maintained library and adds snapshot warm-start and a structural algorithm suite as opt-in features.

## Rationale

- **Eliminate duplication.** `GenerationCache` is the bespoke cache, extracted and generalized. No behaviour change, less code to own.
- **Foundation for Phase 2.** `GraphState` (snapshot warm-start) replaces `GenerationCache` in place — the field swap is one line.
- **Hot-path improvement.** Separate `community_cache` with nested closure pattern: community cache hit never touches the graph cache. `CommunityData.local_count` eliminates graph traversal on the min-nodes threshold check.
- **Algorithm suite.** `petgraph-live` default features include `connect` and `metrics` modules — articulation points, bridges, diameter, radius — available without additional dependencies (Phase 3).

## Consequences

- `petgraph-live = "0.3"` added to `Cargo.toml`.
- `CachedGraph` deleted. `SpaceContext` gains two typed cache fields.
- `get_or_build_graph`, `get_cached_community_map`, `get_cached_community_stats` rewritten; signatures unchanged at call sites.
- `LabeledEdge` gains `Serialize + Deserialize` (prerequisite for Phase 2 snapshot).
- Zero public behaviour change.
