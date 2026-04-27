---
title: "Incremental Validation"
summary: "Restrict the wiki_ingest validation pass to git-changed files, reusing context the engine already has."
status: implemented
last_updated: "2026-04-27"
---

# Incremental Validation

## Problem

`wiki_ingest` validates every `.md` file under the given path on every call,
unconditionally. For a 500-page wiki the caller pays for 500 file reads and 500
frontmatter parses when only 3 files changed.

The engine already knows what changed: `state.toml` records the last indexed commit,
and `git::collect_changed_files` — already used by the indexer — can answer "what
changed since then" in milliseconds. That context is not threaded into the validation
pass.

The watch path (`src/watch.rs`) is unaffected: it skips validation by design and calls
`index_manager.update()` directly. This is correct — the watcher fires on every editor
save and has no git commit to attach warnings to.

## Goal

Narrow the validation surface to files that are new or changed since the last indexed
commit, without adding new state, new files, or new infrastructure.

## Solution

Thread the git-diff context into `ops::ingest` before the directory walk.

`ops::ingest` already holds `SpaceContext` (via `engine.space(wiki_name)`), which
exposes `index_manager.last_commit()` and `space.repo_root`. Use them to build a
`HashSet` of changed paths, then skip any file absent from that set.

```rust
// src/ops/ingest.rs
let changed: HashSet<PathBuf> = if !options.dry_run {
    let last = space.index_manager.last_commit();
    git::collect_changed_files(&space.repo_root, &space.wiki_root, last.as_deref())
        .unwrap_or_default()
        .into_keys()
        .collect()
} else {
    HashSet::new() // dry_run: validate all (explicit full audit)
};

// in the walk loop:
if !changed.is_empty() && !changed.contains(&relative_path) {
    report.unchanged_count += 1;
    continue;
}
```

**Behaviour by call site:**

| Call site | Validation scope |
|---|---|
| `wiki_ingest <path>` (normal) | git-changed files since last indexed commit |
| `wiki_ingest <path> --dry-run` | all files (explicit audit) |
| `wiki_index_rebuild` | no validation; full index rebuild only |
| watch (`src/watch.rs`) | unchanged — skips validation by design |

**Fallback:** when `last_commit` is absent (first ingest, fresh wiki, detached HEAD)
or `collect_changed_files` returns an error, `changed` is empty and the guard is
skipped — all files are validated. Safety first.

**Why the git commit is the right boundary:** a file already in git passed validation
when it was first ingested. Re-validating it on subsequent ingests is redundant.
The commit is the proof of prior validation.

## Branch & PR — `llm-wiki`

```bash
git checkout -b feat/incremental-validation
```

When implementation is complete and all tests pass:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

```bash
git push -u origin feat/incremental-validation
gh pr create \
  --title "feat: incremental validation — git-diff scoped wiki_ingest" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
Implements imp-5 (incremental validation).

- Narrow wiki_ingest validation to git-changed files since last indexed commit
- dry_run: true continues to validate all files (explicit full audit)
- Fallback to full validation when last_commit is absent or git errors
- Add unchanged_count to IngestReport

Closes geronimo-iia/llm-wiki#22 (imp-5)

Spec: docs/improvements/incremental-validation.md
EOF
)"
```

> No `llm-wiki-skills` PR needed — this is a pure engine performance change
> with no user-facing behaviour change in the skill layer.

## Tasks

### Engine — `llm-wiki` (branch: `feat/incremental-validation`)

- [x] In `src/ingest.rs`, add `changed_paths: Option<HashSet<PathBuf>>` to
  `IngestOptions`; when `Some`, skip files not in the set inside the walk loop;
  increment a new `unchanged_count` field on `IngestReport`.
- [x] In `src/ops/ingest.rs`, before calling `ingest::ingest`, build `changed_paths`
  from `space.index_manager.last_commit()` + `git::collect_changed_files`; set to
  `None` when `dry_run` is true or on git error (full fallback).
- [x] Add `unchanged_count: usize` to `IngestReport` (serde default = 0 for
  backwards compatibility).
- [x] Update the CLI text output to show unchanged count:
  `Ingested: 3 pages, 497 unchanged, 0 assets, 0 warnings`.
- [x] Add unit test: ingest a directory with 5 files where only 2 are in
  `changed_paths`; assert `pages_validated == 2` and `unchanged_count == 3`.
- [x] Add unit test: `dry_run: true` with non-empty `changed_paths` still validates
  all files.
- [x] Add unit test: `last_commit` absent → all files validated (fallback path).

### Spec docs
- [x] `docs/specifications/tools/ingest.md`: document `unchanged_count` field in
  `IngestReport`; add a note on validation scope (git-changed vs dry_run vs full).

### Issue update
After merging:
- Check off imp-5 in `geronimo-iia/llm-wiki#22`
- Mark `status: implemented` in this file
