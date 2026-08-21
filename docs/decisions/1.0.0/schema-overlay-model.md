# Schema overlay model: embedded defaults + on-disk overrides

## Decision

Replace the copy-on-create schema model with an overlay model. `spaces::create`
no longer copies embedded schemas into `<wiki>/schemas/`. `space_builder` always
loads embedded defaults first, then merges any files found in `<wiki>/schemas/`
on top. On-disk files are user overrides only; their absence means "use the
engine default".

Add a `wiki migrate` command backed by a SHA manifest (`schemas/manifest.json`,
embedded in the binary) to clean up stock schema copies from existing wikis
without touching user customizations.

Full implementation spec:
[docs/improvements/design-schema-overlay-migration.md](../../improvements/design-schema-overlay-migration.md).

## Context

`spaces::create` copied all embedded schema files into `<wiki>/schemas/` at
wiki creation time. `space_builder::build_space` then loaded schemas exclusively
from disk when that directory existed, ignoring the embedded versions entirely.

This produced two diverging sources of truth. When schemas changed between
releases, existing wikis silently continued using stale on-disk copies. There
was no way to distinguish a user-customized file from an untouched stock copy,
so automated upgrade was impossible without risking data loss.

## Alternatives considered

**Version tag in schema file** — embed `"x-llm-wiki-schema-version": "1.0.0"`
in each stock schema; overwrite if tag is older than current version. Rejected:
a user who edits a tagged file still has the tag; migration would overwrite
their changes.

**Accept divergence and document manual migration** — tell users to diff their
`schemas/` against the release and update manually. Rejected: error-prone,
undiscoverable, and does not scale to a skill-based workflow.

**Full two-phase commit over `wiki.toml`** — a generic transaction layer.
Rejected: disproportionate; the overlay model eliminates the problem class
rather than managing it.

## Why overlay + SHA manifest

**Overlay** eliminates the problem permanently for new wikis and future schema
updates: embedded schemas are always the engine's current defaults; on-disk
files are always the user's choice. No migration is ever needed for a wiki that
was created under this model.

**SHA manifest** solves the one-time transition for existing wikis. It encodes
which on-disk content is "stock" (safe to delete) vs "customized" (must keep)
without requiring a separate install-time database or version metadata in the
files themselves. The manifest is committed alongside the schema files and
embedded in the binary — no runtime I/O, no deployment artifact.

The combination mirrors how package managers handle user-modified config files
(e.g. dpkg conffile handling): known-stock → replace; diverged → warn and skip.

## Scope

- JSON schema files (`.json`) only.
- Template files (`.md` body templates) are out of scope; deferred.
- `wiki migrate` without `--wiki` runs against all registered wikis.
- The migration skill auto-commits deleted files per wiki after a successful
  non-dry-run migration.

## Consequences

- `spaces::create` stops copying schema and template files; `<wiki>/schemas/`
  is created as an empty extension-point directory.
- `space_builder::build_space` merges embedded defaults with on-disk overrides
  on every mount; the branch that chose one or the other is removed.
- `schemas/manifest.json` is added to the repository and must be updated
  (via `make update-schema-manifest` or equivalent) whenever embedded schemas
  change. This becomes part of the schema-change checklist.
- `wiki migrate` / `wiki_migrate` MCP tool are new public surface; their
  JSON output shape is part of the stable 1.0 API contract.
- Tests for `spaces::create` are updated: a newly created wiki has an empty
  `schemas/` directory. Tests for `space_builder` gain overlay coverage
  (on-disk file overrides embedded; absent file uses embedded).
