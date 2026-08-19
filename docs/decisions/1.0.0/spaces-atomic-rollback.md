# Spaces atomic rollback: direct `state.write()` for `set_default` rollback

## Decision

In `spaces_set_default`, capture `prev_default` before the in-memory mutation
and restore it via direct `state.write()` access on disk failure, bypassing
`engine.set_default()`. In `spaces_create` / `spaces_register`, run `mount_wiki`
and the `spaces::remove` rollback inside the same `with_config_lock` closure.

## Context

Space-mutating operations mutate in-memory state then persist to `wiki.toml`.
If the disk write fails, in-memory state and disk diverge: the engine serves a
default wiki or space that the next restart will not find.

`with_config_lock` serialises concurrent space mutations via
`config_write_lock: Mutex<()>` but does not itself hold `state: RwLock`.
`set_default()` and `mount_wiki` / `remove` each acquire and release
`state.write()` independently — no nested lock risk.

## Alternatives considered

**Call `engine.set_default("")` to reset** — `set_default` validates that the
target wiki exists via `contains_key`. An empty string (no default) fails this
check. Rejected: cannot restore "no default" state through the public method.

**Call `engine.set_default(&prev_default)` where `prev_default` is non-empty**
— works for the non-empty case but not the empty case. Rejected: inconsistent;
the rollback path must handle both.

**Wrap the disk write in a separate transaction layer** — a generic two-phase
commit over `wiki.toml`. Rejected: disproportionate. The operations are already
serialised by `config_write_lock`; a thin rollback capture is sufficient.

**Accept the divergence and fix on next restart** — the engine detects missing
spaces at startup and skips them. Rejected: divergence during a live server run
causes tool errors (`default_wiki` refers to a non-existent space) until restart.

## Why direct `state.write()` for `set_default` rollback

`state.write()` is the canonical mutable access path. `set_default` is a
convenience wrapper that adds validation; rollback does not need validation —
it restores a value that was valid moments before. Direct write is the correct
bypass.

`with_config_lock` guarantees no concurrent writer can observe the intermediate
state between the failed disk write and the rollback completing.

## Why `mount_wiki` + rollback inside the `with_config_lock` closure

Before this fix, `mount_wiki` ran inside the lock but `spaces::remove` rollback
ran after the lock was released. A concurrent `spaces_list` or config read
between `mount_wiki` and `remove` would observe a space that was about to be
rolled back. Moving both into the closure closes this window entirely.

## Consequences

- `spaces_set_default` captures `prev_default: String` before `set_default()`.
  On `atomic_write` failure, writes `prev_default` directly to
  `state.write()?.global.default_wiki`.
- `spaces_create` and `spaces_register` move `mount_wiki` + rollback inside
  the `do_create` / `do_register` closures passed to `with_config_lock`.
- No new synchronization primitives. No change to `with_config_lock` semantics.
- Regression tests: `set_default_rollback_restores_prev_on_disk_failure` and
  `create_rollback_removes_space_on_disk_failure` in `tests/spaces.rs`.
