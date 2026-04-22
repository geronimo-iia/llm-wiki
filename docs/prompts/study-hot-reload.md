# Study: Hot reload — add/remove wikis without restart

Explore adding hot reload to `llm-wiki serve` so that wikis can be
added, removed, or re-registered without restarting the server.

## Problem

Today, `llm-wiki serve` mounts all registered wikis at startup. To
add or remove a wiki, the server must be restarted. This breaks
active MCP/ACP sessions and forces agents to re-bootstrap.

For workflows where wikis are created or removed frequently (e.g.
project-scoped wikis, temporary research wikis), this is disruptive.

## Current architecture

From `server.md`, the startup sequence:

```
1. Load ~/.llm-wiki/config.toml — spaces + global config
2. Mount all registered wikis
3. Check index staleness for each wiki
4. Start transports (stdio, HTTP, ACP)
```

Wikis are mounted once. The space registry (`[[wikis]]` in
`config.toml`) is read once at startup. Space management tools
(`wiki_spaces_create`, `wiki_spaces_remove`, `wiki_spaces_set_default`)
modify `config.toml` but the running server doesn't pick up changes.

## Proposed behavior

### Automatic reload

The server watches `~/.llm-wiki/config.toml` for changes. When the
file changes:

1. Re-read the space registry
2. Diff against currently mounted wikis
3. Mount new wikis (create index if needed)
4. Unmount removed wikis (close index handles)
5. Update default wiki if changed
6. Log: `reload: mounted <name>, unmounted <name>`

### Triggered by space management tools

Alternatively (or additionally), the space management tools trigger
reload directly when called from within the running server:

- `wiki_spaces_create` → mount the new wiki immediately
- `wiki_spaces_remove` → unmount immediately
- `wiki_spaces_set_default` → update default immediately

This is simpler than file watching and avoids race conditions.

## Implementation considerations

### Shared state

The wiki engine holds a map of mounted wikis (name → wiki handle).
This needs to be behind a `RwLock` or similar:

```rust
struct Engine {
    wikis: RwLock<HashMap<String, WikiHandle>>,
    default_wiki: RwLock<String>,
}
```

Read path (search, list, read) takes a read lock. Mount/unmount
takes a write lock. Contention should be minimal — mount/unmount
is rare.

### Index lifecycle

- Mount: open or create the tantivy index at
  `~/.llm-wiki/indexes/<name>/`
- Unmount: close the index reader/writer handles. Do not delete
  the index files (the wiki might be re-mounted later).
- `wiki_spaces_remove --delete`: also delete index files.

### In-flight requests

What happens to a request targeting a wiki that's being unmounted?

Options:
1. **Reject** — return error "wiki not found" if unmounted mid-request
2. **Complete** — hold a reference to the wiki handle until the
   request finishes (Arc-based)

Option 2 is safer. The wiki handle is `Arc<WikiHandle>`, so
in-flight requests keep it alive even after unmount removes it from
the map.

### File watching vs tool-triggered

| Approach | Pros | Cons |
|----------|------|------|
| File watching | Picks up external edits to config.toml | Needs a watcher (notify crate), debouncing, race conditions |
| Tool-triggered | Simple, no watcher, no races | Only works for changes made through the engine |
| Both | Covers all cases | More complexity |

Recommendation: start with tool-triggered (simpler), add file
watching later if needed.

### Transport impact

All transports share the same engine. A reload is transparent to
transports — they call into the engine, which resolves the wiki
name at request time from the current map.

No transport restart needed. No session interruption.

## Interaction with existing features

### Cross-wiki search

`wiki_search(cross_wiki: true)` iterates all mounted wikis. After
hot reload, it sees the updated set immediately.

### wiki:// URI resolution

`wiki://research/concepts/moe` resolves the wiki name from the
mounted map. If the wiki was unmounted, the URI fails to resolve
with a clear error.

### Index staleness

A newly mounted wiki may have a stale or missing index. Apply the
same staleness check as startup:
- `index.auto_rebuild: true` → rebuild silently
- `index.auto_rebuild: false` → warn

## Open questions

- Should there be a `wiki_spaces_reload` tool for explicit full
  reload, or is tool-triggered sufficient?
- Does `RwLock` need tuning (fair vs unfair) for read-heavy
  workloads?

## Decisions

- **Tool-triggered only** — no file watching. Space management tools
  (`wiki_spaces_create`, `wiki_spaces_remove`, `wiki_spaces_set_default`)
  mount/unmount immediately when called from the running server.
- **`RwLock<HashMap>`** for shared wiki map. Contention is acceptable
  — mount/unmount is rare, readers wait briefly if needed.
- **MCP notification** — emit `notifications/resources/list_changed`
  on reload if the transport supports it. Low cost, agents can
  re-bootstrap if they care.
- **Refuse unmount of default wiki** — same rule as
  `wiki_spaces_remove`: set a new default first.

## Tasks

### 1. Update specifications

- [ ] Update `docs/specifications/engine/server.md`:
  - Add "Hot Reload" section describing tool-triggered mount/unmount
  - Update startup sequence to mention `RwLock`-based wiki map
  - Document MCP notification on wiki set change
  - Update guarantees table (reload does not interrupt transports)
- [ ] Update `docs/specifications/tools/space-management.md`:
  - Document that `wiki_spaces_create` mounts immediately in a
    running server
  - Document that `wiki_spaces_remove` unmounts immediately
  - Document that `wiki_spaces_set_default` updates immediately
- [ ] Update `docs/specifications/tools/overview.md` if tool
  descriptions change

### 2. Refactor engine shared state

- [ ] Wrap the wiki map in `RwLock<HashMap<String, Arc<WikiHandle>>>`
- [ ] Wrap `default_wiki` in `RwLock<String>`
- [ ] All read paths (search, list, read, graph) take a read lock,
  clone the `Arc<WikiHandle>`, release the lock, then operate on
  the handle
- [ ] Verify existing tests pass with the new locking

### 3. Implement mount/unmount

- [ ] `Engine::mount_wiki(name, path)` — open or create tantivy
  index, insert into wiki map under write lock, run staleness check
- [ ] `Engine::unmount_wiki(name)` — remove from wiki map under
  write lock, drop the `Arc` (in-flight requests keep their clone
  alive)
- [ ] `Engine::set_default(name)` — update default under write lock,
  verify name exists in map
- [ ] Refuse unmount if wiki is the current default (return error)

### 4. Wire space management tools

- [ ] `wiki_spaces_create` — after writing `config.toml`, call
  `engine.mount_wiki(name, path)`
- [ ] `wiki_spaces_remove` — after writing `config.toml`, call
  `engine.unmount_wiki(name)`
- [ ] `wiki_spaces_remove --delete` — also delete index files at
  `~/.llm-wiki/indexes/<name>/`
- [ ] `wiki_spaces_set_default` — after writing `config.toml`, call
  `engine.set_default(name)`

### 5. MCP notification

- [ ] After mount/unmount/set-default, emit
  `notifications/resources/list_changed` via the MCP transport
- [ ] If the transport doesn't support notifications (stdio batch),
  skip silently
- [ ] Add test: notification is emitted after `wiki_spaces_create`

### 6. Index lifecycle on mount

- [ ] If index exists at `~/.llm-wiki/indexes/<name>/`, open it
- [ ] If index doesn't exist, create it and run full rebuild
- [ ] Apply staleness check per `index.auto_rebuild` config
- [ ] On unmount, close reader/writer handles but do not delete
  index files

### 7. Tests

- [ ] Unit test: mount a wiki, verify it appears in search
- [ ] Unit test: unmount a wiki, verify search no longer finds it
- [ ] Unit test: refuse unmount of default wiki
- [ ] Unit test: in-flight request completes after unmount
  (Arc keeps handle alive)
- [ ] Unit test: cross-wiki search reflects updated wiki set
- [ ] Integration test: `wiki_spaces_create` + `wiki_search` in
  same server session without restart
- [ ] Existing test suite passes unchanged

### 8. Update skills

- [ ] Update `llm-wiki-skills/skills/spaces/SKILL.md` — mention
  that create/remove take effect immediately in a running server
- [ ] Update `llm-wiki-skills/skills/setup/SKILL.md` — no restart
  needed after creating the first wiki if server is already running

## Success criteria

- `wiki_spaces_create` from a running server makes the new wiki
  immediately searchable without restart
- `wiki_spaces_remove` from a running server unmounts the wiki
  immediately
- In-flight requests to a removed wiki complete without error
- Cross-wiki search reflects the updated wiki set
- No transport restart or session interruption
- Existing tests pass unchanged
