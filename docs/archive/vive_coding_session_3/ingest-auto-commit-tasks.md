# Ingest Auto-Commit: Task List

Ordered implementation tasks for the commit model change described in
[ingest-auto-commit.md](ingest-auto-commit.md).

Reference: all `git::commit` call sites, config resolution, CLI/MCP handlers.

---

## Task 1 — Config: add `ingest.auto_commit`

**Goal:** Wire `ingest.auto_commit` into the two-level config system.

### Code changes

- `src/config.rs`:
  - Add `IngestConfig { auto_commit: bool }` with `#[serde(default = "default_true")]`
  - Add `ingest: Option<IngestConfig>` to both `GlobalDefaults` and `WikiConfig`
  - Add resolution in `resolve_config()` (per-wiki overrides global)
  - Add `ingest.auto_commit` to `get_config_value` / `set_config_value`

### Exit criteria

- `llm-wiki config get ingest.auto_commit` returns `true` (built-in default)
- `llm-wiki config set ingest.auto_commit false --global` persists to `~/.llm-wiki/config.toml`
- `llm-wiki config set ingest.auto_commit false --wiki research` persists to `wiki.toml`
- Per-wiki overrides global
- `cargo test` passes

---

## Task 2 — Remove commits from `new page` / `new section`

**Goal:** `new page` and `new section` never commit, regardless of config.

### Code changes

- `src/main.rs`:
  - Remove `git::commit(&repo_root, &format!("new: {uri}"))?;` (lines ~219, ~232)
- `src/mcp/tools.rs`:
  - Remove `git::commit` calls in `wiki_new_page` and `wiki_new_section` handlers (lines ~509, ~522)

### Exit criteria

- `llm-wiki new page wiki://test/foo` creates the file, no git commit
- MCP `wiki_new_page` creates the file, no git commit
- `cargo test` passes

---

## Task 3 — Remove commits from `lint` / `lint fix`

**Goal:** `lint` and `lint fix` never commit, regardless of config.

### Code changes

- `src/main.rs`:
  - Remove `git::commit(&entry_path, &msg)?;` in lint handler (line ~523)
  - Remove `git::commit(&entry_path, &msg).is_ok()` in lint fix handler (line ~591)
- `src/mcp/tools.rs`:
  - Remove `git::commit` calls in `wiki_lint` and `wiki_lint_fix` handlers (lines ~724, ~778)

### Exit criteria

- `llm-wiki lint` writes `LINT.md`, no git commit
- `llm-wiki lint fix` creates stubs, no git commit
- MCP equivalents behave the same
- `cargo test` passes

---

## Task 4 — Ingest: respect `auto_commit` config

**Goal:** `llm-wiki ingest` only commits when `auto_commit = true`.
Index is always updated regardless.

### Code changes

- `src/ingest.rs`:
  - Add `auto_commit: bool` to `IngestOptions`
  - Gate `git::commit` on `!dry_run && auto_commit`
  - `IngestReport.commit` is empty string when no commit happens
- `src/main.rs`:
  - Read resolved `ingest.auto_commit` from config
  - Pass to `IngestOptions`
- `src/mcp/tools.rs`:
  - Same: read config, pass to `IngestOptions`

### Exit criteria

- `ingest.auto_commit = true` → ingest validates, indexes, commits (current behavior)
- `ingest.auto_commit = false` → ingest validates, indexes, no commit
- `--dry-run` still skips everything (no write, no commit, no index)
- `IngestReport.commit` is empty when no commit
- `cargo test` passes

---

## Task 5 — `git::commit_paths`: slug-scoped staging

**Goal:** Add a function to stage and commit specific file paths only,
for `llm-wiki commit <slug>...`.

### Code changes

- `src/git.rs`:
  - Add `commit_paths(repo_root, paths: &[&Path], message) -> Result<String>`
  - Stages only the given paths (not `add_all(["*"])`)
  - Commits with the given message

### Exit criteria

- `commit_paths` stages only the listed files
- Other modified files in the repo remain unstaged
- `cargo test` passes (unit test with a temp git repo)

---

## Task 6 — `llm-wiki commit` CLI command

**Goal:** Add the explicit commit command.

### Code changes

- `src/cli.rs`:
  - Add `Commands::Commit` variant:
    ```rust
    Commit {
        /// Page slugs to commit (omit for --all)
        slugs: Vec<String>,
        /// Commit all pending changes
        #[arg(long)]
        all: bool,
        /// Commit message
        #[arg(long, short)]
        message: Option<String>,
    }
    ```
- `src/main.rs`:
  - Add `Commands::Commit` match arm:
    - `--all` → `git::commit(repo_root, message)`
    - `<slug>...` → resolve slugs to file paths, `git::commit_paths(repo_root, paths, message)`
    - No slugs and no `--all` → error: "specify slugs or --all"
    - Default message: `"commit: <slug1>, <slug2>, ..."` or `"commit: all"`

### Exit criteria

- `llm-wiki commit concepts/moe --message "reviewed"` commits only that page
- `llm-wiki commit --all` commits everything
- `llm-wiki commit` (no args, no --all) prints error
- `cargo test` passes

---

## Task 7 — `wiki_commit` MCP tool

**Goal:** Expose commit as an MCP tool.

### Code changes

- `src/mcp/tools.rs`:
  - Add `wiki_commit` tool:
    ```rust
    #[tool(description = "Commit pending changes to git")]
    async fn wiki_commit(
        &self,
        #[tool(param)] slugs: Option<Vec<String>>,
        #[tool(param)] message: Option<String>,
        #[tool(param)] wiki: Option<String>,
    ) -> String { ... }
    ```
  - Same logic as CLI: slugs → scoped commit, no slugs → commit all

### Exit criteria

- MCP `wiki_commit(slugs: ["concepts/moe"])` commits only that page
- MCP `wiki_commit()` (no slugs) commits all
- Returns commit hash or error
- `cargo test` passes

---

## Task 8 — Update CLI descriptions and help text

**Goal:** Fix doc strings and help text to reflect the new model.

### Code changes

- `src/cli.rs`:
  - `Ingest` doc: "Validate, commit, and index" → "Validate and index files in the wiki tree"
  - `Ingest.dry_run` doc: "Show what would be committed without committing" → "Validate only, no disk writes"
  - `Lint.dry_run` doc: "Show what would be written, no commit" → "Show what would be written"
  - `Graph.dry_run` doc: "Print what would be written, no commit" → "Print what would be written"

### Exit criteria

- `llm-wiki --help` and subcommand help text reflects new model
- No mention of "commit" in `new`, `lint`, `graph` help text

---

## Task 9 — Update `instructions.md`

**Goal:** Update embedded workflow instructions for the new commit model.

### Code changes

- `src/assets/instructions.md`:
  - All workflows: remove "commit" from ingest step descriptions
  - Add `wiki_commit` to workflow steps where appropriate
  - Add note about `auto_commit` config controlling ingest commit behavior

### Exit criteria

- `llm-wiki instruct ingest` reflects conditional commit
- `llm-wiki instruct` shows `commit` as a workflow/tool
- `cargo test` passes

---

## Task 10 — Update spec docs

**Goal:** Align all specification documents with the new commit model.

### Files to update

**Pipelines & core:**
- `docs/specifications/pipelines/ingest.md` — conditional commit, index always updates, MCP tool, IngestReport
- `docs/specifications/pipelines/crystallize.md` — "committed pages" → conditional, bootstrap loop
- `docs/specifications/pipelines/asset-ingest.md` — "validate and commit" → conditional
- `docs/specifications/core/page-content.md` — "validates and commits"
- `docs/specifications/core/repository-layout.md` — "committed by llm-wiki lint" → "written by"

**Commands:**
- `docs/specifications/commands/cli.md` — add `llm-wiki commit`, fix ingest/lint/new descriptions
- `docs/specifications/commands/lint.md` — remove all commit references
- `docs/specifications/commands/page-creation.md` — remove all commit references

**Other:**
- `docs/specifications/features.md` — commit references in ingest, lint, graph, MCP tools
- `docs/specifications/overview.md` — "validate, commit, and index" descriptions
- `docs/specifications/rust-modules.md` — pipeline description

**Integrations:**
- `docs/specifications/integrations/claude-plugin.md` — add `/llm-wiki:commit`, update SKILL.md, update `/llm-wiki:ingest`

**Top-level:**
- `README.md` — descriptions and Mermaid diagrams
- `docs/diagrams.md` — ingest/lint flow diagrams

### Exit criteria

- No spec doc describes `new`, `lint`, or `graph` as committing
- `ingest` commit is described as conditional on `auto_commit`
- `llm-wiki commit` is documented everywhere
- Mermaid diagrams reflect the new pipeline

---

## Execution order

| Order | Task | Dependencies |
|-------|------|-------------|
| 1 | Task 1 — Config | None |
| 2 | Task 2 — Remove commits from `new` | None |
| 3 | Task 3 — Remove commits from `lint` | None |
| 4 | Task 4 — Ingest respects config | Task 1 |
| 5 | Task 5 — `git::commit_paths` | None |
| 6 | Task 6 — `llm-wiki commit` CLI | Task 5 |
| 7 | Task 7 — `wiki_commit` MCP tool | Task 5 |
| 8 | Task 8 — CLI help text | Tasks 2, 3, 6 |
| 9 | Task 9 — Instructions | Tasks 2, 3, 4, 6 |
| 10 | Task 10 — Spec docs | All above |

Tasks 1, 2, 3, 5 can run in parallel. Task 10 is last — update docs only
after the code is settled.
