---
title: "Ingest Pipeline"
summary: "How wiki ingest deserialises analysis.json, validates it, writes pages, and commits."
read_when:
  - Implementing or debugging the ingest pipeline
  - Understanding action semantics (create/update/append)
  - Diagnosing validation errors
status: active
last_updated: "2026-04-13"
---

# Ingest Pipeline

`wiki ingest <file|->` is the entry point for every write to the wiki.
It is the only command that modifies the on-disk state; everything else is read-only.

---

## Pipeline overview

```
stdin / file
     │
     ▼
ingest::ingest()          src/ingest.rs
  ├─ read JSON
  ├─ parse_analysis()     serde_json → Analysis struct; error with line/col on bad JSON
  ├─ git::init_if_needed  git init if no .git present
  │
  ├─ integrate::integrate()   src/integrate.rs
  │    ├─ validate all slugs  fail-fast before any write
  │    ├─ for each SuggestedPage:
  │    │    create | update | append
  │    └─ for each Contradiction:
  │         write contradictions/{slug}.md
  │
  └─ git::commit()        stage all + commit "ingest: <title> — +N pages"
       │
       ▼
  IngestReport printed to stdout
```

No LLM calls. No network I/O. Every write is a local file operation followed by a git commit.

---

## Deserialisation and validation

### `parse_analysis(json: &str) -> Result<Analysis>`

- Uses `serde_json::from_str`. On failure the error includes the line and column:
  ```
  invalid analysis JSON at line 3, column 12: expected `:` at line 3 column 12
  ```
- `doc_type` is a closed enum. An unknown value produces:
  ```
  unknown variant `academic-paper`, expected one of `research-paper`, `blog-post`, ...
  ```
- `action` on each `SuggestedPage` is similarly a closed enum. Unknown values are
  rejected by serde before any file is written.

### Slug validation

Before any file is written, every slug in `suggested_pages` is passed through
`integrate::validate_slug`:

| Check | Example rejected | Error |
|---|---|---|
| Path traversal | `../etc/passwd` | `slug contains path traversal` |
| Absolute path | `/concepts/foo` | `slug contains path traversal or is absolute` |
| Unknown prefix | `raw/file` | `slug has unknown prefix; expected one of: concepts/, ...` |

Valid prefixes: `concepts/`, `sources/`, `queries/`, `contradictions/`.

Validation runs over the full list **before** any file is written. If any slug is
invalid the function returns an error and the wiki remains unmodified.

---

## Action semantics

### `create`

**Precondition:** `{wiki_root}/{slug}.md` must not exist.

```
generate PageFrontmatter from SuggestedPage fields
write {slug}.md:
  ---
  <generated frontmatter>
  ---

  {body}
```

Frontmatter defaults at creation time:

| Field | Value |
|---|---|
| `status` | `active` |
| `confidence` | `medium` |
| `last_updated` | today (UTC, ISO 8601) |
| `sources` | `[]` |
| `contradictions` | `[]` |

### `update`

**Precondition:** `{slug}.md` must already exist.

Reads the existing file, merges frontmatter fields, writes the new body:

| Field | Rule |
|---|---|
| `title`, `summary`, `tldr` | **Overwrite** — LLM's latest understanding |
| `tags` | **Union** — accumulate, never shrink |
| `read_when` | **Union** — accumulate, never shrink |
| `sources`, `contradictions` | **Preserve** — wiki-owned relationships |
| `status` | **Preserve** — may have been set by a human or lint pass |
| `last_updated` | **Set** to today |
| Body | **Replaced** entirely by the new `body` field |

### `append`

**Precondition:** `{slug}.md` must already exist.

Appends a new section to the end of the existing body:

```
{old_body}

---

{new_body}
```

The `---` horizontal rule marks the boundary between ingest sessions.
The LLM is responsible for writing `new_body` with a clear heading (e.g.
`## New findings from <source>`).

Frontmatter merge rules for `append`:

| Field | Rule |
|---|---|
| `title`, `summary`, `tldr` | **Preserve** — "additive" means the page identity doesn't change |
| `tags`, `read_when` | **Union** — accumulate only |
| `sources`, `contradictions`, `status` | **Preserve** |
| `last_updated` | **Set** to today |

---

## Contradiction pages

If `contradictions[]` is non-empty, `integrate` writes one file per entry under
`contradictions/`:

- Path: `{wiki_root}/contradictions/{slugified-title}.md`
- Title slugification: lowercase, non-alphanumeric replaced by `-`, runs collapsed
- Format: YAML frontmatter (title, type, claim_a/b, source_a/b, dimension,
  epistemic_value, status, optional resolution) + Markdown body (Claim A / Claim B /
  Analysis sections)

If `contradictions[]` is empty, no `contradictions/` directory is created.

---

## `IngestReport`

```rust
pub struct IngestReport {
    pub pages_created: usize,
    pub pages_updated: usize,
    pub pages_appended: usize,
    pub contradictions_written: usize,
    pub title: String,
}
```

Printed to stdout on success:

```
Ingested: Switch Transformer
  created:        2
  updated:        1
  appended:       0
  contradictions: 1
```

Used to build the commit message: `ingest: <title> — +N pages`
where N = `pages_created + pages_updated + pages_appended`.

---

## Error handling

All errors propagate via `anyhow::Result`. The CLI prints `error: <message>` to
stderr and exits with code 1.

| Scenario | Error message |
|---|---|
| Malformed JSON | `invalid analysis JSON at line L, column C: ...` |
| Unknown `doc_type` | `unknown variant 'X', expected one of ...` |
| Path traversal in slug | `slug '../evil' contains path traversal` |
| `create` on existing slug | `action 'create' failed: 'concepts/foo.md' already exists` |
| `update`/`append` on missing slug | `action 'update' failed: 'concepts/foo.md' does not exist (use 'create' instead)` |

---

## Git integration

`git::init_if_needed` runs before `integrate` — if the target directory has no
`.git`, one is initialised. This means `wiki ingest` works on a fresh empty
directory.

`git::commit` stages all changes (`index.add_all(["*"])`) and creates a commit.
The author and committer are set to `wiki <wiki@llm-wiki>`. The initial commit
(no parent) is handled correctly.
