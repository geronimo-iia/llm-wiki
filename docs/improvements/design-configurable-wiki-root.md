---
title: "Design: Configurable Wiki Root"
summary: "Add wiki_root to wiki.toml so repos with non-standard content directories (e.g. llm-wiki-skills) can register as wiki spaces without restructuring."
read_when:
  - Implementing wiki_root support in the engine
  - Understanding what changes are needed in specs and guides
status: proposal
last_updated: "2026-05-01"
---

# Design: Configurable Wiki Root

**Motivation:** Allow repos where the content directory is not `wiki/` — enabling `llm-wiki-skills` (and similar plugin/tool repos) to register as a wiki space without restructuring their layout.

## Problem

The engine hardcodes `wiki/` as the content root. Every path derivation, slug resolution, ingest pipeline, search index, and `wiki_content_*` tool assumes content lives at `<repo>/wiki/`.

This makes it impossible to register repos with a different content layout as wiki spaces without moving their files. Concrete case: `llm-wiki-skills` uses `skills/*/SKILL.md`. Renaming to `wiki/skills/` breaks the Claude plugin path convention and requires updating downstream consumers.

## Proposed Change

Add an optional `wiki_root` key to `wiki.toml`:

```toml
name        = "llm-wiki-skills"
description = "Skills for the llm-wiki engine"
wiki_root   = "skills"           # default: "wiki"
```

When absent, behavior is unchanged. When present, the engine resolves all content paths relative to `<repo>/<wiki_root>/` instead of `<repo>/wiki/`.

## Impact Analysis

### Engine internals

| Area | Impact |
|------|--------|
| `SpaceContext` / space registry | Stores resolved `wiki_root` path at registration. All downstream operations read from `SpaceContext`, not from a hardcoded string. Low blast radius if `SpaceContext` already centralizes path resolution. |
| `wiki_content_*` tools | Use `SpaceContext.wiki_root()` — no logic change, just path source. |
| `wiki_ingest` | Same — path prefix swap. |
| `wiki_search` / `wiki_list` | Index stores slugs relative to `wiki_root`. Rebuilding index on a non-default `wiki_root` must strip the correct prefix. |
| `wiki_graph` | Edges are slugs — correct if ingest uses the right prefix. |
| `wiki_index_rebuild` | Must walk `<repo>/<wiki_root>/` not `<repo>/wiki/`. |
| `wiki_spaces_create` | Should NOT create `wiki/` when `wiki_root` is provided in an existing repo. |
| `wiki_spaces_create` (new repo) | When `--wiki-root` flag given, create `<wiki_root>/` instead of `wiki/`. |

### Slug resolution

Slugs are relative to `wiki_root`. A page at `skills/bootstrap/SKILL.md` in a repo with `wiki_root = "skills"` gets slug `bootstrap`. This is consistent with how `wiki/concepts/foo.md` gets slug `concepts/foo`.

No change to the slug format or `wiki://` URI scheme. The space registry already maps space name → repo path; it will now also carry `wiki_root`.

### `inbox/` and `raw/`

These are sibling directories to `wiki_root`, not children of it. They remain at the repo root regardless of `wiki_root`. A repo with `wiki_root = "skills"` still has `inbox/` and `raw/` at the top level.

The DKR three-layer flow (`inbox` → `raw` → wiki content) is optional — repos that don't use it simply leave those directories empty or absent. The engine should not require their existence.

### `schemas/`

Schemas are discovered from `<repo>/schemas/` — always relative to the repo root, not `wiki_root`. No change.

### `wiki_spaces_create` and `wiki_spaces_register`

Three cases:

1. **New repo** — `llm-wiki spaces create <path> --name <n> --wiki-root <dir>` creates `<wiki_root>/` instead of `wiki/`. Writes `wiki_root` into `wiki.toml`.
2. **Existing repo via create** — reads `wiki.toml` if present and honors `wiki_root`. Does not create or rename directories.
3. **Existing repo via register** — `llm-wiki spaces register <path> --name <n> [--wiki-root <dir>]` registers a pre-existing repo. `--wiki-root` overrides whatever is in `wiki.toml` (or sets it if absent). The engine validates that `<repo>/<wiki_root>/` exists on disk before completing registration — hard error if not found.

### Backward compatibility

`wiki_root` absent → defaults to `"wiki"` → zero behavior change for all existing wikis.

## `wiki.toml` change

Add to identity section:

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `wiki_root` | no | `"wiki"` | Content directory relative to repo root. Supports multi-component paths (e.g. `"src/wiki"`). |

Constraints:
- Must be a relative path (no leading `/`)
- Must not contain `..` (no traversal outside repo root)
- Must not resolve to `inbox`, `raw`, or `schemas` (reserved top-level dirs)
- Must not start with `.`
- Engine validates the directory exists at registration time — hard error if missing

## `wiki_spaces_create` CLI change

Add optional flag to both `create` and `register`:

```
llm-wiki spaces create   <path> --name <name> [--wiki-root <dir>]
llm-wiki spaces register <path> --name <name> [--wiki-root <dir>]
```

`create`: writes `wiki_root = "<dir>"` into the generated `wiki.toml` and creates `<dir>/` instead of `wiki/`.

`register`: writes `wiki_root = "<dir>"` into `wiki.toml` (creating or updating the field) and validates the directory exists before completing.

## Implementation Sketch

1. **Parse** `wiki_root` from `wiki.toml` at space registration time. Store on `SpaceContext` as `wiki_root: PathBuf` (resolved to absolute).
2. **Replace** all `repo_root.join("wiki")` occurrences with `space.wiki_root()`.
3. **`wiki_spaces_create`**: accept `--wiki-root`, pass through to `wiki.toml` generation and directory creation.
4. **`wiki_index_rebuild`**: walk `space.wiki_root()`.
5. **Tests**: add a fixture wiki with `wiki_root = "content"` to the integration test suite. Run search, graph, ingest, content ops against it.

## wiki_root Validation (sub-path check)

`wiki_root` must resolve to a path strictly inside the repository root.
The check must use canonicalized absolute paths to handle symlinks:

```rust
let repo_abs = fs::canonicalize(&repo_path)?;
let root_abs = fs::canonicalize(repo_path.join(&wiki_root))?;
if !root_abs.starts_with(&repo_abs) {
    return Err("wiki_root must be inside the repository");
}
```

Validation runs at registration time (`spaces create` and `spaces register`).
The directory must exist before `canonicalize` succeeds — no separate existence check needed, `canonicalize` fails if the path is missing.

Additional pre-canonicalize checks (fast-fail before filesystem access):
- `wiki_root` must not be an absolute path
- `wiki_root` must not contain `..` components
- `wiki_root` must not be empty or `"."`
- Resolved top-level component must not be `inbox`, `raw`, or `schemas`

## Conflict on `spaces register`

If `--wiki-root` is passed to `register` and `wiki.toml` already contains a different `wiki_root`, the engine errors:

```
error: wiki.toml already declares wiki_root = "skills".
       Remove it manually before registering with a different value.
```

No `--force` flag. The user must edit `wiki.toml` explicitly — prevents silent data loss on a mis-typed flag.

## Documents to Update

### Specs (normative — must be updated before implementation ships)

| File | What changes |
|------|-------------|
| `specifications/model/wiki-repository-layout.md` | `wiki/` → `<wiki_root>/` throughout; add `wiki_root` to layout diagram; note default is `"wiki"` |
| `specifications/model/wiki-toml.md` | Add `wiki_root` to identity section with type, default, constraints |
| `specifications/model/page-content.md` | Slug definition: relative to `wiki_root`, not hardcoded `wiki/` |
| `specifications/engine/ingest-pipeline.md` | "walks `wiki/` recursively" → "walks `wiki_root` recursively" |
| `specifications/engine/watch.md` | `wiki/` path reference → `wiki_root` |
| `specifications/tools/space-management.md` | Add `--wiki-root` flag to `spaces create` and `spaces register`; document validation |
| `specifications/tools/content-operations.md` | `wiki_root` field in responses already present — just clarify it reflects configured value, not always `wiki/` |

### Guides (user-facing — update alongside or just after)

| File | What changes |
|------|-------------|
| `guides/getting-started.md` | Layout diagram shows `wiki/` — add note that it's configurable |
| `guides/writing-content.md` | `wiki_root` examples show hardcoded `/…/wiki` path — stays correct, just derived from config |
| `guides/custom-types.md` | Any path examples using `wiki/` |

### Implementation notes (update when code changes)

| File | What changes |
|------|-------------|
| `implementation/engine.md` | `SpaceContext.wiki_root` field — document it is now read from `wiki.toml`, not hardcoded |
| `implementation/index-manager.md` | `wiki_root` parameter origin — from `SpaceContext`, not assumed `wiki/` |
| `implementation/mcp-tool-pattern.md` | Comment `space.wiki_root // PathBuf — wiki/ directory` → remove `wiki/` assumption |

### Overview (last — reflects the settled design)

| File | What changes |
|------|-------------|
| `overview.md` | Repository layout section: note `wiki/` is the default `wiki_root`, not a fixed name |

### MCP server (update alongside engine changes)

| Area | What changes |
|------|-------------|
| `wiki_spaces_create` tool schema | Add optional `wiki_root` parameter — string, default `"wiki"` |
| `wiki_spaces_register` tool | **New tool** — `spaces register` does not exist today. Must be added: registers an existing repo without creating files. Parameters: `path`, `name`, optional `description`, optional `wiki_root`. Validates `wiki_root` exists and is inside repo before writing to registry. Hot-mounts the wiki if server is running. |
| `wiki_spaces_list` response | Add `wiki_root` field to each space entry (text and JSON output) |
| `SpaceContext` hot-reload | `wiki_root` is read at mount time — changing it requires unmount + remount (same as today for path changes) |

`specifications/tools/space-management.md` must document both the new `--wiki-root` flag and the new `spaces register` subcommand.

### No change needed

- `design-origins/` — historical, not normative
- `decisions/` — record past decisions, not affected
- `diagrams.md` — `wiki_root` already appears as a field name in the diagram, semantics unchanged
- `specifications/tools/export.md` — "relative to wiki root" language already correct
- `testing/validate-skills.md` — test checklist, update separately after implementation

## Use Case Validation

| Repo | `wiki_root` | Outcome |
|------|-------------|---------|
| Standard wiki | `"wiki"` (default) | Unchanged |
| `llm-wiki-skills` | `"skills"` | Plugin path unchanged, engine indexes `skills/*/SKILL.md` |
| Docs repo with `docs/` as content | `"docs"` | `docs/*.md` searchable as wiki pages |
| Monorepo with `knowledge/` | `"knowledge"` | No restructuring needed |
| Monorepo with `src/wiki/` | `"src/wiki"` | Multi-component path, resolves to `<repo>/src/wiki/` |
