---
title: "Config Loader Implementation"
summary: "Two-level config loading — GlobalConfig, WikiConfig, resolution order, get/set by key."
status: draft
last_updated: "2025-07-17"
---

# Config Loader Implementation

Implementation reference for config loading and merging. Not a
specification — see
[global-config.md](../specifications/model/global-config.md) and
[wiki-toml.md](../specifications/model/wiki-toml.md) for the design.

## Two Config Files

| File                      | Struct         | Scope                                         |
| ------------------------- | -------------- | --------------------------------------------- |
| `~/.llm-wiki/config.toml` | `GlobalConfig` | Spaces, global defaults, global-only settings |
| `<wiki>/wiki.toml`        | `WikiConfig`   | Wiki identity, per-wiki overrides             |

Both are TOML. Both use serde for deserialization. Missing files
return defaults — never error.

## Structs

### GlobalConfig

All sections present, all fields have defaults via serde:

```rust
struct GlobalConfig {
    global: GlobalSection,      // default_wiki
    wikis: Vec<WikiEntry>,      // space registry
    defaults: Defaults,         // search, page, list defaults
    read: ReadConfig,
    index: IndexConfig,         // global-only
    graph: GraphConfig,
    serve: ServeConfig,         // global-only
    ingest: IngestConfig,
    validation: ValidationConfig,
    logging: LoggingConfig,     // global-only
}
```

### WikiConfig

Overridable sections are `Option` — `None` means "use global default":

```rust
struct WikiConfig {
    name: String,
    description: String,
    defaults: Option<Defaults>,
    read: Option<ReadConfig>,
    validation: Option<ValidationConfig>,
    ingest: Option<IngestConfig>,
    graph: Option<GraphConfig>,
}
```

No `index`, `serve`, or `logging` — those are global-only.

### ResolvedConfig

The merged result — per-wiki wins over global:

```rust
struct ResolvedConfig {
    defaults: Defaults,
    read: ReadConfig,
    index: IndexConfig,
    graph: GraphConfig,
    serve: ServeConfig,
    ingest: IngestConfig,
    validation: ValidationConfig,
}
```

## Resolution

```rust
fn resolve(global: &GlobalConfig, per_wiki: &WikiConfig) -> ResolvedConfig
```

For each overridable section: if `WikiConfig` has `Some(value)`, use
it. Otherwise use the global value. Global-only sections (`index`,
`serve`, `logging`) always come from `GlobalConfig`.

Resolution order (as seen by a tool):

```
1. CLI flag          (handled by clap, overrides resolved config)
2. Per-wiki config   (WikiConfig)
3. Global config     (GlobalConfig)
4. Built-in default  (serde defaults)
```

## Loading

```rust
fn load_global(path: &Path) -> Result<GlobalConfig>
fn load_wiki(wiki_root: &Path) -> Result<WikiConfig>
```

Both return defaults if the file doesn't exist. Never error on missing
file — only on parse failure.

## Saving

```rust
fn save_global(config: &GlobalConfig, path: &Path) -> Result<()>
fn save_wiki(config: &WikiConfig, wiki_root: &Path) -> Result<()>
```

Creates parent directories if needed. Uses `toml::to_string_pretty`.

## Get/Set by Key

For `wiki_config get/set`:

```rust
fn set_global_config_value(global: &mut GlobalConfig, key: &str, value: &str) -> Result<()>
fn set_wiki_config_value(wiki: &mut WikiConfig, key: &str, value: &str) -> Result<()>
```

`set_wiki_config_value` rejects global-only keys (`index.*`, `serve.*`,
`logging.*`) with an error message.

Both use a match on the key string to dispatch to the right field.
Values are parsed from string to the target type (`parse()` for
numbers and bools, direct assignment for strings).

## Existing Code

The current `src/config.rs` is largely reusable:

| Component                   | Reusable | Notes                                                             |
| --------------------------- | -------- | ----------------------------------------------------------------- |
| `GlobalConfig` struct       | yes      | Add new fields: `output_format`, `memory_budget_mb`, `tokenizer`  |
| `WikiConfig` struct         | mostly   | Add `graph: Option<GraphConfig>`, remove `lint` (moved to skills) |
| `ResolvedConfig`            | mostly   | Remove `lint`, add new fields                                     |
| `resolve()`                 | yes      | Add new sections                                                  |
| `load_global` / `load_wiki` | yes      | As-is                                                             |
| `save_global` / `save_wiki` | yes      | As-is                                                             |
| `set_global_config_value`   | yes      | Add new keys                                                      |
| `set_wiki_config_value`     | yes      | Add new keys, remove lint keys                                    |
| `load_schema`               | remove   | `schema.md` eliminated — type registry handles this               |
| `SchemaConfig`              | remove   | Same reason                                                       |
| Default value helpers       | yes      | Add new helpers for new fields                                    |

### Changes needed

- Remove `LintConfig` and `SchemaConfig` (lint moved to skills,
  schema.md eliminated)
- Add `defaults.output_format` (default: `"text"`)
- Add `index.memory_budget_mb` (default: `50`)
- Add `index.tokenizer` (default: `"en_stem"`)
- Add `graph` to `WikiConfig` as overridable
- WikiConfig needs `[types.*]` section for the type registry — but
  that's loaded separately by `SpaceTypeRegistryManager`, not by the
  config loader
