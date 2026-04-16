---
title: "Commit"
summary: "Commit pending changes to git — by slug (scoped) or all at once. The only explicit commit path in the engine."
read_when:
  - Implementing or extending the commit command
  - Understanding how commits work after ingest, new, or lint
  - Understanding slug-scoped vs full commits
status: draft
last_updated: "2025-07-15"
---

# Commit

`llm-wiki commit` is the explicit way to commit changes to git. No other
command commits — `new`, `lint`, and `lint fix` never commit, and `ingest`
only commits when `ingest.auto_commit` is `true`.

---

## 1. CLI Interface

```
llm-wiki commit [<slug>...] [--message <msg>]   # commit specific pages by slug
llm-wiki commit --all [--message <msg>]          # commit all pending changes
```

No slugs and no `--all` → error: `"specify slugs or --all"`.

### Examples

```bash
# Commit a single flat page
llm-wiki commit concepts/scaling-laws

# Commit a bundle (index.md + all assets)
llm-wiki commit concepts/mixture-of-experts

# Commit a section and everything under it
llm-wiki commit concepts/

# Commit multiple pages
llm-wiki commit concepts/scaling-laws sources/switch-transformer-2021

# Commit with a custom message
llm-wiki commit concepts/scaling-laws --message "reviewed: scaling laws"

# Commit all pending changes
llm-wiki commit --all
llm-wiki commit --all --message "batch: session 12 pages"
```

---

## 2. Slug Resolution

When committing by slug, the engine resolves each slug to file paths:

| Slug resolves to | What gets staged |
|------------------|-----------------|
| Flat page (`concepts/scaling-laws.md`) | That single `.md` file |
| Bundle (`concepts/moe/index.md`) | The entire bundle folder recursively — `index.md` + all co-located assets |
| Section (`concepts/index.md`) | The entire section folder recursively — section index + all nested pages, bundles, and sub-sections |

The rule: if the slug resolves to an `index.md`, the entire parent folder
is walked recursively. This covers both bundles and sections uniformly.

---

## 3. Staging Behavior

### `llm-wiki commit <slug>...`

Uses `git::commit_paths` — only the resolved file paths are staged via
`index.add_path()`. Other modified files in the working tree remain
unstaged.

### `llm-wiki commit --all`

Uses `git::commit` — stages everything via `index.add_all(["*"])`, then
commits. Equivalent to `git add -A && git commit`.

---

## 4. Default Commit Messages

| Invocation | Default message |
|-----------|----------------|
| `llm-wiki commit concepts/moe` | `commit: concepts/moe` |
| `llm-wiki commit concepts/moe sources/paper` | `commit: concepts/moe, sources/paper` |
| `llm-wiki commit --all` | `commit: all` |
| `llm-wiki commit --all -m "reviewed"` | `reviewed` |

The `--message` / `-m` flag overrides the default in all cases.

---

## 5. Errors

| Condition | Error |
|-----------|-------|
| No slugs and no `--all` | `specify slugs or --all` |
| Slug not found | `page not found for slug: <slug>` |
| No git repo | `failed to open repo at <path>` |
| Nothing to commit | git error (empty commit) |

---

## 6. MCP Tool

```rust
#[tool(description = "Commit pending changes to git")]
async fn wiki_commit(
    &self,
    #[tool(param)] slugs: Option<String>,   // comma-separated
    #[tool(param)] message: Option<String>,
    #[tool(param)] wiki: Option<String>,
) -> String { ... }   // returns commit hash
```

When `slugs` is provided, each slug is resolved and only those files are
committed. When `slugs` is omitted, all pending changes are committed.

Returns the git commit hash on success.

---

## 7. Relationship to Other Commands

```
llm-wiki new page   → creates scaffold, no commit
llm-wiki new section → creates scaffold, no commit
llm-wiki lint       → writes LINT.md, no commit
llm-wiki lint fix   → creates stubs, no commit
llm-wiki ingest     → validates + indexes; commits only if auto_commit = true
llm-wiki commit     → always commits
```

Typical workflow:

```
write pages → wiki_ingest → (human reviews) → wiki_commit
```

When `ingest.auto_commit = true`, ingest commits automatically and
`wiki_commit` is not needed unless committing non-ingested changes
(e.g. lint output, new scaffolds).
