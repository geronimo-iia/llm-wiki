---
title: "Git Implementation"
summary: "How the engine uses git2 — repo init, commit, diff, HEAD resolution."
status: ready
last_updated: "2025-07-17"
---

# Git Implementation

Implementation reference for git operations via git2. Not a
specification — see
[ingest-pipeline.md](../specifications/engine/ingest-pipeline.md) and
[index-management.md](../specifications/engine/index-management.md)
for the design.

## Operations

The engine needs five git operations:

| Operation    | Used by                                  | Description                                       |
| ------------ | ---------------------------------------- | ------------------------------------------------- |
| Init         | `wiki_spaces_create`                     | Create a new git repo with initial commit         |
| Commit paths | `wiki_content_commit` (by slug)          | Stage specific files, commit                      |
| Commit all   | `wiki_content_commit --all`, auto_commit | Stage everything, commit                          |
| Diff         | `SpaceIndexManager.update`               | Detect changed files for incremental index update |
| HEAD         | `SpaceIndexManager.has_changed`          | Get current HEAD for staleness check              |

## Repo Init

`wiki_spaces_create` initializes a git repo at the wiki path:

1. `git2::Repository::init(path)`
2. Create directory structure (wiki/, inbox/, raw/, schemas/)
3. Write wiki.toml, README.md, schema files
4. Stage all files
5. Create initial commit: `create: <name>`

If the path already exists and is a git repo, skip init. If it exists
but is not a git repo, init in place.

## Commit

Two modes, same underlying flow:

### Commit by slug

Resolve each slug to file paths (flat file, bundle folder, or section
folder). Stage only those paths.

1. For each slug, resolve to disk path(s)
2. `index.add_path()` for each resolved path
3. Write index to tree
4. Create commit with message `commit: <slug>, <slug>`

### Commit all

Stage everything in the working tree.

1. `index.add_all(["*"])` — stages all changes
2. Write index to tree
3. Create commit with message or default `commit: all`

### Auto-commit on ingest

Same as commit all, but with message `ingest: <path> — +N pages, +M
assets`. Triggered by the ingest pipeline when `auto_commit` is true.

### Empty commits

If nothing is staged (no changes), skip the commit. Don't error — it's
a no-op.

## Diff

The index manager needs two diffs to detect changed files for
incremental update:

### Working tree vs HEAD

Catches uncommitted changes — files written by `wiki_content_write`
that haven't been committed yet.

Uses `diff_tree_to_workdir_with_index` — compares HEAD tree against
the working directory, including staged changes.

### Stored commit vs HEAD

Catches committed changes since the last index update — someone
committed outside llm-wiki, or prior ingests moved HEAD.

Uses `diff_tree_to_tree` — compares the tree at `state.toml.commit`
against the tree at HEAD.

### Merging diffs

Both diffs produce a set of changed paths. Merge them into one
`HashMap<PathBuf, DeltaKind>` (added, modified, deleted). Later entries
(working tree) win on duplicates — they reflect the most recent state.

Filter to `.md` files under `wiki/` only. Non-markdown files and files
outside `wiki/` are ignored.

### Edge cases

| Condition                                        | Behavior                               |
| ------------------------------------------------ | -------------------------------------- |
| No HEAD (fresh repo, no commits)                 | Both diffs impossible — full rebuild   |
| No stored commit (first index build)             | Skip diff B, use diff A only           |
| Stored commit not in history (rebase/force-push) | Diff B fails — full rebuild            |
| Both diffs empty                                 | No-op — index is up to date            |
| Renamed file                                     | Delete old slug, insert new slug       |
| Non-`.md` file changed (bundle asset)            | Ignored — only `.md` triggers re-index |

## HEAD Resolution

Simple: `repo.head()?.peel_to_commit()?.id()`. Returns the current
HEAD commit hash as a string for comparison with `state.toml.commit`.

If HEAD doesn't exist (empty repo), return `None` — the caller treats
this as "never indexed."

## Design Decisions

### No git history queries

The engine doesn't expose `wiki_history` or `wiki_diff` tools (those
are in the roadmap as future ideas). Git operations are limited to
what the engine needs internally: init, commit, diff for index update,
HEAD for staleness.

### No remote operations

No push, pull, fetch, or clone. The wiki is a local git repo. Syncing
with remotes is the user's responsibility.

### Commit signatures

Use the git2 default signature — reads from git config
(`user.name`, `user.email`). If not configured, use
`llm-wiki <llm-wiki@localhost>` as fallback.

## Crate

```toml
git2 = "0.19"
```

Reference: https://docs.rs/git2/latest/git2/

## Existing Code

The current `src/git.rs` is reusable with minor changes:

| Function               | Reusable | Notes                                          |
| ---------------------- | -------- | ---------------------------------------------- |
| `init_repo`            | yes      | As-is                                          |
| `commit`               | yes      | Commit-all path                                |
| `commit_paths`         | yes      | Commit-by-slug path                            |
| `current_head`         | yes      | HEAD resolution                                |
| `changed_wiki_files`   | yes      | Working tree vs HEAD diff                      |
| `changed_since_commit` | yes      | Stored commit vs HEAD diff                     |
| `collect_md_changes`   | yes      | Shared filter for both diffs                   |
| `diff_last`            | maybe    | Not needed — replaced by the two-diff approach |

Improvements to make:
- Handle empty commits gracefully (no-op, not error)
- If `diff_last` is not used after implementation, remove it
- Diff merging stays in the index manager, not in this module
