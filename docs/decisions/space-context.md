# SpaceContext: Per-Wiki Aggregate

## Decision

Rename `SpaceState` to `SpaceContext`. It holds both value data (paths,
registry, schema) and the per-wiki index manager (`SpaceIndexManager`).
There is no separate `SpaceManager` — `SpaceContext` is the single
per-wiki aggregate that callers receive from `engine.space(name)`.

## Context

`SpaceState` was originally a value bag — paths, type registry, index
schema. When introducing `SpaceIndexManager` (a struct with lifecycle
behavior: rebuild, update, staleness detection, recovery), the question
was where it should live.

Three options were considered:

| Approach | Description | Why not |
|----------|-------------|---------|
| `SpaceManager` wrapping `SpaceState` + `SpaceIndexManager` | New orchestrator per wiki | Extra layer with no second concern to justify it — YAGNI |
| `EngineManager` owns managers separately | `HashMap<String, SpaceIndexManager>` alongside spaces | Two lookups per operation, wiring scattered across engine |
| **`SpaceContext` holds everything** | Rename + add `index_manager` field | Chosen — single lookup, cohesive, honest name |

## Rationale

- **"State" was a misnomer.** The struct is immutable after construction.
  Nobody mutates it. It's a read-only context bag that every operation
  destructures to get what it needs.
- **"Context" is accurate.** Callers do `let space = engine.space(name)?`
  then reach into it. It's the per-wiki context for any operation.
- **`SpaceIndexManager` belongs here** because it shares the same
  lifetime (created at startup, lives until shutdown) and identity
  (one per wiki name). Callers already accessed `space.index_path` —
  now they access `space.index_manager.rebuild()`. Same pattern.
- **No `SpaceManager` needed.** The only per-wiki lifecycle subsystem
  is the index. `SpaceTypeRegistry` is built once and immutable — it's
  data, not a managed subsystem. If a second lifecycle concern appears
  (e.g. hot-reload of type schemas), extract a manager then.
- **`SpaceIndexManager` stays focused.** It manages one thing: the
  tantivy index for one wiki. It doesn't know about `SpaceContext` or
  the engine. It receives what it needs as parameters.

## Structure

```
EngineManager
└── Engine
    └── spaces: HashMap<String, SpaceContext>
        └── SpaceContext
            ├── name, wiki_root, repo_root    (identity)
            ├── type_registry, index_schema   (value data)
            └── index_manager                 (lifecycle: index ops)
```

## Consequences

- `SpaceContext` is the single type callers interact with for per-wiki
  operations — no indirection
- `EngineManager` stays thin: lock + routing + global policy
- `SpaceIndexManager` is independently testable (proven by
  `tests/index_manager.rs`)
- If type hot-reload is ever needed, `SpaceTypeRegistryManager` would
  be added as another field in `SpaceContext` — same pattern
