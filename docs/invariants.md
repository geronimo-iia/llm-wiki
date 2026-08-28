---
title: "Architectural Invariants"
summary: "Non-obvious constraints that must hold for correctness. Violations produce no compile error."
last_updated: "2026-08-28"
---

# Architectural Invariants

Constraints that are not enforced by the type system or compiler. Violating them
produces silent correctness bugs — no panic, no error, wrong output.

Each exception notes inline where a constraint is compile-time enforced.

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

## Concurrent rebuild serialisation

**Invariant:** at most one `rebuild()` call runs per wiki space at any time.
`SpaceIndexManager.rebuild_lock: Mutex<()>` enforces this.

**Why:** two concurrent full rebuilds on the same space both write to
`search-index-building/`, then both attempt the three-rename atomic swap.
The second rename would overwrite the first's committed index, and both
readers would reload at unpredictable points. Serialising via `rebuild_lock`
ensures each rebuild sees the outcome of the previous one.

**Corollary:** `rebuild_lock` is separate from `state: RwLock`. `state` is
held at read level during rebuild; `rebuild_lock` is the concurrency guard.
Never hold both write guards simultaneously.

## Space mutation atomicity

**Invariant:** in-memory space state and the persisted `wiki.toml` must never
diverge after a `spaces_create`, `spaces_register`, or `spaces_set_default`
call returns.

**Why:** if the in-memory mutation succeeds but the disk write fails (and is
not rolled back), subsequent requests see a default or space that the next
engine restart will not find — silent divergence between memory and disk.

Rollback rules:
- `spaces_set_default`: capture `prev_default` before calling `set_default()`;
  on disk failure restore via `state.write()` directly (bypasses
  `contains_key` validation, which rejects an empty string).
- `spaces_create` / `spaces_register`: run `mount_wiki` and
  `spaces::remove` rollback inside the same `with_config_lock` closure so no
  other write can observe the intermediate state.

See [lock-patterns.md](implementation/lock-patterns.md) for the rollback
patterns.

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

## NormalizedSlug type boundary

**Invariant (compile-time enforced):** a `NormalizedSlug` can only be constructed
via `Slug::normalize()` (public) or `NormalizedSlug::from_normalized()` (crate-internal).
Raw `String` slugs must never be compared to normalized slugs without going through
normalization first.

**Why:** slug normalisation (lowercasing all path components) was previously
convention-enforced only. A raw user-input slug and an index-read slug compared
with `==` without the compiler catching the mismatch, producing silent
false-negatives in search and lint. `NormalizedSlug(String)` makes mixing raw and
normalized slugs a type error.

Construction paths:
- `Slug::normalize() -> NormalizedSlug` — the public path for all external callers.
- `NormalizedSlug::from_normalized(String)` — `pub(crate)`, for tantivy index reads
  where the stored value is already known to be lowercased at index time.

`Slug` is unchanged: it enforces structural validity (no `..`, no leading `/`, no
extension). `NormalizedSlug` adds the lowercasing guarantee on top.

## `redact_error` Windows path gap (known limitation)

**Invariant:** `redact_error` in `src/mcp/handlers.rs` strips filesystem paths from
MCP error strings before returning them to LLM clients. The current implementation
covers Unix absolute paths (`/…`) and tilde-prefixed paths (`~/…`, `~user/…`).

**Known gap:** Windows drive-letter paths (`C:\Users\…`) and UNC paths
(`\\server\share\…`) are NOT redacted. A Windows build of `llm-wiki serve` will
leak these forms in MCP error responses.

**Why deferred:** no Windows maintainer can run `cargo test` to verify that the fix
does not over-redact short strings (`C:` alone, short UNC prefixes). The fix and the
required unit tests must be contributed together. See
`docs/decisions/1.0.0/redact-error-windows-paths.md` for the exact regex change and
test requirements.

## `WikiEngine.state` visibility

**Invariant:** `WikiEngine.state` is `pub(crate)`. External crates must not access
it directly. The public read entry point is `WikiEngine::with_state`, which acquires
the lock and maps the poison error uniformly.

**Why:** exposing the `Arc<RwLock<EngineState>>` field as `pub` couples embedders to
the lock type, poison-handling convention, and `EngineState` field layout. Any change
to the interior mutability model would be a breaking API change.

Access patterns:
- External embedders: `engine.with_state(|s| { … })`.
- Integration tests (`tests/*.rs`): `engine.state_for_test().read()` —
  `#[doc(hidden)]`, no stability guarantee.
- Crate-internal code (`pub(crate)`): `self.state.read()` / `.write()` directly.

`config_write_lock` is also `pub(crate)` with no public accessor — it is an internal
serialisation primitive, not part of the embedding contract.

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
