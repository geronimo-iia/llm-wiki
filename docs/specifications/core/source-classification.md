---
title: "Source Classification"
summary: "How source types work within the unified page type taxonomy — source pages use specific types instead of a generic source-summary type."
read_when:
  - Understanding how sources are typed during ingest
  - Configuring custom source types for a wiki instance
  - Implementing type-based filtering in search or lint
status: active
last_updated: "2025-07-15"
---

# Source Classification

Source classification is folded into the `type` field. Instead of
`type: source-summary` with a separate `classification: paper`, source pages
use the specific type directly: `type: paper`.

This eliminates a field, simplifies filtering (`--type paper` instead of
`--type source-summary --classification paper`), and makes the type system
the single axis for page categorization.

---

## 1. Source Types

Source types represent what a specific document claims — provenance pages.
They can live anywhere in the wiki tree; folder structure is organizational,
not epistemic.

| Type | Source nature | Description |
|------|-------------|-------------|
| `paper` | Academic | Research papers, preprints, academic material |
| `article` | Editorial | Blog posts, news, long-form essays, opinion pieces |
| `documentation` | Reference | Product docs, API references, technical specifications |
| `clipping` | Web capture | Web saves, browser clips, bookmarks with content |
| `transcript` | Spoken | Meeting transcripts, podcast transcripts, interviews |
| `note` | Informal | Freeform drafts, quick captures, personal notes |
| `data` | Structured | CSV, JSON, structured datasets, spreadsheets |
| `book-chapter` | Published | Excerpts or chapters from books |
| `thread` | Discussion | Forum threads, social media threads, discussion archives |

### Classification rule

Classify by the source material's nature, not its topic. A blog post about
academic research is `article`, not `paper`. A PDF of API docs is
`documentation`, not `paper`.

---

## 2. Custom Types

Wiki owners can add domain-specific types in `schema.md`:

```yaml
types:
  - patent
  - legal-filing
  - specification
  - meeting-notes
```

Custom types are additive — they extend the built-in list. The engine
validates `type` against the combined list (built-in + custom) on ingest,
according to the `validation.type_strictness` setting in `wiki.toml` or
`~/.wiki/config.toml`:

| Strictness | Unknown type behavior |
|------------|----------------------|
| `loose` (default) | Warning — ingest proceeds with the type as-is |
| `strict` | Error — ingest rejected |

See [configuration.md](../commands/configuration.md) for the full config reference.

---

## 3. Search and List Integration

`wiki search` and `wiki list` filter by `type` directly:

```bash
wiki search "MoE scaling" --type paper
wiki list --type documentation
wiki list --type paper,article          # multiple types
```

No separate `--classification` flag needed.

The tantivy index includes `type` as a filterable field (already the case).

---

## 4. Lint Integration

Lint checks for pages with source-like content but unrecognized or missing types:

### Untyped source pages

Pages without a recognized source type that appear to be source summaries
are flagged:

```markdown
## Untyped Sources (2)

| slug | current type |
|------|-------------|
| sources/random-blog-post | (missing) |
| sources/meeting-notes-2025-03 | source-summary |

_Set a specific source type: paper, article, documentation, etc._
```

`source-summary` is flagged as a legacy type that should be replaced with
a specific source type.

---

## 5. Migration from `classification` Field

The previous design used `type: source-summary` + `classification: paper`.
This is replaced by `type: paper` directly.

| Before | After |
|--------|-------|
| `type: source-summary` + `classification: paper` | `type: paper` |
| `type: source-summary` + `classification: article` | `type: article` |
| `--type source-summary --classification paper` | `--type paper` |

The `classification` frontmatter field is removed. The `source-summary`
type is deprecated — lint flags it.

---

## 6. Epistemic Model Impact

The [epistemic model](epistemic-model.md) separates sources from concepts
via the `type` field:

```
type: paper, article, etc.  → what each source claims (provenance)
type: concept               → what we know (synthesized knowledge)
```

This separation is carried entirely by `type`. Source types all serve the
provenance role regardless of which folder they live in. The type field
carries both the epistemic role (it's a source) and the source nature
(it's a paper) in one value.

To query "all sources": `wiki list --type paper,article,documentation,clipping,transcript,note,data,book-chapter,thread`
or filter by whatever folder convention the wiki uses.
