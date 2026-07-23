---
title: "Expose search.status multipliers via config set/get"
summary: "search.status.<key> is now a valid dot-notation path in config set/get, enabling CLI access to search ranking multipliers."
status: accepted
date: "2026-07-23"
---

# Expose search.status multipliers via config set/get

## Origin

Ported from [como-technologies/llm-wiki#6](https://github.com/como-technologies/llm-wiki/pull/6). Applied cleanly with no conflicts.

## Decision

`config set search.status.<key> <value>` and `config get search.status.<key>` now accept any status string as the map key. The `[search.status]` map already existed in `GlobalConfig` and `WikiConfig`; this change wires the CLI path through to it.

## Context

Before this, `config set search.status.archived 0.1` was rejected with "unknown key". Users had to edit `config.toml` or `wiki.toml` directly to tune search ranking by status. Custom statuses (e.g. `superseded`, `deprecated`) were already valid in the TOML files but unreachable via the CLI.

## Rationale

- The underlying map already supported arbitrary keys — the CLI just didn't expose them
- Consistent with how other config keys work: no special cases, same `--global`/`--wiki` scope rules
- Empty key (`search.status.`) and non-numeric values are rejected at parse time

## Implementation

`search_status_key(key)` helper strips the `search.status.` prefix and rejects empty remainders. Called from the `_` arm of `set_global_config_value`, `set_wiki_config_value`, and `get_config_value` before falling through to the "unknown key" error.
