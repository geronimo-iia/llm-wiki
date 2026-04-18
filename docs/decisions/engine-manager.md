# EngineManager for Change Propagation

## Decision

All state mutations go through `EngineManager`. It detects what changed,
rebuilds only the affected components, and cascades to downstream
dependencies.

## Context

The engine has several stateful components that depend on each other:
type registry → index schema → tantivy index → graph. A change in one
may require rebuilding others. Without a central coordinator, each
component would need to know about its dependents.

## Alternatives Considered

| Approach                               | Why not                                                     |
| -------------------------------------- | ----------------------------------------------------------- |
| Each component watches its own sources | Duplicated change detection, no coordination                |
| Rebuild everything on any change       | Wasteful — a type change shouldn't rebuild unaffected wikis |
| Event bus / pub-sub                    | Over-engineered for a single-process engine                 |

## How It Works

`EngineManager` wraps `Engine` with `Arc<RwLock<>>`:

- Tools read from `Engine` via the shared reference (no lock contention)
- Mutations go through `EngineManager` methods:
  - `on_ingest` → incremental index update
  - `on_type_change` → rebuild type registry → rebuild index schema →
    partial or full index rebuild
  - `on_wiki_added` / `on_wiki_removed` → update global registries
  - `on_config_change` → reload affected values

Each component manager (`SpaceTypeRegistryManager`,
`SpaceIndexManager`) follows the same pattern: `has_changed()` for
cheap detection, `refresh()` for targeted rebuild, returns a report
describing what cascading actions are needed.

## Consequences

- Single point of coordination for all mutations
- Read-heavy workload — most tool calls only read, no lock contention
- Writes are brief — rebuild the affected component, swap the state
- Each component manager is independently testable
- Future hot reload (file watcher) plugs into the same interface
