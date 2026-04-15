---
title: "Ingest"
summary: "How content enters a wiki — the LLM writes complete Markdown files with frontmatter, the engine validates, places, commits, and indexes."
read_when:
  - Implementing or extending the ingest pipeline
  - Understanding where ingested content lands in the wiki
  - Understanding the two ingestion workflows
  - Understanding what the engine validates vs what the LLM authors
status: draft
last_updated: "2025-07-15"
---

# Ingest

`wiki ingest` takes a Markdown file or folder and places it into the wiki.
The file is the complete page — frontmatter and body, authored by whoever
produced it (human or LLM). The engine validates, resolves the target path,
commits to git, and updates the search index.

---

## 1. Core Principle

The LLM writes complete Markdown files. The engine does not assemble pages,
merge frontmatter fields, or translate JSON into YAML. The file the LLM
writes is the file that lands in the wiki.

```
LLM writes:    concepts/mixture-of-experts.md  (frontmatter + body)
Engine does:   validate → place → commit → index
```

This replaces the previous model where the LLM produced `analysis.json` /
`enrichment.json` and the engine assembled pages from JSON fields.

---

## 2. Source and Destination

```
External filesystem          Target wiki (git repository)
────────────────────         ────────────────────────────────────
/any/path/on/disk/     →     wiki://research/skills/
  folder/                      wiki://research/skills/semantic-commit
    SKILL.md                   wiki://research/skills/semantic-commit/index.md
    lifecycle.yaml             wiki://research/skills/semantic-commit/lifecycle.yaml
```

The source path is always **outside** the wiki root. The target is specified
via `--target`, which accepts either a wiki name or a `wiki://` URI:

```bash
# --target as wiki:// URI — encodes both wiki name and prefix
wiki ingest ~/agent-skills/semantic-commit/ --target wiki://research/skills

# --target as wiki name — slug derived directly from source path
wiki ingest ~/agent-skills/semantic-commit/ --target research
```

When `--target` is a `wiki://` URI, the URI path becomes the prefix.
When `--target` is a wiki name, the slug is derived directly from the
source path with no prefix.

If `--target` is omitted, the default wiki (`global.default_wiki`) is used.

---

## 3. What the Engine Does

### Validation

The engine validates every `.md` file on ingest:

| Check | Behavior on failure |
|-------|---------------------|
| Valid YAML frontmatter block | Error — file rejected |
| `title` field present | Error — file rejected |
| `type` field present and recognized | Warning — ingest proceeds, type set to `page` |
| `status` field present | Warning — ingest proceeds, status set to `active` |
| Slug does not already exist (create) | Error — use `--update` to overwrite |
| Slug exists (update) | Error without `--update` flag |
| No path traversal (`../`) in slug | Error — file rejected |

The engine does **not** modify frontmatter content except:
- `last_updated` — always set to today on ingest
- Missing `status` — set to `active` if absent
- Missing `type` — inferred from slug prefix if absent

### Placement

Slug derivation from source path + target:

```
source root:  /home/user/agent-skills/
file:         /home/user/agent-skills/semantic-commit/SKILL.md
target:       wiki://research/skills

relative path:  semantic-commit/SKILL.md
slug stem:      semantic-commit
with prefix:    skills/semantic-commit
wiki URI:       wiki://research/skills/semantic-commit
disk path:      /wikis/research/skills/semantic-commit/index.md  (bundle)
```

### Commit

Every ingest produces a git commit:
- Create: `ingest: <slug> — +N pages, +M assets`
- Update: `ingest(update): <slug>`

### Index

The tantivy search index is updated after commit. All frontmatter fields
and body content are indexed.

---

## 4. Two Workflows

### Workflow 1 — Human ingest

No LLM involved. The human ingests existing Markdown files directly.

```bash
wiki ingest ~/agent-skills/semantic-commit/ --target wiki://research/skills
```

Files copied as-is. Frontmatter preserved if present; minimal frontmatter
generated if absent (title from H1 or filename, status `active`, type from
slug prefix).

Right for: skills, guides, specs, reference folders, any already-structured
Markdown content.

### Workflow 2 — LLM-driven ingest (MCP)

The LLM reads a source, writes complete wiki pages, and ingests them.

```
1. LLM reads the source document (via filesystem or user paste)

2. LLM searches for existing wiki context:
   wiki_search("<topic>")           → Vec<PageRef>
   wiki_read(<relevant slugs>)     → existing page content

3. LLM writes complete .md files to a temp location:
   - Source summary page (frontmatter + body)
   - New or updated concept pages (frontmatter + body)
   - Query result pages if valuable synthesis emerged

4. wiki_ingest(path, target)
   → engine validates, places, commits, indexes
   → IngestReport returned

5. For updates to existing pages:
   wiki_read(<slug>)               → current content
   LLM merges new knowledge into existing content
   LLM writes updated .md file
   wiki_ingest(path, target, update: true)
```

The LLM is responsible for:
- Reading existing pages before updating them
- Merging new knowledge with existing content
- Writing complete, valid Markdown with frontmatter
- Following the frontmatter authoring guide
- Applying the backlink quality test for links

The engine is responsible for:
- Validating frontmatter structure
- Placing files at the correct path
- Git commit
- Search index update

---

## 5. Update Flow

When the LLM wants to update an existing page:

```
1. wiki_read(slug)                  → current frontmatter + body
2. LLM modifies content as needed
3. LLM writes the complete updated file
4. wiki_ingest(path, target, update: true)
```

The LLM is responsible for preserving fields it doesn't intend to change.
There are no automatic merge rules — the file the LLM writes is the file
that lands in the wiki.

**Accumulation responsibility:** when updating, the LLM must not silently
drop existing `tags`, `read_when`, `sources`, or `claims` values added by
prior ingests. The frontmatter authoring guide documents this as a common
mistake. The instruct workflow reminds the LLM to read before updating.

---

## 6. CLI Interface

```
wiki ingest <path>                         # file or folder
            [--target <name|wiki:// URI>]  # target wiki (default: global.default_wiki)
            [--update]                     # overwrite existing page
            [--dry-run]                    # show what would be written, no commit
```

`--append` is removed. The LLM reads the existing page, appends content,
and writes the complete file with `--update`.

`--analysis` is removed. The LLM writes complete Markdown files instead of
producing enrichment JSON.

---

## 7. MCP Tool

```rust
#[tool(description = "Ingest a Markdown file or folder into the wiki")]
async fn wiki_ingest(
    &self,
    #[tool(param)] path: String,
    #[tool(param)] target: Option<String>,  // wiki name or wiki:// URI
    #[tool(param)] update: Option<bool>,
) -> IngestReport { ... }
```

---

## 8. IngestReport

```rust
pub struct IngestReport {
    pub pages_written:   usize,
    pub assets_written:  usize,
    pub bundles_created: usize,
    pub commit:          String,   // git commit hash
}
```

`enriched` and `queries_written` are removed — the engine does not
distinguish between page types during ingest. A page is a page.

---

## 9. What Was Removed

| Removed | Reason |
|---------|--------|
| `--analysis <file>` flag | LLM writes complete Markdown instead of enrichment JSON |
| `--append` flag | LLM reads, appends, writes complete file with `--update` |
| `enrichment.json` contract | Replaced by LLM-authored Markdown |
| `Enrichment` struct | No longer needed |
| `QueryResult` struct | LLM writes query-result pages as regular Markdown |
| Frontmatter merge rules (UNION, APPEND, SET, PRESERVE) | LLM manages the full file |
| `integrate_enrichment()` | No longer needed |
| `integrate_query_result()` | No longer needed |
| `analysis.rs` enrichment types | Replaced by frontmatter validation in `markdown.rs` |

---

## 10. Rust Module Changes

| Module | Change |
|--------|--------|
| `cli.rs` | `ingest` takes `<path>`; `--target`, `--update`, `--dry-run`. Remove `--analysis`, `--append` |
| `ingest.rs` | `Input::Direct(PathBuf)`; `IngestOptions { target, update }` |
| `integrate.rs` | `integrate_file`, `integrate_folder`. Remove `integrate_enrichment`, `integrate_query_result` |
| `markdown.rs` | `validate_frontmatter(fm) -> Result<Vec<Warning>>`. Remove `merge_enrichment`, `generate_minimal_frontmatter` (keep for files without frontmatter only) |
| `analysis.rs` | Remove `Enrichment`, `QueryResult`, `Analysis`. Keep `Claim`, `Confidence` as frontmatter field types |
| `server.rs` | `wiki_ingest` without `analysis` param |

---

## 11. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki ingest <file>` (direct file) | **not implemented** |
| `wiki ingest <folder>` (direct folder) | **not implemented** |
| `--target` as `wiki://` URI | **not implemented** |
| `--update` flag | **not implemented** |
| Frontmatter validation | **not implemented** |
| `wiki_ingest` MCP tool | **not implemented** |
