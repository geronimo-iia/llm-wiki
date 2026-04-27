---
title: "Backlinks"
summary: "Expose incoming links on wiki_content_read via an index query, no file writes."
status: implemented
last_updated: "2026-04-27"
---

# Backlinks

## Problem

The tantivy index already stores `body_links` (all `[[slug]]` references extracted
from each page's body). Incoming links are queryable in aggregate — but they are
invisible to an LLM reading a specific page.

When an agent calls `wiki_content_read` on a page, it sees the page's outgoing links
embedded in the markdown body. It has no way to know what links *to* this page without
issuing a separate `wiki_search` with a `body_links` filter — a non-obvious,
multi-step lookup that no skill currently performs. The agent cannot discover related
pages without additional queries.

## Goal

Make incoming links available as part of a `wiki_content_read` response, without
requiring a separate query and without any file writes.

## Solution

Add a `backlinks: bool` parameter to `wiki_content_read`. When `true`, the response
includes an additional `backlinks` field: an array of `{ slug, title }` objects for
all pages whose `body_links` index field contains the target slug.

```
wiki_content_read(slug: "my-page", backlinks: true)
→ { content: "...", frontmatter: {...}, backlinks: [{ slug: "other", title: "Other Page" }] }
```

Implementation: a targeted tantivy term query on the `body_links` keyword field.
No file writes, no git commits, no index mutation. The index is the source of truth.

**Why not persist backlinks into the `.md` file:**
- Git churn: any link change anywhere causes unrelated page diffs
- Stale data: the injected section is only current as of the last write; the index
  is always current
- Responsibility leak: Hugo and other renderers compute backlinks at build time from
  their own link graph — duplicating this in source files splits ownership across layers
- `[[slug]]` syntax in an injected section is unsafe: `extract_body_wikilinks` scans
  raw body text with no sentinel awareness and would index backlink sources as outgoing
  links, corrupting `body_links` for the page

The index query gives the right answer at the right time with no state to go stale.

## Branch & PR — `llm-wiki`

```bash
git checkout -b feat/backlinks
```

When implementation is complete and all tests pass:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

```bash
git push -u origin feat/backlinks
gh pr create \
  --title "feat: backlinks param on wiki_content_read" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
Implements imp-3 (backlinks).

- Add `backlinks: bool` parameter to `wiki_content_read`
- `backlinks_for` term query on `body_links` tantivy field
- `BacklinkRef { slug, title }` in response when `backlinks: true`

Closes geronimo-iia/llm-wiki#22 (imp-3)

Spec: docs/improvements/backlinks.md
EOF
)"
```

## Branch & PR — `llm-wiki-skills`

```bash
# in llm-wiki-skills repo
git checkout -b feat/backlinks
```

When done:

```bash
git push -u origin feat/backlinks
gh pr create \
  --repo geronimo-iia/llm-wiki-skills \
  --milestone "v0.4.0" \
  --title "feat: document backlinks param in content and research skills" \
  --body "$(cat <<'EOF'
Aligns skill documentation with the backlinks feature added in llm-wiki imp-3.

- `skills/content/SKILL.md`: document `backlinks: true` on `wiki_content_read`
- `skills/research/SKILL.md`: note backlinks as a discovery technique

Companion to llm-wiki feat/backlinks (imp-3).
Closes geronimo-iia/llm-wiki-skills#1 (imp-3)
EOF
)"
```

> Merge timing: skills PR requires `llm-wiki` v0.2.0 imp-3 to be merged first.

## Tasks

### Engine — `llm-wiki` (branch: `feat/backlinks`)

- [x] Add `BacklinkRef { slug: String, title: String }` struct to content ops return types; derive `Serialize`.
- [x] Add `backlinks_for(slug: &str, searcher: &Searcher, is: &IndexSchema) -> Vec<BacklinkRef>` in `src/ops/content.rs`; term query on `body_links` keyword field.
- [x] Add `backlinks: bool` parameter to `wiki_content_read` MCP tool definition in `src/tools.rs`.
- [x] In `handlers::handle_content_read`, when `backlinks: true`, call `backlinks_for` and include the result in the JSON response.
- [x] Update `wiki_content_read` response schema in `docs/specifications/tools/content-operations.md`.
- [x] Unit test: two pages link to a third; `backlinks_for` returns both; a page with no incoming links returns an empty vec.
- [x] Unit test: `backlinks: false` (default) returns no `backlinks` field in the response (no overhead for the common case).

### Skills — `llm-wiki-skills` (branch: `feat/backlinks`)

- [x] `skills/content/SKILL.md`: document `backlinks: true` parameter on `wiki_content_read`; show example response with `backlinks` array.
- [x] `skills/research/SKILL.md`: add backlinks as a discovery technique (when tracing what references a page).
