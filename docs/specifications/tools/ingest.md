---
title: "Ingest"
summary: "Validate, index, and optionally commit."
read_when:
  - Ingesting content into the wiki
status: ready
last_updated: "2025-07-17"
---

# Ingest

MCP tool: `wiki_ingest`

```
llm-wiki ingest <slug|uri>              # file or folder
            [--dry-run]
            [--redact]                  # opt-in redaction pass (lossy)
            [--format <fmt>]            # text | json (default: from config)
            [--wiki <name>]
```

Validates frontmatter, updates the search index, and commits to git
when `ingest.auto_commit` is true. Accepts a bare slug or `wiki://`
URI. When a `wiki://` URI is used, `--wiki` is ignored.

For the full pipeline, see
[ingest-pipeline.md](../engine/ingest-pipeline.md).

### Output

Text (default):

```
Ingested: 3 pages, 497 unchanged, 0 assets, 0 warnings
Commit: a3f9c12
```

JSON (`--format json`):

```json
{
  "pages_validated": 3,
  "unchanged_count": 497,
  "assets_found": 0,
  "warnings": [],
  "commit": "a3f9c12",
  "redacted": [
    {
      "slug": "inbox/transcript",
      "matches": [
        { "pattern_name": "github-pat", "line_number": 14 }
      ]
    }
  ]
}
```

`redacted` is an empty array when `redact: false` (default) or when no
patterns matched. `#[serde(default)]` — absent from older responses.

`commit` is empty when `ingest.auto_commit` is false.

### Validation scope

Normal ingest (`dry_run: false`) narrows validation to files that are new or
modified since the last indexed commit. `unchanged_count` reports how many
`.md` files were skipped. Files already in git passed validation when first
ingested — the commit is the proof of prior validation.

| Call site | Validated files |
|---|---|
| `wiki_ingest <path>` (normal) | Git-changed since last indexed commit |
| `wiki_ingest <path> --dry-run` | All files (explicit full audit) |
| `wiki_index_rebuild` | None — index rebuild only |

**Fallback:** when `last_commit` is absent (fresh wiki, first ingest) or the
git query returns an error, all files are validated.
