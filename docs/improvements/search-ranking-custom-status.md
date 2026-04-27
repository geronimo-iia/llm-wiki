---
title: "Flat status multiplier map for search ranking"
summary: "Replace the four named SearchConfig fields with a single status map — uniform, extensible, supports custom status values globally and per wiki."
status: implemented
last_updated: "2026-04-27"
depends_on: search-ranking
---

# Flat status multiplier map for search ranking

## Problem

`SearchConfig` was introduced with four hardcoded fields:

```rust
pub status_active:   f32,   // 1.0
pub status_draft:    f32,   // 0.8
pub status_archived: f32,   // 0.3
pub status_unknown:  f32,   // 0.9
```

This creates two problems:

1. **No custom status support.** Any status not matching `active`, `draft`,
   or `archived` falls through to `status_unknown`. A user who adds `verified`,
   `stub`, `deprecated`, `review`, etc. cannot give it a distinct multiplier.

2. **Split mental model in TOML.** Built-in statuses are top-level scalar
   keys; custom ones would require a separate sub-table
   (`[search.status_custom]`). This is an arbitrary distinction — from the
   user's perspective, all status multipliers are the same thing.

There is also no guide documenting the search ranking system or how to tune it.

## Goal

- **Single flat map** — all status multipliers live in `[search.status]`;
  built-ins and custom entries are written the same way.
- **Extensible** — adding a new status requires only config, not code.
- **Global and per-wiki** — `config.toml` sets defaults; `wiki.toml`
  overrides them for a specific wiki.
- **Breaking change is safe** — `[search]` was just added and no user
  config exists yet.
- Add `docs/guides/search-ranking.md`.

## Solution

### Config — `src/config.rs`

Replace the four named fields with a single map:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_search_status")]
    pub status: std::collections::HashMap<String, f32>,
}

fn default_search_status() -> std::collections::HashMap<String, f32> {
    [
        ("active".into(),   1.0_f32),
        ("draft".into(),    0.8),
        ("archived".into(), 0.3),
        ("unknown".into(),  0.9),
    ]
    .into_iter()
    .collect()
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { status: default_search_status() }
    }
}
```

`unknown` is the reserved fallback key — used when a page's status is absent
or not listed in the map.

TOML representation in `config.toml` or `wiki.toml`:

```toml
[search.status]
active     = 1.0
draft      = 0.8
archived   = 0.3
unknown    = 0.9
# custom entries — same table, no distinction
verified   = 1.0
stub       = 0.6
deprecated = 0.1
review     = 0.85
```

### Lookup in `src/search.rs`

Replace the four-arm `match` with a single map lookup:

```rust
let status_mult = match &status_col {
    Some(col) => match col.term_ords(doc).next() {
        Some(ord) => {
            let mut buf = String::new();
            col.ord_to_str(ord, &mut buf).ok();
            let unknown = sc.status.get("unknown").copied().unwrap_or(0.9);
            sc.status.get(buf.as_str()).copied().unwrap_or(unknown)
        }
        None => sc.status.get("unknown").copied().unwrap_or(0.9),
    },
    None => sc.status.get("unknown").copied().unwrap_or(0.9),
};
```

### Global and per-wiki resolution

Unlike scalar sections (`[suggest]`, `[graph]`, etc.) which are resolved
all-or-nothing, the `status` map is merged **key by key**: the global map
provides the baseline; per-wiki entries override or extend it.

```
resolved[key] = per_wiki[key]  if present
              | global[key]    otherwise
```

This means a `wiki.toml` only needs to declare the entries it wants to
change or add. The four built-in defaults come from `config.toml` (or the
compiled-in defaults) and are inherited automatically.

Example:

```toml
# config.toml (global)
[search.status]
active   = 1.0
draft    = 0.8
archived = 0.3
unknown  = 0.9
```

```toml
# wiki.toml (per-wiki — only declares what differs)
[search.status]
archived = 0.0   # suppress archived for this wiki
stub     = 0.6   # add a custom status
```

Resolved for that wiki:

```toml
active   = 1.0   # inherited from global
draft    = 0.8   # inherited from global
archived = 0.0   # overridden by wiki
unknown  = 0.9   # inherited from global
stub     = 0.6   # added by wiki
```

**Implementation** — replace the current wholesale-clone resolution in
`resolve()` with an explicit map merge:

```rust
search: {
    let mut merged = global.search.status.clone();
    for (k, v) in &per_wiki.search.as_ref()
                            .map(|s| &s.status)
                            .unwrap_or(&Default::default()) {
        merged.insert(k.clone(), *v);
    }
    SearchConfig { status: merged }
},
```

### `config set` / `config get`

`llm-wiki config set` only handles scalar keys today and cannot reach
`search.status.*` map entries. Users must edit `wiki.toml` or `config.toml`
by hand for status entries. This is acceptable — `config set` is for
scalar settings. The gap can be addressed in a future improvement.

### `get_config_value` / `set_global_config_value`

The four existing `search.status_*` key handlers in these functions must be
removed; map entries are not reachable via `config set`.

## Known constraints

- `unknown` is a reserved key name — it is the fallback, not an actual
  `status: unknown` value. A page with `status: unknown` would match the
  `unknown` entry in the map, which is intentional and useful.
- Map keys are case-sensitive (matches YAML frontmatter values).
- Values outside `[0.0, 1.0]` are not clamped. `0.0` effectively suppresses
  a status; values above `1.0` boost it. Both are intentional escape hatches.

## Tasks

### Source code
- [x] `src/config.rs`: replace four named fields on `SearchConfig` with
  `status: HashMap<String, f32>`; `default_search_status()` seeds the four
  built-in defaults; remove `default_search_status_active/draft/archived/unknown`
  helper fns; remove the four `search.status_*` arms from
  `get_config_value` and `set_global_config_value`; replace the
  wholesale-clone `search` line in `resolve()` with a key-level merge
  (global baseline, per-wiki entries override/extend).
- [x] `src/search.rs`: replace the four-arm `match` with a single
  `sc.status.get(buf.as_str())` lookup falling back to the `"unknown"` entry.

### Tests
- [x] `tests/config.rs`: update any existing `SearchConfig` tests for the new
  shape; add round-trip test for TOML with custom entries.
- [x] `tests/search.rs`: update the four existing ranking tests that construct
  `SearchConfig` explicitly (they reference `status_active`, `status_draft`, etc.).
- [x] `tests/search.rs`: add `search_ranking_custom_status_mapped` — index two
  pages `status: stub` and `status: active`; configure `stub = 0.6`; assert
  active (×1.0×0.5=0.5) ranks above stub (×0.6×0.5=0.3).
- [x] `tests/search.rs`: add `search_ranking_custom_status_falls_back_to_unknown`
  — index a page with `status: stub`; no `stub` entry in map; assert it scores
  the same as a page with no status (both use the `unknown` multiplier).
- [x] `tests/config.rs`: add `resolve_search_status_merges_per_wiki` — global
  has `archived = 0.3`; per-wiki sets `archived = 0.0` and adds `stub = 0.6`;
  resolved map contains `active = 1.0` (inherited), `archived = 0.0`
  (overridden), `stub = 0.6` (added), `draft = 0.8` (inherited).

### Spec docs
- [x] `docs/specifications/tools/search.md`: rewrite `[search]` config section —
  replace the four-key scalar table (`status_active`, `status_draft`,
  `status_archived`, `status_unknown`) with the `[search.status]` map syntax;
  document `unknown` as the reserved fallback key; add a custom status example
  (`verified`, `stub`, `deprecated`); update the multiplier table to note that
  the map is extensible; update any TOML example blocks that show the old scalar
  keys.
- [x] `docs/specifications/model/global-config.md`:
  - Replace the four `search.status_active / status_draft / status_archived /
    status_unknown` rows in the overridable defaults table with a single
    `search.status` map entry (type: `{ string → float }`, default: the four
    built-in key/value pairs).
  - Update the `[search]` example block from four scalar keys to a
    `[search.status]` sub-table.
  - In the Resolution Order section (CLI → wiki.toml → config.toml → built-in),
    add a note that `[search.status]` is resolved **key-by-key** (not
    all-or-nothing like other sections): global map is the baseline, per-wiki
    entries override or extend individual keys.
- [x] `docs/specifications/model/wiki-toml.md`:
  - Update the Complete Example to use `[search.status]` sub-table instead of
    four scalar keys.
  - Update the per-wiki overridable settings table: replace the four
    `status_active / status_draft / status_archived / status_unknown` entries
    with a single `[search.status]` row; add a note that only keys that differ
    from the global baseline need to be declared.
  - Add a short prose note below the table explaining the key-level merge: a
    `wiki.toml` `[search.status]` block overrides or adds individual keys; it
    does **not** replace the entire global map.

### Guides
- [x] Create `docs/guides/search-ranking.md` covering:
  - Ranking formula: `final_score = bm25 × status_multiplier × confidence`
  - The `[search.status]` map — all entries on equal footing
  - Built-in defaults table (`active`, `draft`, `archived`, `unknown`)
  - How to set globally in `config.toml`
  - How to override per-wiki in `wiki.toml`
  - How to add custom statuses (`verified`, `stub`, `deprecated`, …)
  - Example: demoting stubs without suppressing them (`stub = 0.6`)
  - Example: suppressing a status entirely (`deprecated = 0.0`)
  - Example: wiki-specific override (a `review` wiki uses `review = 1.0`)
  - Note: `config set` cannot reach map entries; edit TOML directly
- [x] `docs/guides/configuration.md`: add `### Tune search ranking` section
  with a one-liner example and link to `search-ranking.md`.
- [x] `docs/guides/README.md`: add `search-ranking.md` row to the guide index.
</content>
