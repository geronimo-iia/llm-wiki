---
title: "Ingest"
summary: "How content enters the wiki — the engine validates files already in the wiki tree, commits to git, and updates the search index."
read_when:
  - Implementing or extending the ingest pipeline
  - Understanding what the engine validates vs what the author writes
  - Understanding the two ingestion workflows
status: draft
last_updated: "2025-07-15"
---

# Ingest

`wiki ingest` validates files already in the wiki tree, commits them to git,
and updates the search index. It does not move, copy, or place files — the
author (human or LLM) writes directly into the wiki.

---

## 1. Core Principle

The author writes files directly into the wiki tree. The engine validates
what's on disk, stages it, commits, and indexes. No file movement. No
source-to-destination copy. No slug derivation from external paths.

```
Author writes:   wiki/concepts/mixture-of-experts.md  (frontmatter + body)
Engine does:     validate → git add → commit → index
```

---

## 2. What the Engine Does

### Validation

The engine validates every `.md` file on ingest:

| Check | Behavior on failure |
|-------|---------------------|
| Valid YAML frontmatter block | Error — file rejected |
| `title` field present | Error — file rejected |
| `type` field present and recognized | `strict`: Error — file rejected; `loose`: Warning — ingest proceeds, type kept as-is |
| `status` field present | Warning — ingest proceeds, status set to `active` |
| No path traversal (`../`) in path | Error — file rejected |

The engine does **not** modify frontmatter content except:
- `last_updated` — always set to today on ingest
- Missing `status` — set to `active` if absent
- Missing `type` — set to `page` if absent

### Commit

Every ingest produces a git commit:
- `ingest: <path> — +N pages, +M assets`

### Index

The tantivy search index is updated after commit. All frontmatter fields
and body content are indexed.

---

## 3. Two Workflows

### Workflow 1 — Human writes directly

The human creates or edits files in the wiki tree, then runs ingest to
validate and commit.

```bash
# human writes wiki/concepts/mixture-of-experts.md
wiki ingest wiki/concepts/mixture-of-experts.md

# human drops a folder of markdown files
wiki ingest wiki/sources/moe-papers/
```

Frontmatter preserved if present; minimal frontmatter generated if absent
(title from H1 or filename, status `active`, type `page`).

### Workflow 2 — LLM writes via MCP

The LLM writes directly into the wiki tree, then calls ingest.

```
1. LLM searches for existing wiki context:
   wiki_search("<topic>")           → Vec<PageRef>
   wiki_read(<relevant slugs>)     → existing page content

2. LLM writes complete .md files directly into the wiki tree:
   wiki_write("concepts/mixture-of-experts.md", content)

3. wiki_ingest("concepts/mixture-of-experts.md")
   → engine validates, git add, commits, indexes
   → IngestReport returned
```

For updates to existing pages:

```
1. wiki_read(<slug>)               → current content
2. LLM modifies content
3. wiki_write(<slug path>, updated content)
4. wiki_ingest(<slug path>)
```

---

## 4. Accumulation Contract

When updating pages, the LLM must preserve existing list values (`tags`,
`sources`, `claims`). The engine does not enforce this — the file on disk
is what gets committed. The instruct workflow reminds the LLM to read
before writing.

---

## 5. Files Without Frontmatter

When `wiki ingest` processes a file with no frontmatter block, minimal
frontmatter is generated:

| Field | Value |
|-------|-------|
| `title` | First H1 heading in body, or filename stem if no H1 |
| `summary` | `""` (empty) |
| `status` | `active` |
| `last_updated` | Today's ISO 8601 date |
| `type` | `page` as default |
| `tags` | `[]` |

The body is preserved exactly as found.

---

## 6. CLI Interface

```
wiki ingest <path>           # file or folder, relative to wiki root
            [--dry-run]      # show what would be committed, no commit
```

`<path>` is relative to the wiki root. The file must already exist in the
wiki tree.

---

## 7. MCP Tools

### wiki_write

Writes a file into the wiki tree. Does not validate or commit — that's
what `wiki_ingest` is for.

```rust
#[tool(description = "Write a file into the wiki tree")]
async fn wiki_write(
    &self,
    #[tool(param)] path: String,      // relative to wiki root
    #[tool(param)] content: String,
) -> WriteResult { ... }
```

### wiki_ingest

Validates, commits, and indexes files already in the wiki tree.

```rust
#[tool(description = "Validate, commit, and index files in the wiki tree")]
async fn wiki_ingest(
    &self,
    #[tool(param)] path: String,      // relative to wiki root, file or folder
    #[tool(param)] dry_run: Option<bool>,
) -> IngestReport { ... }
```

---

## 8. IngestReport

```rust
pub struct IngestReport {
    pub pages_validated: usize,
    pub assets_found:    usize,
    pub warnings:        Vec<String>,
    pub commit:          String,   // git commit hash
}
```

---

## 9. What Was Removed

| Removed | Reason |
|---------|--------|
| `--target <name\|wiki:// URI>` flag | No destination — files are already in the wiki |
| `--update` flag | No create/update distinction — file is on disk, ingest commits it |
| Source-to-destination placement | Author writes directly into the wiki tree |
| Slug derivation from external paths | No external paths |
| `wiki://` URI resolution in ingest | Ingest takes a path relative to wiki root |
| `integrate_file` / `integrate_folder` | No file movement to integrate |
| Bundle promotion logic in ingest | Author creates bundles directly |
