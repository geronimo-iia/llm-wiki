---
title: "Architectural Invariants"
summary: "Non-obvious constraints that must hold for correctness. Violations produce no compile error."
last_updated: "2026-08-04"
---

# Architectural Invariants

Constraints that are not enforced by the type system or compiler. Violating them
produces silent correctness bugs — no panic, no error, wrong output.

## Snapshot key stability

**Invariant:** `key_fn` in `WikiGraphCache::WithSnapshot` must return a value that is
identical across process restarts for the same index content, and changes when index
content changes.

**Why:** the CLI is a fresh process on every invocation. A key that resets on startup
causes stale snapshots to be served permanently (issue #112).

```
✓  last_commit()   — reads state.toml; stable across restarts; changes on index rebuild
✗  generation()    — AtomicU64 starting at 0 on every process start
```

Set in `src/engine.rs` inside `build_wiki_graph_cache`, `key_fn` closure.

## Graph cache refresh ownership

**Invariant:** after any index write, `graph_cache` must be refreshed. The owner is
`ops::index_rebuild` (and any future `ops::*` that writes the index).

**Why:** callers of `ops::index_rebuild` (both `main.rs` and `handlers.rs`) must not
add their own `graph_cache.rebuild()` calls — the ops layer owns the side effect.
Duplicating it causes a silent double-rebuild; removing it from ops breaks CLI.

**Corollary for tests:** tests asserting cache invalidation must call
`ops::index_rebuild`, not `index_manager.rebuild()` directly. The latter rebuilds
tantivy but does not refresh the graph cache.

## Lock ordering

**Invariant:** `WikiEngine.state` (`RwLock`) must be released before acquiring it
again in the same call chain.

**Why:** `RwLock` is not reentrant on std. A read guard held across a call that
tries to acquire a write guard (or vice versa) deadlocks silently on some platforms.

Pattern in `ops::index_rebuild`:
```rust
// 1. acquire read, do index work, drop guard
let report = manager.rebuild_index(wiki_name)?;   // acquires + releases internally
// 2. acquire read again for graph cache refresh
let engine = manager.state.read()?;
```

Never hold a `state` guard across a call to any `WikiEngine` method that also
acquires `state`.

## SpaceContext field passing

**Invariant:** `graph.rs` functions accept individual fields (`&IndexSchema`,
`&SpaceTypeRegistry`, `&SpaceIndexManager`, `&WikiGraphCache`, …), never `&SpaceContext`.

**Why:** `graph.rs` and `engine.rs` would form a circular dependency if `graph.rs`
imported `SpaceContext`. Keep the boundary: `engine.rs` knows about `graph.rs`;
`graph.rs` knows nothing about `engine.rs`.

## NoSnapshot vs WithSnapshot invalidation

| Variant | Invalidation trigger | Safe for |
|---|---|---|
| `NoSnapshot(GenerationCache)` | `generation()` counter increments | same-process use (MCP long-lived) |
| `WithSnapshot(GraphState)` | `last_commit()` key changes | cross-process use (CLI short-lived) |

Do not use `NoSnapshot` in production config — it produces empty graphs on fresh CLI
processes. `graph.snapshot = false` is for tests only (avoids `.snap.lz4` files in tmpdirs).

## ops layer owns side effects

**Invariant:** `main.rs` and `handlers.rs` are thin dispatch layers. Side effects
(cache refresh, logging beyond tracing, state mutations) belong in `ops::*`.

**Why:** CLI and MCP must behave identically for the same operation. If a side effect
lives only in `handlers.rs`, the CLI silently skips it (and vice versa).

## No runtime LLM dependency

**Invariant:** llm-wiki must run as a single static binary with zero runtime LLM
dependency. No embedding model, no vector store, no network call to any AI service.

**Why:** the project is a local-first knowledge tool. Users must run it offline, in air-gapped
environments, or CI. Adding any LLM-at-runtime requirement breaks the deployment model
and forces a dependency on external services. Search stays BM25 (tantivy). Graph analysis
stays structural (petgraph). See `docs/decisions/no-embedding-search.md`.

## Schema-driven type discovery

**Invariant:** wiki types are discovered from `schemas/*.json` files via the
`x-wiki-types` extension. `wiki.toml` provides display overrides only — it cannot
introduce types that have no schema file.

**Why:** the schema is the source of truth for field structure, indexing, and validation.
If a type exists only in `wiki.toml`, tantivy schema construction and type-registry
initialization diverge silently. The authoritative lookup order is:

```
schemas/*.json (x-wiki-types) → type registry → wiki.toml overrides
```

Adding a type only in `wiki.toml` produces a misconfigured `SpaceTypeRegistry` without
any compile-time or startup error. See `docs/decisions/schema-driven-types.md`.

## Cross-wiki link resolution

**Invariant:** `wiki://` links are resolved at graph build time, not at index time.
Cross-wiki graph building is an explicit opt-in (`cross_wiki: true` in config) — it is
disabled by default.

**Why:** resolving at index time would couple per-wiki ingest to the state of other wikis,
making incremental updates non-local. Graph build time is the correct boundary because
the full multi-wiki state is available there. Broken cross-wiki links produce a lint
warning, not an ingest failure — links may temporarily dangle during partial rebuilds.

Do not validate `wiki://` slugs during `index rebuild`. See `docs/decisions/cross-wiki-links.md`.

## No stable page id

**Invariant:** there is no stable, committed page identifier (ULID, UUID, or numeric id).
Page identity is the slug (filesystem path relative to wiki root). Any id→slug mapping
must not be stored exclusively in tantivy — it would be invisible to git and lost on index
rebuild.

**Why:** tantivy is a derived, rebuildable artifact. Anything that lives only in tantivy
is implicitly ephemeral. Stable identity requires a source-of-truth in the git-tracked
markdown files or `state.toml`. Introducing a ULID field would require committing ids to
frontmatter or `state.toml` before they could be relied upon. See `docs/decisions/reject-page-id.md`.

## Filtered graph requests bypass cache

**Invariant:** only unfiltered full graphs are stored in `WikiGraphCache`. Any request
with a non-default `GraphFilter` (non-empty `root`, `types`, or `relation`) builds fresh
and is not cached.

**Why:** each distinct filter combination would require its own cache slot. The cache key
is a single generation/commit value — it cannot distinguish filter variants. Caching
filtered subgraphs would require a secondary keyed cache layer, add eviction complexity,
and provide marginal benefit (filtered requests are rare, full-graph requests dominate).

`depth` is excluded from the `is_default()` check: depth-limited requests extract a
subgraph from the full cached graph post-fetch.

Set in `src/graph.rs` `get_or_build_graph` — check `filter.is_default()` before
calling `graph_cache`. See `docs/implementation/graph-cache.md`.
