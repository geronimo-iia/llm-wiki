---
title: "Ingest"
summary: "File/folder as the default ingest entry point with optional analysis JSON enrichment. Three modes: direct, direct + enrichment, analysis-only (legacy)."
read_when:
  - Implementing or extending the ingest pipeline
  - Ingesting local Markdown files or folders into the wiki
  - Adding a skill, guide, or spec to the wiki without an LLM analysis step
  - Understanding the relationship between direct ingest and analysis JSON
status: draft
last_updated: "2025-07-15"
---

# Ingest

File/folder is the default ingest entry point. Analysis JSON is an optional
enrichment layer, not a prerequisite.

---

## 1. The Problem with the Current Design

The current pipeline has one entry point: `wiki ingest analysis.json`. An
external LLM must produce a structured analysis before anything can be written.

This is right for sources that need analysis — research papers, blog posts,
transcripts. It is wrong for already-structured content:

- **Skills** — `SKILL.md`, `lifecycle.yaml`, scripts. Nothing to analyze.
- **Guides and how-to docs** — already written Markdown, ready to index.
- **Reference folders** — YAML configs, scripts, data files.
- **Agent-foundation specs** — finalized specs that should be searchable without
  LLM processing.

Requiring an LLM step for these adds friction and cost with no benefit.

---

## 2. Three Ingest Modes

### Mode A — Direct (default)

```bash
wiki ingest path/to/file.md
wiki ingest path/to/folder/
```

Files ingested as-is. No LLM step. Frontmatter preserved if present; minimal
frontmatter generated if absent. Non-Markdown files co-located with their page
in a bundle folder. Right for: skills, guides, specs, reference folders, any
already-structured content.

### Mode B — Direct + Analysis enrichment

```bash
wiki ingest path/to/folder/ --analysis analysis.json
```

Files ingested as-is first, then the analysis JSON applied on top: suggested
pages merged, contradictions written, claims and concepts added to frontmatter.
The analysis enriches what was already ingested — it does not replace it.

### Mode C — Analysis only (legacy)

```bash
wiki ingest --analysis-only analysis.json
```

No file/folder path. Pure analysis JSON pipeline, existing behaviour. Right for:
sources with no local files (URLs, transcripts, remote papers).

All modes produce a git commit and update the tantivy index.

---

## 3. Direct File Ingest

```bash
wiki ingest docs/skills/my-skill/SKILL.md
wiki ingest agent-foundation/skills/authoring-guide.md
```

- Valid frontmatter with `title` → preserved as-is; slug derived from file path
  (or `--slug` override)
- No frontmatter → generate minimal: `title` from H1 or filename,
  `status: active`, `last_updated: today`
- `action` defaults to `create`; `--update` or `--append` to override
- Git commit: `ingest(direct): <filename>`

---

## 4. Direct Folder Ingest

```bash
wiki ingest agent-skills/my-skill/
wiki ingest guides/python/
```

- Walk recursively
- Each `.md` file → direct file ingest (§ 3)
- Each non-`.md` file → co-located asset: placed beside the folder's `index.md`,
  not under central `assets/`. The folder becomes a bundle automatically.
  See [asset-ingest.md](asset-ingest.md) and [repository-layout.md](repository-layout.md).
- Slug derivation for pages: path relative to the ingest root, preserving
  structure. `my-skill/SKILL.md` → `skills/my-skill` (bundle slug)
- `--prefix` overrides the page slug prefix. Asset slugs follow the page slug.
- Single git commit: `ingest(direct): <folder-name> — +N pages, +M assets`

---

## 5. Skill Example

```bash
# Ingest a skill folder — no LLM needed
wiki ingest agent-skills/semantic-commit/ --prefix skills

# Optionally enrich with LLM analysis afterwards
wiki ingest agent-skills/semantic-commit/ --prefix skills --analysis analysis.json

# Result:
# skills/semantic-commit/index.md        ← from SKILL.md (bundle, has assets)
# skills/semantic-commit/lifecycle.yaml  ← co-located, not in assets/
# skills/semantic-commit/install.sh      ← co-located
```

The skill's `SKILL.md` frontmatter is preserved. Non-Markdown files stay beside
the page. `git log skills/semantic-commit/` shows the full history of the skill
including its assets.

---

## 6. CLI Interface

```
wiki ingest <path>                       # file or folder (default)
            [--prefix <slug-prefix>]
            [--update]                   # action=update instead of create
            [--append]                   # action=append instead of create
            [--analysis <file>]          # optional enrichment
            [--dry-run]                  # show what would be written, no commit

wiki ingest --analysis-only <file>       # legacy: analysis JSON only
```

---

## 7. MCP Tools

```rust
// Primary — file or folder, optional analysis enrichment
#[tool(description = "Ingest a local file or folder into the wiki")]
async fn wiki_ingest(
    &self,
    #[tool(param)] path: String,
    #[tool(param)] prefix: Option<String>,
    #[tool(param)] update: Option<bool>,
    #[tool(param)] analysis: Option<serde_json::Value>,
    #[tool(param)] wiki: Option<String>,
) -> IngestReport { ... }

// Legacy — analysis JSON only
#[tool(description = "Ingest a pre-built Analysis JSON into the wiki")]
async fn wiki_ingest_analysis(
    &self,
    #[tool(param)] analysis: serde_json::Value,
    #[tool(param)] wiki: Option<String>,
) -> IngestReport { ... }
```

---

## 8. Rust Module Changes

| Module | Change |
|--------|--------|
| `cli.rs` | `ingest` takes `<path>` as primary arg; add `--prefix`, `--update`, `--append`, `--analysis`, `--dry-run`, `--analysis-only` |
| `ingest.rs` | `Input` becomes `Direct(PathBuf)` (default) and `AnalysisOnly(PathBuf)` (legacy); add `DirectIngestOptions { prefix, update, analysis }` |
| `integrate.rs` | Add `integrate_direct_file` and `integrate_direct_folder`; existing `integrate` becomes `integrate_analysis` |
| `markdown.rs` | Add `generate_minimal_frontmatter(title, slug)` for files without frontmatter |
| `server.rs` | Rename `wiki_ingest` → `wiki_ingest_analysis` (legacy); add new `wiki_ingest` as primary tool |

No changes to `analysis.rs`, `search.rs`, `context.rs`, `git.rs`, `graph.rs`,
`contradiction.rs`.

---

## 9. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki ingest --analysis-only <file>` (legacy) | implemented (as `wiki ingest analysis.json`) |
| `wiki ingest <file>` (direct file) | **not implemented** |
| `wiki ingest <folder>` (direct folder) | **not implemented** |
| `wiki ingest <path> --analysis <file>` (enrichment) | **not implemented** |
| `wiki_ingest` MCP tool (primary) | **not implemented** |
| `wiki_ingest_analysis` MCP tool (legacy rename) | implemented |
