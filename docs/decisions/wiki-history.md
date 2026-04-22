# wiki_history — git log for a page

## Decision

Add `wiki_history` as the 17th MCP tool. Returns git commit history
for a specific page via shell `git log`.

## Context

The engine uses git for all mutations but exposes no history. The
only timestamp is `last_updated` in frontmatter — manually
maintained, often stale. LLM agents can't answer "what changed?" or
"is this page fresh?".

## Key decisions

- **Shell `git log`** — not `git2` revwalk. `git log --follow` is
  battle-tested, rename tracking is built-in. `git` is already a
  hard dependency. Keep `git2` for commit, diff, change detection
  where programmatic access matters.
- **`--follow` via config** — `history.follow = true` as global
  default, overridable per wiki. CLI flag `--no-follow` to override.
  Tracks renames across flat→bundle migration.
- **NUL-delimited format** — `--format=%H%x00%aI%x00%s%x00%an`
  avoids ambiguity with commit messages containing special characters.
- **No shell injection** — `Command::new("git")` with `.arg()` per
  argument, no string interpolation.

## Consequences

- 17 tools (was 16)
- Agents can assess page freshness and track session changes
- `wiki_diff` (future) can use commit hashes from history
- Requires `git` on PATH (already a hard dependency)
