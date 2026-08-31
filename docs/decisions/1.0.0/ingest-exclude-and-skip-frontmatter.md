# Ingest filtering: exclude patterns and skip-no-frontmatter

## Decision

Add two optional filters to `[ingest]` configuration that apply at all three
index call sites (`rebuild`, `update`, `rebuild_types`):

**`exclude: Vec<String>`** (default: `[]`) — gitignore-style glob patterns
matched against the slug (path relative to `wiki_root`). Matching files are
skipped during indexing but remain on disk and accessible via
`wiki_content_read` and `wiki_list`.

**`skip_no_frontmatter: bool`** (default: `true`) — skip `.md` files that
have no YAML frontmatter opening (`---`). The check is
`!content.trim_start().starts_with("---")`, not `frontmatter.is_empty()`.

Both filters are implemented in a shared `should_index(slug, content, config,
globset) -> bool` helper in `src/index_manager.rs` to prevent the three call
sites from diverging.

`skip_no_frontmatter = true` is a **behaviour change** for existing wikis that
have bare `.md` files under `wiki_root`. Documented in release notes.

## Context

The WalkDir walk had no path filtering beyond `.md` extension. Every `.md`
file under `wiki_root` was indexed unconditionally. Common cases this blocked:

- `drafts/` or `wip/` subdirectories not ready for search
- Auto-generated pages that pollute BM25 ranking and lint results
- `README.md`, changelogs, and template files that lack frontmatter — silently
  indexed under the `default` type with no `title` or `type`, producing noise
  in search results and spurious `missing-fields` lint findings

## Alternatives considered

**`.llmwikiignore` file** — a gitignore-style file at the wiki root, parsed by
the `ignore` crate (which wraps WalkDir with gitignore support). Rejected for
1.0.0: adds file-watching complexity (changes to `.llmwikiignore` must trigger
a rebuild) and a new config surface outside `wiki.toml`. Deferred.

**`ignore` crate instead of `globset`** — `ignore` combines WalkDir with
gitignore natively. Rejected: the surface area is larger than needed, and the
three call sites are not uniform WalkDir loops (the `update` path iterates a
git diff list, not a directory). `globset` pairs with the existing WalkDir
calls without restructuring them.

**Inclusion patterns (whitelist model)** — only index files matching a pattern.
Rejected: the default-include model covers all known use cases; a whitelist
would require every wiki to enumerate its own content, which is friction for
zero benefit in most cases.

**`wiki_list` / `wiki_content_read` exclusion** — apply the same filter to
read operations. Rejected: excluded files are still valid pages a user or tool
may want to inspect; exclusion applies to index coverage (search, lint, stats),
not to file access. If full exclusion from all tools is needed, that is a
separate, larger item.

## Design choices

**Match against slugs, not absolute paths** — patterns are portable across
machines. A pattern written on a developer's workstation works identically on
CI and in Docker.

**`should_index` placement in `update`** — the `update` loop deletes the
existing index entry before checking `should_index`. This is intentional: if a
file was previously indexed and is now excluded (or frontmatter was removed),
the stale entry must be removed. The delete runs unconditionally; the add is
gated by `should_index`.

**`open()` recovery path** — `SpaceIndexManager::open` has an internal recovery
branch that calls `rebuild` when the index is corrupt. The recovery tuple is
extended to carry `&IngestConfig` so the recovery rebuild applies the same
filters as a normal rebuild.

**No-frontmatter check** — `!content.trim_start().starts_with("---")` rather
than `frontmatter.is_empty()`. `frontmatter::parse` returns empty frontmatter
for both the no-`---` case and malformed YAML; the latter already emits a
`tracing::warn` and should not be silently skipped as if it were a README.
The raw content check distinguishes the two cases without changing
`frontmatter::parse` behaviour (callers outside the index walk rely on
passthrough).

## Scope

- Filtering applies to indexing only: `rebuild`, `update`, `rebuild_types`,
  and the `open()` recovery path.
- `wiki_list` and `wiki_content_read` are unaffected.
- `.llmwikiignore` file support is deferred.
- Inclusion patterns (whitelist) are out of scope.
