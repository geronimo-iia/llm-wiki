---
title: "Design: Schema Overlay Model and Migration Command"
summary: "Replace the copy-on-create schema model with an overlay model (embedded defaults + on-disk overrides) and add a wiki migrate command backed by a SHA manifest to clean up stock schema copies from existing wikis."
read_when:
  - Implementing the overlay schema loader in space_builder.rs
  - Implementing wiki migrate / spaces migrate
  - Implementing the schema SHA manifest and update tooling
  - Updating spaces::create to stop copying schemas
  - Writing a migration skill
status: proposal
last_updated: "2026-08-21"
---

# Design: Schema Overlay Model and Migration Command

## Problem

When a wiki is created with `spaces create`, `spaces::create` in `src/spaces.rs`
copies all embedded schema files from `schemas/` into `<wiki>/schemas/`. This
produces two sources of truth for schema content:

1. The **embedded schemas** in the binary (`src/default_schemas.rs`, via
   `include_str!`).
2. The **on-disk copies** in `<wiki>/schemas/`.

`space_builder::build_space` currently loads schemas exclusively from disk if
`<wiki>/schemas/` exists, and falls back to embedded only when the directory is
absent.

This creates a maintenance problem at upgrade time:

- Stock schema files in existing wikis are stale copies of what was current at
  creation time.
- When schemas change between releases (as they did for v1.0.0), existing wikis
  silently continue using the old files.
- There is no way to tell whether a file in `<wiki>/schemas/` is an untouched
  stock copy or a user customization.
- Overwriting on upgrade risks destroying user customizations. Skipping risks
  leaving stale schemas.

## Goals

1. **Eliminate the divergence** — embedded schemas are always the source of
   truth for stock types; on-disk files are always user customizations.
2. **Preserve user customizations** — files in `<wiki>/schemas/` that differ
   from any known stock version are never touched by tooling.
3. **Clean up existing wikis** — a `wiki migrate` command removes on-disk
   copies of stock schemas that are now redundant, leaving only genuine
   overrides.
4. **Keep future schema updates free** — changing an embedded schema requires
   no migration logic; users get the update automatically on next server start.

## Non-Goals

- Schema validation or type system changes.
- Versioning of user-defined schemas.
- Remote schema distribution.

## Solution Overview

Two independent changes, applied together:

### A — Overlay loader in `space_builder`

Change `build_space` to always start from embedded defaults and then layer
on-disk files on top, rather than choosing one or the other:

```
effective schema = embedded_default  ←  overridden by  ←  <wiki>/schemas/<file>
```

If `<wiki>/schemas/concept.json` exists it replaces the embedded `concept.json`.
If it does not exist the embedded version is used. Files in `<wiki>/schemas/`
that have no embedded counterpart are treated as new user-defined types and
loaded as-is (current behaviour for custom types is preserved).

This makes `<wiki>/schemas/` a pure **override directory**: presence means
"replace this type's schema with mine"; absence means "use the engine default".

### B — SHA manifest + `wiki migrate` command

A file `schemas/manifest.json` is committed in the repository alongside the
schema files. It maps each stock schema filename to its SHA-256 digest, grouped
by release version:

```json
{
  "1.0.0": {
    "base.json":    "sha256:...",
    "concept.json": "sha256:...",
    "doc.json":     "sha256:...",
    "paper.json":   "sha256:...",
    "section.json": "sha256:...",
    "skill.json":   "sha256:..."
  },
  "0.x": {
    "concept.json": "sha256:...",
    ...
  }
}
```

The manifest is embedded in the binary via `include_str!` (same mechanism as
the schemas themselves) so no runtime file I/O is required to read it.

`wiki migrate` walks `<wiki>/schemas/` for each registered wiki and for each
file:

- Computes the SHA-256 of the on-disk content.
- Looks up the SHA across **all versions** in the manifest.
- **Match found** → file is an unmodified stock copy → delete it (the overlay
  loader will now use the embedded version, which is up to date).
- **No match** → file has been customized → leave it untouched; log it as
  "kept (custom override)".

After migration, `<wiki>/schemas/` contains only genuine user overrides. Future
schema updates in new releases require no migration — the overlay loader picks
them up automatically.

## `spaces::create` change

Stop copying schemas to `<wiki>/schemas/` at creation time. The directory is
still created (it is a documented extension point), but it is left empty. Users
who want to override a schema place their file there; the overlay loader handles
the rest.

Template files (`.md` body templates) follow the same rule: embedded defaults,
on-disk overrides, no copy at creation.

## Technical Details

### Manifest file: `schemas/manifest.json`

Format:

```json
{
  "<version-label>": {
    "<filename>": "sha256:<hex>"
  }
}
```

Version labels are arbitrary strings (`"1.0.0"`, `"0.x"`). Migration code
collects all known SHAs across all version labels into a flat set and checks
membership — version labels are informational only.

SHA-256 is computed over the raw file bytes (no trailing-newline normalisation).

### Manifest update workflow

A helper target regenerates the manifest whenever schemas change:

```
make update-schema-manifest
# or
cargo xtask update-schema-manifest
```

The target:
1. Reads every `.json` file in `schemas/`.
2. Computes SHA-256 of each.
3. Writes / updates the `"<current-version>"` key in `schemas/manifest.json`.
4. Leaves older version keys intact (they are needed for migration of older
   wikis).

Updating the manifest is part of the schema-change checklist: change schema →
run updater → commit both files together.

### Overlay loader (`src/space_builder.rs`)

Current logic (simplified):

```rust
if schemas_dir.is_dir() {
    parse_from_dir(&schemas_dir, repo_root)
} else {
    parse_from_embedded()
}
```

New logic:

```rust
// Always start from embedded defaults
let mut schemas = embedded_schema_map();  // HashMap<filename, content>

// Layer on-disk overrides on top
if schemas_dir.is_dir() {
    for entry in read_dir(&schemas_dir)? {
        if entry has .json extension {
            schemas.insert(filename, fs::read_to_string(entry.path())?);
        }
    }
}

parse_schemas(schemas, repo_root)
```

User files that share a name with an embedded schema replace it. User files
with a new name add a new type. Embedded schemas with no on-disk counterpart
are used as-is.

### `wiki migrate` command

CLI surface:

```
wiki migrate [--wiki <name>] [--dry-run] [--format text|json]
```

- Without `--wiki`: runs against all registered wikis.
- `--dry-run`: reports what would be deleted/kept without modifying anything.
- `--format json`: machine-readable output for the migration skill.

JSON output shape:

```json
{
  "wikis": [
    {
      "name": "research",
      "deleted": ["concept.json", "doc.json"],
      "kept_custom": ["my-type.json"],
      "already_clean": false
    }
  ]
}
```

MCP tool: `wiki_migrate` with the same parameters, added alongside existing
space-management tools.

### Migration skill

A dedicated skill (`wiki-migrate`) wraps the command for guided execution:

1. Run `wiki migrate --dry-run --format json` and present the report.
2. If `kept_custom` is non-empty, show the list and confirm the user understands
   those files will remain as overrides.
3. If `deleted` is non-empty, confirm before running for real.
4. Run `wiki migrate --format json`.
5. For each wiki with deleted files, commit the result:
   `git add schemas/ && git commit -m "chore: remove redundant stock schema copies"`.

### Embedded manifest (`src/default_schemas.rs`)

```rust
const SCHEMA_MANIFEST: &str = include_str!("../schemas/manifest.json");

pub fn stock_schema_shas() -> std::collections::HashSet<String> {
    let manifest: serde_json::Value =
        serde_json::from_str(SCHEMA_MANIFEST).expect("manifest.json is not valid JSON");
    let mut shas = std::collections::HashSet::new();
    if let Some(versions) = manifest.as_object() {
        for (_version, files) in versions {
            if let Some(files) = files.as_object() {
                for (_name, sha) in files {
                    if let Some(s) = sha.as_str() {
                        shas.insert(s.to_owned());
                    }
                }
            }
        }
    }
    shas
}
```

Migration code computes `sha256hex(file_bytes)` and checks
`stock_schema_shas().contains(&digest)`.

## Migration path for existing wikis

| Wiki state | After `wiki migrate` | After next server start |
|---|---|---|
| Stock schemas only (unmodified) | All stock files deleted | Embedded schemas used — up to date |
| Mix of stock + custom | Stock files deleted, custom files kept | Embedded defaults + custom overrides |
| Custom schemas only | Nothing deleted | Custom overrides applied on top of embedded defaults |
| Empty `schemas/` dir | Nothing to do | Embedded schemas used |

## Files affected

| File | Change |
|---|---|
| `schemas/manifest.json` | New — committed SHA manifest |
| `src/default_schemas.rs` | Add `stock_schema_shas()` and `SCHEMA_MANIFEST` |
| `src/spaces.rs` | Remove schema/template copy loop in `create()` |
| `src/space_builder.rs` | Replace branch with overlay merge logic |
| `src/ops/` | New `migrate.rs` — `wiki_migrate` operation |
| `src/cli.rs` | New `migrate` subcommand |
| `src/mcp/tools.rs` | New `wiki_migrate` tool definition |
| `src/mcp/handlers.rs` | New `handle_wiki_migrate` handler |
| `Makefile` or `xtask` | `update-schema-manifest` target |
| `tests/spaces.rs` | Update creation tests — no schemas in new wiki |
| `tests/space_builder.rs` | Add overlay tests |
| `tests/migrate.rs` | New — stock detection, dry-run, delete, keep-custom |

## Decisions

- **`--all-wikis` default**: `wiki migrate` without `--wiki` runs against all
  registered wikis. Explicit `--wiki <name>` scopes to one.
- **Template files** (`.md`): out of scope. Template overlay and migration are
  deferred; `.md` files in `<wiki>/schemas/` are not touched by `wiki migrate`.
- **Git commit step**: the migration skill automatically commits deleted files
  in each affected wiki repo after a successful (non-dry-run) migration:
  `git add schemas/ && git commit -m "chore: remove redundant stock schema copies"`.
