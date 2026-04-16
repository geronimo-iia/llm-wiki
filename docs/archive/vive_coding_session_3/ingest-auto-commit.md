# Ingest Auto-Commit: Impact Analysis

Making git commit optional in the ingest pipeline, controlled by a config
flag. `new page`, `new section`, and `lint` never commit — the only way to
commit is `llm-wiki commit` or `llm-wiki ingest` (when `auto_commit = true`).

---

## Current Behavior

Today, `llm-wiki ingest` is an atomic validate → commit → index operation.
Every call produces a git commit unless `--dry-run` is passed. There is no
middle ground — you either commit everything or nothing.

### All git::commit call sites

| Location | Trigger | Message format |
|----------|---------|----------------|
| `src/ingest.rs:81` | `llm-wiki ingest <path>` | `ingest: <path> — +N pages, +M assets` |
| `src/main.rs:196` | `llm-wiki new page` (CLI) | `new: <uri>` |
| `src/main.rs:209` | `llm-wiki new section` (CLI) | `new: <uri>` |
| `src/main.rs:427` | `llm-wiki lint` (CLI) | `lint: <date> — N orphans, N stubs, N empty` |
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

## New Model

### Commit rules

| Command | Commits? |
|---------|----------|
| `llm-wiki new page` | **Never** |
| `llm-wiki new section` | **Never** |
| `llm-wiki lint` / `llm-wiki lint fix` | **Never** |
| `llm-wiki ingest` | Only when `ingest.auto_commit = true` |
| `llm-wiki commit` | **Always** — this is the explicit commit command |

`new page` never commits because the LLM (or human) typically writes
content immediately after scaffolding. Committing an empty scaffold is
wasteful.

### Config

```toml
# ~/.llm-wiki/config.toml or wiki.toml
[ingest]
auto_commit = true   # default: ingest commits after validate + index
```

When `auto_commit = false`:
- `llm-wiki ingest` validates and indexes, but does **not** commit
- Files remain in the git working tree
- The user reviews, then explicitly commits via `llm-wiki commit`

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

Three states:
- `dry_run = true` → validate only, no disk writes, no commit, no index
- `auto_commit = false` → validate → write frontmatter → index (no commit)
- `auto_commit = true` → validate → write frontmatter → index → commit

**Important:** ingest currently *modifies files on disk* (sets `last_updated`,
fills missing `status`/`type`, generates frontmatter for bare files). These
mutations happen regardless of commit. With `auto_commit = false`, the user
sees the mutations in their working tree before deciding to commit. This is
the review point.

### 2. Index staleness

The staleness check compares `state.toml` commit hash against `git HEAD`.
If ingest doesn't commit, HEAD doesn't move, so the index won't be marked
stale — but the index *was* just rebuilt from the current files.

**Decision:** Ingest always updates the search index, even when
`auto_commit = false`. The index reflects what's on disk, search works
immediately. The index is a local artifact, not committed to git.
`state.toml` commit hash would be stale, but that's cosmetic.

### 3. MCP workflow (LLM-driven)

The LLM doesn't have to use `wiki_write` — it can write files directly
into the wiki tree. `wiki_write` is a convenience, not a requirement.

With `auto_commit = true`:

```
write pages → wiki_ingest → committed + indexed → done
```

With `auto_commit = false`:

```
write pages → wiki_ingest → validated + indexed (not committed)
→ (human reviews) → wiki_commit → done
```

**Affected MCP handlers:**
- `handle_ingest` — respect `auto_commit` config
- `handle_new_page` — remove commit call
- `handle_new_section` — remove commit call
- `handle_lint` — remove commit call
- New: `handle_commit` — explicit commit tool

### 4. New `llm-wiki commit` command

Stages and commits changes in the working tree. Accepts slugs to commit
specific pages, or `--all` for everything.

```
llm-wiki commit [<slug>...] [--message <msg>]   # commit specific pages by slug
llm-wiki commit --all [--message <msg>]          # commit all pending changes
```

**Slug resolution for commit:**

- Flat page (`concepts/scaling-laws`) → stages `concepts/scaling-laws.md`
- Bundle page (`concepts/mixture-of-experts`) → stages the entire bundle
  folder recursively: `index.md` + all co-located assets
- Section (`concepts/`) → stages the section `index.md` + all nested
  pages, bundles, and sub-sections recursively

The rule: if the slug resolves to an `index.md`, the entire parent folder
is walked recursively. This covers both bundles and sections uniformly.

MCP tool:

```
wiki_commit(slugs?, message?, wiki?)   # commit specific pages or all pending changes
```

This is a thin wrapper around `git::commit`, but it gives the human a
deliberate approval point.

### 5. `llm-wiki new page` / `llm-wiki new section`

Remove all commit calls. These commands create the scaffold and return.
The user commits later via `llm-wiki commit`.

### 6. `llm-wiki lint` / `llm-wiki lint fix`

Remove all commit calls. Lint writes `LINT.md` (and fix creates stubs),
but does not commit. The user reviews, then commits.

### 7. Instructions and workflows

`src/assets/instructions.md` says "Commit: `wiki_ingest(<path>)`" in every
workflow. Update to reflect the new model:

```
# auto_commit = true
write → wiki_ingest → done

# auto_commit = false
write → wiki_ingest → report to human → (human reviews) → wiki_commit
```

### 8. Session bootstrap loop

The bootstrap loop depends on committed state:

```
Session N:  bootstrap → work → crystallize → ingest → commit
Session N+1: bootstrap → read pages → ...
```

With `auto_commit = false`, Session N+1 can still read uncommitted files
via `wiki_read` (it reads from disk, not from git). But the git history
won't reflect the changes until the human commits. This means:

- `llm-wiki index status` shows stale (commit hash mismatch)
- `git log` doesn't show the work
- If the user discards (`git checkout`), the work is lost

This is the intended behavior — the human is the gatekeeper.

### 9. `git::commit` blast radius

`git::commit` does `index.add_all(["*"])` — it stages everything. With
`auto_commit = false`, the user might have multiple pending ingests before
committing. When they finally run `llm-wiki commit --all`, everything gets
committed in one batch.

This is better for the review workflow: the LLM writes 5 pages, the human
reviews all 5, then commits once with a meaningful message instead of 5
auto-generated messages.

When using `llm-wiki commit <slug>...`, only the resolved file paths for
those slugs are staged and committed.

### 10. Config resolution

`auto_commit` follows the existing config resolution pattern:
global → per-wiki override.

```rust
// config.rs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestConfig {
    #[serde(default = "default_true")]
    pub auto_commit: bool,
}
```

Both `GlobalConfig` and `WikiConfig` get an `ingest` section.

---

## Summary of Required Changes

### Source files

| File | Change |
|------|--------|
| `src/config.rs` | Add `IngestConfig { auto_commit: bool }`, wire into resolution |
| `src/ingest.rs` | Accept `auto_commit`, skip `git::commit` when false |
| `src/main.rs` | Remove commits from `new`, `lint`; pass `auto_commit` to ingest |
| `src/mcp/tools.rs` | Remove commits from `new`, `lint`; add `wiki_commit` tool |
| `src/cli.rs` | Add `Commands::Commit` variant |
| `src/git.rs` | Add slug-scoped staging (for `commit <slug>...`) |
| `src/assets/instructions.md` | Update workflow steps |

### Spec docs — pipelines & core

| File | What to fix |
|------|-------------|
| `docs/specifications/pipelines/ingest.md` | §1 "validate → git add → commit → index" → conditional. §2 "Every ingest produces a git commit" → only when `auto_commit = true`. §2 index section → always updates. §7 MCP tool description. §8 `IngestReport.commit` → optional. Summary line. |
| `docs/specifications/pipelines/crystallize.md` | Lines 32–33 "→ committed pages". Line 188 "validate, commit, and index". Line 217 "validates and commits". Line 229 bootstrap loop. |
| `docs/specifications/pipelines/asset-ingest.md` | Line 88 "validate and commit". Lines 99–100 "git add + commit". |
| `docs/specifications/core/page-content.md` | Line 168 "validates and commits". |
| `docs/specifications/core/repository-layout.md` | Line 105 "committed by llm-wiki lint" → "written by llm-wiki lint". |

### Spec docs — commands

| File | What to fix |
|------|-------------|
| `docs/specifications/commands/cli.md` | Add `llm-wiki commit` command. Line 64 "Validate, commit, and index" → conditional. Line 142 "Writes and commits LINT.md" → "Writes LINT.md". |
| `docs/specifications/commands/lint.md` | Summary, lines 15, 80, 221, 248, 255 — all references to lint committing. |
| `docs/specifications/commands/page-creation.md` | Summary, lines 31, 93, 96, 129 — all references to `new` committing. |
| `docs/specifications/commands/configuration.md` | Add `ingest.auto_commit` key. ✅ Already updated. |

### Spec docs — other

| File | What to fix |
|------|-------------|
| `docs/specifications/features.md` | Lines 45, 48, 51–52, 99, 114, 138, 215 — commit references in ingest, lint, graph, MCP tool descriptions. |
| `docs/specifications/overview.md` | Lines 86, 99, 141 — "validate, commit, and index" descriptions. |
| `docs/specifications/rust-modules.md` | Lines 33, 62 — "validate → git add → commit → index" pipeline description. |

### Top-level docs

| File | What to fix |
|------|-------------|
| `README.md` | Line 148 "validates, commits to git, and indexes". Lines 155, 184, 229 — Mermaid diagrams showing "git add + commit" and "validate → git commit → index" as unconditional steps. |
| `docs/diagrams.md` | Any ingest/lint flow diagrams that show unconditional commit. |

→ [Implementation tasks](ingest-auto-commit-tasks.md)

### Integrations

| File | What to fix |
|------|-------------|
| `docs/specifications/integrations/claude-plugin.md` | Add `/llm-wiki:commit` slash command. Update `SKILL.md` workflow list to include `commit`. Update `/llm-wiki:ingest` description to reflect conditional commit. |
