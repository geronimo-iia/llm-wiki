# SpaceContext caches ResolvedConfig at mount time

## Decision

Add `resolved_cfg: ResolvedConfig` to `SpaceContext`. Populate it in
`mount_space` from the `ResolvedConfig` already computed there. Change
`resolved_config()` from a disk-reading method to a field accessor:

```rust
// Before
pub fn resolved_config(&self, global: &GlobalConfig) -> ResolvedConfig { ... }

// After
pub fn resolved_config(&self) -> &ResolvedConfig { &self.resolved_cfg }
```

The `global: &GlobalConfig` parameter is dropped. All ten `src/ops/` call sites
become zero-argument borrows.

## Context

`SpaceContext::resolved_config` was called at every operation boundary — graph
builds, schema validation, export, lint, suggest, search — to obtain the merged
per-wiki config. Its implementation read `wiki.toml` from disk on every call and
merged it with the global config via `config::resolve`.

`mount_space` already computed `resolved_cfg: ResolvedConfig` in full. It
discarded it, storing only `resolved_cfg.ingest` in `ingest_config: IngestConfig`
for a separate hot path in the watcher. Every subsequent operation re-read the
same file to reconstruct the same value.

Under a concurrent read workload (multiple MCP tool calls in flight, each holding
a `SpaceContext` arc) this meant multiple redundant disk reads and `config::resolve`
calls per request cycle with no possibility of the result differing — the file is
only written through `with_config_lock`, which also triggers a space remount.

## Why load errors are not a concern

`mount_space` loads `wiki.toml` with `unwrap_or_default`: if the file is missing
or malformed, the mount proceeds with defaults and the error is logged. A space
that mounts with a degraded config does so consistently for its lifetime — the
field will hold whatever `config::resolve` produced at mount time, including the
default fallback. No silent per-call variance is introduced.

Load errors that are truly fatal (I/O errors beyond missing file) already
propagate through the `mount_space` return type before `SpaceContext` is
constructed. The accessor has no error path to add.

## Why `ingest_config: IngestConfig` is kept

The watcher and write-lock callers in `engine.rs` access `space.ingest_config`
directly while holding the engine write lock. Changing those sites to
`space.resolved_cfg.ingest` would require touching write-lock-holding code with
no benefit — `ingest_config` was a deliberate field for that path. The field
stays; it is now populated as `resolved_cfg.ingest.clone()` in `mount_space`
before the `resolved_cfg` value is moved into the struct.

## Alternatives considered

**Lazy init with `Mutex<Option<ResolvedConfig>>`.** Would defer the disk read
to first use. Rejected — `mount_space` already does the work; lazy init adds a
mutex, an `Option` branch, and the same disk read deferred by a few microseconds.
No benefit.

**Pass `ResolvedConfig` as a parameter to each op function.** Would make the
caching explicit at call sites. Rejected — ten call sites in `src/ops/` would
all grow a `resolved: &ResolvedConfig` parameter that callers must source from
somewhere, pushing the caching decision to every handler. The field approach
centralises it without changing the external API.

**Keep the disk read but add a `Mutex<Option<ResolvedConfig>>` cache.** Same
cache-on-first-use pattern. Rejected for the same reason as lazy init, with the
added downside that invalidation (on remount) would require clearing the cache.

## Consequences

- `resolved_config()` is a zero-cost borrow; no disk I/O, no allocation, no
  lock on the hot path.
- `global: &GlobalConfig` is removed from the signature — ten call sites
  in `src/ops/` are simplified.
- `SpaceContext` struct grows by the size of `ResolvedConfig` (all config
  scalars and strings for one wiki); negligible per-space overhead.
- Config changes take effect on the next remount (same guarantee as before —
  `resolved_config` never reflected mid-lifetime config file edits).
