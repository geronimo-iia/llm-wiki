# Study: wiki_history — git log for a page

Add a `wiki_history` tool that returns the git commit history for a
specific page. Enables trust assessment ("is this stale?"), session
tracking ("what did I add last session?"), and change auditing.

## Current state

The engine uses git for commits (`wiki_content_commit`, auto-commit
on ingest) but exposes no history. The only timestamp is
`last_updated` in frontmatter — manually maintained, often stale.

`git.rs` uses `git2` (libgit2 bindings) for all git operations:
commit, change detection, HEAD resolution. No history/log function
exists yet.

## Decisions

- **Shell out to `git log`** — not `git2` revwalk. `git log --follow`
  is battle-tested, rename tracking is built-in. `git` is already a
  hard dependency. Keep `git2` for commit, diff, change detection.
- **`--follow` via config** — `history.follow = true` as global
  default, overridable per wiki. Tracks renames (flat→bundle
  migration). CLI flag `--no-follow` to override.
- **Bundles** — log the `index.md` file. Asset changes are not
  included (they don't affect page content).
- **Sections** — log the section's `index.md`.
- **Limit** — `history.default_limit = 10` in config.

## Proposed behavior

### CLI

```
llm-wiki history <slug|uri>
            [--limit <n>]           # default: from config
            [--no-follow]           # disable rename tracking
            [--format <fmt>]        # text | json
            [--wiki <name>]
```

### MCP

```json
{
  "slug": "concepts/moe",
  "limit": 10,
  "follow": true
}
```

### Response (text)

```
a3f9c12  2025-07-21  ingest: concepts/moe.md
b7e4d56  2025-07-18  create: research
```

### Response (JSON)

```json
{
  "slug": "concepts/moe",
  "entries": [
    {
      "hash": "a3f9c12",
      "short_hash": "a3f9c12",
      "date": "2025-07-21T14:32:01Z",
      "message": "ingest: concepts/moe.md",
      "author": "Jerome Guibert"
    }
  ]
}
```

## Implementation: shell out to git log

Use `git log` for history — simplest approach, `--follow` works out
of the box, and `git` is already a hard dependency (the wiki is a
git repo). Keep `git2` for everything else (commit, diff, change
detection) where programmatic access matters.

```rust
// Pseudocode
let mut cmd = Command::new("git");
cmd.current_dir(repo_root)
    .args(["log", "--format=%H%x00%aI%x00%s%x00%an"]);
if follow {
    cmd.arg("--follow");
}
if limit > 0 {
    cmd.args(["-n", &limit.to_string()]);
}
cmd.args(["--", &rel_path]);

let output = cmd.output()?;
// Parse NUL-delimited fields per line
```

`--format=%H%x00%aI%x00%s%x00%an` outputs hash, ISO date, subject,
author separated by NUL bytes — safe parsing, no ambiguity with
commit messages containing special characters.

## Interaction with existing features

- `wiki_diff` (future) — uses commit hashes from history
- Bootstrap — could check recent history to report activity
- Crystallize — reference last commit to avoid duplicating work
- Slug resolution — same `Slug::from_path` / `WikiUri::resolve`
  as other tools

## Open questions

- Should history include commits from before the page was ingested
  (e.g. manual git commits)? (Yes — all commits that touch the file)
- Max limit cap to prevent huge responses? (e.g. hard cap at 100)

## Tasks

### 1. Update specifications

- [x] Create `docs/specifications/tools/history.md`
- [x] Update `docs/specifications/model/global-config.md` — add
  `history.follow` and `history.default_limit`
- [x] Update `docs/specifications/tools/overview.md` — add
  `wiki_history` (17 tools)

### 2. Config

- [x] `src/config.rs` — add `HistoryConfig { follow, default_limit }`
- [x] Add to `GlobalConfig`, `WikiConfig` (optional), `ResolvedConfig`
- [x] Wire get/set for `history.follow` and `history.default_limit`

### 3. Git history

- [x] `src/git.rs` — add `HistoryEntry { hash, date, message, author }`
- [x] `src/git.rs` — add `page_history(repo_root, rel_path, limit, follow)`
- [x] Implement via `Command::new("git")` with `--format=%H%x00%aI%x00%s%x00%an`
- [x] Parse NUL-delimited output into `HistoryEntry` vec
- [x] `--follow` passed to `git log` when enabled

### 4. Ops layer

- [x] `src/ops/history.rs` — resolve slug to file path, call
  `git::page_history`, return structured result
- [x] `src/ops/mod.rs` — export history

### 5. MCP

- [x] `src/mcp/tools.rs` — add `wiki_history` tool schema
- [x] `src/mcp/handlers.rs` — `handle_history` handler

### 6. CLI

- [x] `src/cli.rs` — add `History` command with `--limit`,
  `--no-follow`, `--format`
- [x] `src/main.rs` — render history in text and JSON

### 7. Tests

- [ ] History returns commits that touch the page
- [ ] History respects limit
- [ ] History excludes commits that don't touch the page
- [ ] Follow tracks renames (create flat file, rename to bundle,
  history shows both)
- [ ] Empty history for a page with no commits
- [ ] Existing test suite passes unchanged

### 8. Decision record

- [ ] `docs/decisions/wiki-history.md`

### 9. Update skills

- [ ] `llm-wiki-skills/skills/content/SKILL.md` — mention
  `wiki_history` for page change tracking
- [ ] `llm-wiki-skills/skills/research/SKILL.md` — use history
  to assess page freshness

### 10. Finalize

- [ ] `cargo fmt && cargo clippy --all-targets -- -D warnings`
- [ ] Update `CHANGELOG.md`
- [ ] Update `docs/roadmap.md` — move wiki_history from Active to
  Completed
- [ ] Remove this prompt

## Success criteria

- `wiki_history("concepts/moe")` returns commit entries that touched
  the page
- `--follow` tracks renames across flat→bundle migration
- `history.follow` config key works globally and per-wiki
- Limit is respected, default from config
- No regression in existing tools
