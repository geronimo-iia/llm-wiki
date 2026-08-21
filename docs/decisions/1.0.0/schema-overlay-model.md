# Schema overlay model: embedded defaults + on-disk overrides

## Decision

Replace the copy-on-create schema model with an overlay model. `spaces::create`
no longer copies embedded schemas into `<wiki>/schemas/`. `space_builder` always
loads embedded defaults first, then merges any files found in `<wiki>/schemas/`
on top. On-disk files are user overrides only; their absence means "use the
engine default".

Add a `wiki migrate` command that detects and removes stock schema copies from
existing wikis using parsed JSON equality, leaving genuine user customizations
untouched. `wiki migrate` requires `--wiki <name>` or `--all` explicitly — no
silent all-wikis default.

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

**SHA manifest (`schemas/manifest.json`)** — embed a JSON file mapping each
stock schema filename to its SHA-256 digest per release version; migration
checks on-disk bytes against the manifest. Rejected: SHA-256 of raw bytes is
sensitive to CRLF line endings and trailing whitespace, producing false
"customized" classifications on Windows. Requires a `make update-schema-manifest`
step whenever schemas change, adding friction to the schema-change workflow.

## Why overlay + JSON equality archive

**Overlay** eliminates the problem permanently for new wikis and future schema
updates: embedded schemas are always the engine's current defaults; on-disk
files are always the user's choice. No migration is ever needed for a wiki
created under this model.

**JSON equality** (`serde_json::Value` comparison) detects stock copies without
byte-level fragility. Whitespace, key ordering, and line endings are all
normalized by parsing. A file is stock if its parsed content matches any known
stock version — current or historical.

**Source archive** (`schemas/archive/<version>/`) embeds historical schema
content directly in the binary via `include_str!`. When schemas change in a
future release, the current files are copied to `schemas/archive/<version>/`
before being updated. No manifest file, no CI target, no version metadata in
the schema files themselves. `is_stock_schema(content)` checks against
`all_stock_schema_contents()`, which combines current embedded schemas and all
archived versions.

The archive currently covers one historical set: `schemas/archive/pre-1.0.0/`
(five files: base, concept, doc, paper, section — retrieved from git tag
`v0.5.9`; skill.json was unchanged and is not archived).

The combination mirrors how package managers handle user-modified config files
(e.g. dpkg conffile handling): known-stock → replace; diverged → warn and skip.

## Scope

- JSON schema files (`.json`) only.
- Template files (`.md` body templates) are out of scope; deferred.
- `wiki migrate` requires `--wiki <name>` or `--all` (no silent all-wikis
  default — running against all wikis without explicit opt-in is a footgun).
- The migration skill auto-commits deleted files per wiki after a successful
  non-dry-run migration.

## Consequences

- `spaces::create` stops copying schema and template files; `<wiki>/schemas/`
  is created as an empty extension-point directory (`.gitkeep` only).
- `space_builder::build_space` merges embedded defaults with on-disk overrides
  on every mount; the branch that chose one or the other is removed.
- `compute_disk_hashes` in `type_registry.rs` applies the same overlay logic to
  stay in sync with `build_space` (discovered during implementation: empty
  `schemas/` dir produced a different hash with the old disk-only path).
- When schemas change in a future release: copy current schema files to
  `schemas/archive/<version>/`, add `include_str!` constants and push them onto
  `all_stock_schema_contents()` in `src/default_schemas.rs`.
- `wiki migrate` / `wiki_migrate` MCP tool are new public surface; their
  JSON output shape is part of the stable 1.0 API contract.
- Tests for `spaces::create` are updated: a newly created wiki has an empty
  `schemas/` directory. Tests for `space_builder` gain overlay coverage
  (on-disk file overrides embedded; absent file uses embedded).
