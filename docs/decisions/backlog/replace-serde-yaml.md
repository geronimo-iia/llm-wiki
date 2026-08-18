---
title: "Replace serde_yaml 0.9 (blocked)"
summary: "Migrate away from abandoned serde_yaml 0.9 — blocked until saphyr-serde reaches a usable release."
status: blocked
date: "2026-08-18"
---

# Replace `serde_yaml 0.9`

## Context

`serde_yaml 0.9` is abandoned upstream. The crate still works but receives no security or maintenance updates. The engine uses it in six files:

| File | Usage |
|------|-------|
| `src/frontmatter.rs` | `from_str`, `to_string`, `Value` type, `Number::from(f64)` |
| `src/markdown.rs` | `Value::String` constructors |
| `src/type_registry.rs` | `Value` import, `to_string`/`from_str` round-trip in `yaml_fm_to_json` |
| `src/index_manager.rs` | `Value` in four function signatures, variant match arms throughout |
| `src/ops/export.rs` | `Value` parameter type in `remaining_frontmatter` |
| `Cargo.toml` | direct dependency `serde_yaml = "0.9"` |

## Migration target

The intended replacement is `saphyr-serde` (successor to `serde_yaml`, actively maintained saphyr YAML ecosystem). The internal `serde_yaml::Value` type would be replaced with `serde_json::Value` throughout — `serde_json` is already a direct dependency and its `Value` enum covers the same data model with renamed variants (`Sequence`→`Array`, `Mapping`→`Object`).

## Why blocked

**`saphyr-serde v0.0.0` is a stub.** As of 2026-08-18, the crate reserves the name on crates.io but ships only `fn add()` — the default `cargo new` placeholder. Neither `from_str` nor `to_string` exist. The crate has no transitive dependencies on `saphyr` itself.

**`serde_yaml2 v0.1.3` is not a viable write path.** This drop-in fork of `serde_yaml 0.9` does have `de::from_str` and `ser::to_string`, but the serializer produces a non-standard YAML format — quoted keys, values on separate lines, broken sequence items:

```yaml
# serde_yaml2 output (broken for frontmatter)
'confidence':
  0.8
'tags':
  - 
   'routing'
  - 
   'scaling'
'title':
  'Mixture of Experts'
```

Writing this format back to Markdown files would corrupt every wiki page on the next `wiki_ingest`. Parse roundtrip is correct, but the output is unusable for human-readable git-tracked files.

## What the fix looks like

Once `saphyr-serde` publishes a release with `from_str` and `to_string` (expected at `0.1.x`), the migration is mechanical:

1. `Cargo.toml`: remove `serde_yaml = "0.9"`, add `saphyr-serde = "0.1"`.
2. All `use serde_yaml::Value` → `use serde_json::Value`.
3. All `serde_yaml::from_str(...)` → `saphyr_serde::from_str(...)`.
4. All `serde_yaml::to_string(...)` → `saphyr_serde::to_string(...)`.
5. All `v.as_sequence()` → `v.as_array()` (two sites in `frontmatter.rs`).
6. All `Value::Sequence` → `Value::Array`, `Value::Mapping` → `Value::Object` (match arms in `index_manager.rs`).
7. `Value::Number(serde_yaml::Number::from(0.5f64))` → `serde_json::json!(0.5)` (one site in `frontmatter.rs:scaffold`).
8. `yaml_fm_to_json` in `type_registry.rs` simplifies to `Ok(serde_json::to_value(fm)?)`.
9. `remaining_frontmatter` in `ops/export.rs` simplifies from `filter_map` + `to_value` to plain `map` + `clone`.

Before using any version of `saphyr-serde`, verify the serializer output matches standard frontmatter format (unquoted string keys, inline scalar values).

## Revisit conditions

- `saphyr-serde` publishes `>= 0.1.0` on crates.io with `from_str` and `to_string`.
- Verify serializer output is human-readable before adopting.
- `serde_yaml 0.9` triggers a cargo-audit advisory that cannot be suppressed.
