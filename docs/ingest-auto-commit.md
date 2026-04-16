# Ingest Auto-Commit: Impact Analysis

Making git commit optional in the ingest pipeline, controlled by a global
config flag.

---

## Current Behavior

Today, `wiki ingest` is an atomic validate → commit → index operation.
Every call produces a git commit unless `--dry-run` is passed. There is no
middle ground — you either commit everything or nothing.

### All git::commit call sites

| Location | Trigger | Message format |
|----------|---------|----------------|
| `src/ingest.rs:81` | `wiki ingest <path>` | `ingest: <path> — +N pages, +M assets` |
| `src/main.rs:196` | `wiki new page` (CLI) | `new: <uri>` |
| `src/main.rs:209` | `wiki new section` (CLI) | `new: <uri>` |
| `src/main.rs:427` | `wiki lint` (CLI) | `lint: <date> — N orphans, N stubs, N empty` |
| `src/mcp/tools.rs:447` | `wiki_new_page` (MCP) | `new: <uri>` |
| `src/mcp/tools.rs:458` | `wiki_new_section` (MCP) | `new: <uri>` |
| `src/mcp/tools.rs:580` | `wiki_lint` (MCP) | `lint: <date> — ...` |

The ingest call in `src/ingest.rs` is the only one gated by `dry_run`.
All others commit unconditionally.

### git::commit behavior

`git::commit` in `src/git.rs` does `git add *` + commit. It stages
**everything** in the repo, not just the ingested path. This means any
uncommitted change anywhere in the repo gets swept into the ingest commit.

---

## Proposed Change

Add `ingest.auto_commit` config flag (default: `true` for backward
compatibility).

```toml
# ~/.wiki/config.toml
[ingest]
auto_commit = true   # default: current behavior
```

When `auto_commit = false`:
- `wiki ingest` validates and indexes, but does **not** commit
- Files remain staged (or unstaged) in the git working tree
- The user reviews, then explicitly commits (via `wiki commit` or `git commit`)

---

## Impact Analysis

### 1. Ingest pipeline (`src/ingest.rs`)

**Change:** Skip `git::commit` when `auto_commit = false`.

The `IngestReport.commit` field becomes empty string when no commit happens.
Callers that check `report.commit` need to handle the empty case.

```rust
pub struct IngestOptions {
    pub dry_run: bool,
    pub auto_commit: bool,  // new
}
```

The ingest function already has the `dry_run` gate — `auto_commit = false`
is a third state: validate + write frontmatter changes, but don't commit.

**Important:** ingest currently *modifies files on disk* (sets `last_updated`,
fills missing `status`/`type`, generates frontmatter for bare files). These
mutations happen regardless of commit. With `auto_commit = false`, the user
sees the mutations in their working tree before deciding to commit. This is
actually desirable — it's the review point.

### 2. Index staleness

The staleness check compares `state.toml` commit hash against `git HEAD`.
If ingest doesn't commit, HEAD doesn't move, so the index won't be marked
stale — but the index *was* just rebuilt from the current files.

**Decision needed:** should ingest still update the search index when
`auto_commit = false`?

- **Yes (recommended):** index reflects what's on disk, search works
  immediately. The index is a local artifact anyway, not committed to git.
  `state.toml` commit hash would be stale, but that's cosmetic.
- **No:** search returns stale results until the user commits and rebuilds.
  Poor UX.

### 3. MCP workflow (LLM-driven)

This is the biggest impact. The current LLM workflow is:

```
wiki_write → wiki_ingest → done (committed, indexed, searchable)
```

With `auto_commit = false`, the LLM writes and validates, but the knowledge
isn't committed. The LLM can't tell the user "committed at abc123" — it can
only say "validated, pending review."

**New workflow:**

```
wiki_write → wiki_ingest → (human reviews) → wiki_commit → done
```

This requires a new `wiki_commit` tool/command that the human (or LLM with
human approval) can invoke.

**Affected MCP handlers:**
- `handle_ingest` — must respect `auto_commit` config
- `handle_new_page` — currently commits unconditionally
- `handle_new_section` — currently commits unconditionally
- `handle_lint` — currently commits unconditionally

### 4. New `wiki commit` command

Needed when `auto_commit = false`. Stages and commits whatever is in the
working tree.

```
wiki commit [--message <msg>]     # explicit commit
wiki commit --all                 # commit all pending changes
```

MCP tool:

```
wiki_commit(message?, wiki?)      # commit pending changes
```

This is essentially a thin wrapper around `git::commit`, but it gives the
human a deliberate approval point.

### 5. `wiki new page` / `wiki new section`

These currently commit unconditionally (both CLI and MCP). With
`auto_commit = false`, they should create the scaffold but not commit.

The scaffold is useful to review — the user can see the generated frontmatter
before committing.

### 6. `wiki lint` / `wiki lint fix`

Lint writes `LINT.md` and commits. With `auto_commit = false`, it should
write `LINT.md` but not commit. The user reviews the lint report, then
commits.

### 7. Instructions and workflows

`src/assets/instructions.md` says "Commit: `wiki_ingest(<path>)`" in every
workflow. With `auto_commit = false`, the instructions need a conditional:

- If auto-commit is on: workflow unchanged
- If auto-commit is off: add a "wait for human review" step before commit

The instruct workflows become:

```
# auto_commit = true (current)
wiki_write → wiki_ingest → done

# auto_commit = false (new)
wiki_write → wiki_ingest → report to human → (human reviews) → wiki_commit
```

### 8. Session bootstrap loop

The bootstrap loop depends on committed state:

```
Session N:  bootstrap → work → crystallize → ingest → commit
Session N+1: bootstrap → read committed pages → ...
```

With `auto_commit = false`, Session N+1 can still read uncommitted files
via `wiki_read` (it reads from disk, not from git). But the git history
won't reflect the changes until the human commits. This means:

- `wiki index status` shows stale (commit hash mismatch)
- `git log` doesn't show the work
- If the user discards (`git checkout`), the work is lost

This is the intended behavior — the human is the gatekeeper.

### 9. `git::commit` blast radius

`git::commit` does `index.add_all(["*"])` — it stages everything. With
`auto_commit = false`, the user might have multiple pending ingests before
committing. When they finally run `wiki commit`, everything gets committed
in one batch.

**This is actually better** for the review workflow: the LLM writes 5 pages,
the human reviews all 5, then commits once with a meaningful message instead
of 5 auto-generated messages.

### 10. Config resolution

`auto_commit` should follow the existing config resolution pattern:
global → per-wiki override.

```rust
// config.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}
```

Both `GlobalConfig` and `WikiConfig` get an `ingest` section. A research
wiki might want `auto_commit = false` (human reviews LLM output), while a
personal notes wiki keeps `auto_commit = true`.

---

## Summary of Required Changes

| File | Change |
|------|--------|
| `src/config.rs` | Add `IngestConfig { auto_commit: bool }`, wire into resolution |
| `src/ingest.rs` | Accept `auto_commit`, skip `git::commit` when false |
| `src/main.rs` | Pass `auto_commit` to ingest; gate commits in `new`, `lint` |
| `src/mcp/tools.rs` | Same gating in all MCP handlers; add `wiki_commit` tool |
| `src/cli.rs` | Add `Commands::Commit` variant; add `wiki_commit` subcommand |
| `src/git.rs` | No change (commit logic stays the same) |
| `src/assets/instructions.md` | Conditional workflow steps for review mode |
| `docs/specifications/pipelines/ingest.md` | Document both modes |
| `docs/specifications/commands/cli.md` | Add `wiki commit` command |

---

## Workflow Comparison

### auto_commit = true (current, default)

```
LLM writes pages → wiki_ingest → committed + indexed → done
Human writes pages → wiki ingest → committed + indexed → done
```

Single-step. No review gate. Fast. Suitable for trusted workflows or
personal wikis.

### auto_commit = false (new, opt-in)

```
LLM writes pages → wiki_ingest → validated + indexed (not committed)
Human reviews working tree (git diff, read pages)
Human runs: wiki commit --message "reviewed: MoE routing pages"
→ committed → done
```

Two-step. Human review gate. Suitable for shared wikis, quality-sensitive
domains, or when the LLM is not fully trusted.

### Mixed: per-command override

Consider a `--no-commit` / `--commit` CLI flag that overrides the config
for a single invocation:

```bash
wiki ingest wiki/concepts/moe.md --no-commit    # override auto_commit=true
wiki ingest wiki/concepts/moe.md --commit        # override auto_commit=false
```

This gives maximum flexibility without changing the default.
