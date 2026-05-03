---
title: "Lock Patterns"
summary: "How RwLock<EngineState> is acquired and released across the codebase — rules, anti-patterns, and call-site examples."
status: ready
last_updated: "2026-05-03"
read_when:
  - Adding a new operation that reads or writes engine state
  - Writing an MCP handler that needs both a read and a write path
  - Debugging a deadlock or "lock poisoned" error
---

# Lock Patterns

`WikiEngine` wraps `EngineState` in `Arc<RwLock<EngineState>>`. Every tool
handler acquires a read guard; write paths acquire a write guard. This doc
describes the access patterns, rules, and common mistakes.

## Core Rule: Hold the Lock for the Minimum Scope

Acquire → do work → drop. Never hold a guard across an `await` point, a
blocking I/O call, or a function that internally acquires the same lock.

The guard drops when it goes out of scope. Explicitly dropping early with
`drop(guard)` is acceptable and sometimes necessary.

## The Two Access Paths

### 1. Read-only operations (tools, graph, search, lint)

```rust
// Option A: McpServer helper (used in handlers.rs)
let engine = server.engine();           // RwLockReadGuard<'_, EngineState>
let space = engine.space(wiki_name)?;   // &Arc<SpaceContext>
let searcher = space.index_manager.searcher()?;
// use space, searcher — engine guard stays alive for the block

// Option B: Direct lock acquisition (used in WikiEngine methods)
let engine = self.state.read()
    .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
let space = engine.space(wiki_name)?;
```

The guard must outlive all borrows derived from it. `space` and `searcher`
borrow through the guard — they cannot escape the block.

### 2. Write-triggering operations (rebuild_index, refresh_index)

Write paths in `WikiEngine` acquire the read lock, do the work using
`SpaceIndexManager` (which has its own internal synchronisation), then
return. They do NOT hold a write lock for the duration of the rebuild.

```rust
pub fn rebuild_index(&self, wiki_name: &str) -> Result<IndexReport> {
    let engine = self.state.read()
        .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
    let space = engine.space(wiki_name)?;
    // SpaceIndexManager.rebuild() does its own I/O; RwLock is still read-held
    let report = space.index_manager.rebuild(...)?;
    Ok(report)  // guard drops here
}
```

The write lock (`self.state.write()`) is used only in `mount_space` at startup
or hot-reload, when `SpaceContext` itself needs to be inserted or replaced.

## Two-Phase Pattern: Read → Drop → New Read

When an operation must first read, then later re-read after some external
mutation, release the guard between the two reads. Do not try to hold
the guard across the mutation.

```rust
// CORRECT: release read guard before doing the external work, re-acquire after
let wiki_name = {
    let engine = server.engine();           // read guard acquired
    resolve_wiki_name(&engine, args)?       // borrow resolved while guard held
};                                          // guard drops here

let report = ops::index_rebuild(&server.manager, &wiki_name)?;  // external work

// Now re-acquire for the post-rebuild graph refresh
let engine = server.engine();               // fresh read guard
if let Ok(space) = engine.space(&wiki_name) {
    // ...
}
```

This is the pattern used in `handle_index_rebuild` — `wiki_name` is extracted
first (guard drops at the closing `}`), `ops::index_rebuild` runs (which
internally acquires its own read guard), then a second `server.engine()` call
gets a fresh guard for the graph refresh.

## Never Hold a Guard Across a Function That Re-Acquires It

`WikiEngine::rebuild_index` acquires `self.state.read()` internally. If a
caller already holds `self.state.read()`, calling `rebuild_index` will
deadlock (RwLock is not re-entrant on most implementations).

```rust
// WRONG — caller holds read guard, then calls rebuild_index which re-acquires
let engine = self.state.read().unwrap();
let _ = self.rebuild_index(wiki_name);   // deadlock

// CORRECT — drop guard before calling
let wiki_name = { let e = self.state.read().unwrap(); e.space(name)?.name.clone() };
let _ = self.rebuild_index(&wiki_name);
```

## Lifetime Escaping: 'static Closures

`GraphState::builder().key_fn(...).build_fn(...)` requires `'static` closures.
Guards cannot be captured there — they hold a borrow of the `RwLock`.

```rust
// WRONG — tries to capture engine guard in 'static closure
let engine = self.state.read().unwrap();
let state = GraphState::builder(cfg)
    .build_fn(move || {
        let space = engine.space(name)?;   // E0521: borrowed data escapes
        ...
    })
    .init()?;

// CORRECT — capture only owned/Arc data; re-acquire inside closure
let repo_root: PathBuf = { let e = self.state.read().unwrap(); e.space(name)?.repo_root.clone() };
let im = Arc::clone(&space.index_manager);
let state = GraphState::builder(cfg)
    .build_fn(move || {
        let searcher = im.searcher()...;
        let (tr, is) = space_builder::build_space(&repo_root, &tokenizer)?;
        build_graph(&searcher, &is, &filter, &tr)
    })
    .init()?;
```

The rule: closures that must be `'static` may only capture `Arc<T>`, `PathBuf`,
`String`, and other owned types. Never capture guard-derived borrows.

## `SpaceContext` field access under the guard

`engine.space(name)` returns `&Arc<SpaceContext>`. Fields on `SpaceContext` are
accessible directly. `Arc<T>` fields (like `index_manager`) can be cloned for
`'static` captures:

```rust
let engine = server.engine();                         // RwLockReadGuard
let space = engine.space(wiki_name)?;                 // &Arc<SpaceContext>
let im = Arc::clone(&space.index_manager);            // Arc clone — cheap
drop(engine);                                         // release guard
// im is now an independent Arc, safe to move into async or 'static closures
```

Plain fields like `type_registry` and `wiki_root` are not `Arc`-wrapped. To
capture them for `'static` use, clone them while the guard is held (if Clone),
or re-derive from disk inside the closure (if not Clone — e.g. `SpaceTypeRegistry`).

## Poisoned Lock

`RwLock::read()` returns `Err` if a previous holder panicked while holding
a write guard. Pattern throughout the codebase:

```rust
self.state.read().map_err(|_| anyhow::anyhow!("lock poisoned"))?
```

`McpServer::engine()` panics directly (`expect("engine lock poisoned")`).
Both are acceptable — a poisoned lock means the engine is in an inconsistent
state and the request cannot be served anyway.

## Fields That Have Their Own Synchronisation

These `SpaceContext` fields do NOT need the `EngineState` write lock for
mutations — they use internal synchronisation:

| Field | Type | Sync mechanism |
|-------|------|----------------|
| `index_manager` | `Arc<SpaceIndexManager>` | Internal `RwLock<IndexInner>` |
| `graph_cache` | `WikiGraphCache` | `GenerationCache` (atomic) or `GraphState` (internal Mutex) |
| `community_cache` | `GenerationCache<CommunityData>` | Atomic generation check |

The `EngineState` write lock is only needed to add, remove, or replace
a `SpaceContext` entry in `EngineState.spaces`.
