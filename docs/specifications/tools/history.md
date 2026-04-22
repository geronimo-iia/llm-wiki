---
title: "History"
summary: "Git commit history for a page."
read_when:
  - Checking when a page was last modified
  - Reviewing change history for a page
  - Assessing page freshness
status: ready
last_updated: "2025-07-22"
---

# History

MCP tool: `wiki_history`

```
llm-wiki history <slug|uri>
            [--limit <n>]           # default: from config
            [--no-follow]           # disable rename tracking
            [--format <fmt>]        # text | json (default: text)
            [--wiki <name>]
```

Returns the git commit history for a specific page. Accepts a bare
slug or `wiki://` URI.

For flat pages, logs the `.md` file. For bundles, logs the `index.md`.
For sections, logs the section's `index.md`.

When `history.follow` is true (default), rename tracking is enabled
— history follows the file across flat→bundle migration or slug
renames. `--no-follow` disables this per invocation.

### Examples

```bash
llm-wiki history concepts/moe
llm-wiki history concepts/moe --limit 5
llm-wiki history wiki://research/concepts/moe --no-follow
```

### Output

Text (default):

```
a3f9c12  2025-07-21  ingest: concepts/moe.md          Jerome Guibert
b7e4d56  2025-07-18  create: research                  Jerome Guibert
```

JSON (`--format json`):

```json
{
  "slug": "concepts/moe",
  "entries": [
    {
      "hash": "a3f9c12",
      "date": "2025-07-21T14:32:01Z",
      "message": "ingest: concepts/moe.md",
      "author": "Jerome Guibert"
    },
    {
      "hash": "b7e4d56",
      "date": "2025-07-18T10:15:00Z",
      "message": "create: research",
      "author": "Jerome Guibert"
    }
  ]
}
```

## MCP Tool Definition

```json
{
  "name": "wiki_history",
  "description": "Git commit history for a page",
  "parameters": {
    "slug": "(required) slug or wiki:// URI",
    "limit": "max entries to return (default: from config)",
    "follow": "track renames (default: from config)",
    "wiki": "target wiki name (default: default wiki)"
  }
}
```
