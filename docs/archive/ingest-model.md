# Ingest Model

Content enters the wiki through a single command: `wiki ingest`. Three modes
cover every use case — from ingesting a folder of existing files with no LLM
involved, to applying LLM-produced enrichments on top of existing pages.

---

## The Core Principle

**Files first, enrichment optional.**

The wiki is a structured copy of existing content. The LLM annotates pages
it did not write. This is the opposite of the original Karpathy model where
the LLM authored page bodies — a deliberate evolution driven by the reality
that most valuable content already exists as structured files.

A wiki of direct-ingested pages is immediately useful for search and context
retrieval, with no LLM step required.

---

## Three Ingest Modes

### Mode A — Direct (default)

```
wiki ingest path/to/file.md
wiki ingest path/to/folder/
```

Files ingested as-is. No LLM step. Frontmatter preserved if present; minimal
frontmatter generated if absent. Non-Markdown files co-located with their page
as bundle assets.

Right for: skills, guides, specs, reference folders, any already-structured
content.

### Mode B — Direct + Enrichment

```
wiki ingest path/to/folder/ --analysis enrichment.json
```

Files ingested first, then the enrichment JSON applied on top. Claims,
concepts, confidence, and contradictions are merged into page frontmatter.
The page body is never touched.

Right for: ingesting a source and immediately enriching it with LLM analysis.

### Mode C — Enrichment Only (legacy)

```
wiki ingest --analysis-only enrichment.json
```

No file path. Pure enrichment pipeline. Right for: sources with no local
files (URLs, transcripts, remote papers already in `raw/`).

All modes produce a git commit and update the search index.

---

## What Gets Written

### Pages

A page is a Markdown file with YAML frontmatter. Two forms:

**Flat** — `concepts/mixture-of-experts.md` — page with no assets.

**Bundle** — `concepts/mixture-of-experts/` folder containing `index.md`
and co-located assets. Created automatically when a page has assets.

Slug derivation for direct ingest: path relative to the ingest source root,
preserving directory structure. `my-skill/SKILL.md` with `--prefix skills`
becomes slug `skills/my-skill`.

### Assets

Non-Markdown files (images, YAML, scripts, data) are co-located with their
page in the bundle folder — not in a central `assets/` directory. The folder
becomes a bundle automatically.

```
skills/semantic-commit/
├── index.md          ← from SKILL.md
├── lifecycle.yaml    ← co-located
└── install.sh        ← co-located
```

Shared assets (referenced by multiple pages) go under `assets/` with
subdirectory by kind: `assets/diagrams/`, `assets/configs/`, `assets/scripts/`.

### Contradictions

Contradiction pages are written by the enrichment pipeline when the LLM
detects a tension between sources. They are never written by direct ingest.
Each contradiction page carries two claims, a dimension, an epistemic value,
and a status.

---

## The Enrichment Contract

When an LLM enriches existing pages, it produces an enrichment JSON with
three arrays:

**`enrichments[]`** — metadata additions to existing pages. Each entry
targets a page by slug and merges claims, concepts, tags, confidence, and
source references into the frontmatter. The body is never touched.

**`query_results[]`** — LLM-authored pages. The one case where the LLM
writes a page body: saving a synthesized answer as a `queries/` page.

**`contradictions[]`** — detected tensions between sources. The LLM must
call `wiki context` first to know what pages exist before writing contradictions.

---

## Git as the Audit Trail

Every ingest session produces a git commit. The commit message records what
changed: `ingest(direct): semantic-commit — +2 pages, +1 asset`.

`git log` shows the full history of how the wiki evolved. `git diff HEAD~1`
shows exactly what the last ingest changed. `git revert` rolls back a bad
enrichment pass.

---

## Slug Validation

Two validation levels:

**Strict** (enrichment-only ingest) — slugs must start with a fixed category
prefix: `concepts/`, `sources/`, `queries/`, `contradictions/`. Path traversal
rejected.

**Relaxed** (direct ingest) — any prefix allowed. User-defined prefixes like
`skills/` or `guides/` are valid. Path traversal and absolute paths rejected.

---

## Dry Run

```
wiki ingest path/to/folder/ --dry-run
```

Shows what would be written without committing anything. Useful for verifying
slug derivation and asset placement before a real ingest.
