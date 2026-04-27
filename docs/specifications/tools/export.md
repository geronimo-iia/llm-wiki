---
title: "Export"
summary: "Export the full wiki to a file — llms.txt, llms-full, or JSON."
read_when:
  - Exporting wiki for llms.txt ecosystem
  - Offline analysis or CI auditing
  - Batch processing scripts
status: ready
last_updated: "2026-04-27"
---

# Export

MCP tool: `wiki_export`

```
llm-wiki export
          [--path <path>]           # output path (default: llms.txt at wiki root)
          [--format <fmt>]          # llms-txt | llms-full | json (default: llms-txt)
          [--status <filter>]       # active | all (default: active)
          [--wiki <name>]
```

Writes the full wiki to a file. All pages, no pagination. Response is
a report (`path`, `pages_written`, `bytes`, `format`), not the content.

This is the publishing and audit path — distinct from `format: "llms"` on
`wiki_list` / `wiki_search` / `wiki_graph`, which produce LLM-optimized
tool responses for session use.

## Parameters

| Parameter | Required | Default | Description |
|-----------|----------|---------|-------------|
| `wiki` | yes | — | Target wiki name |
| `path` | no | `llms.txt` | Output path — relative to wiki root or absolute |
| `format` | no | `llms-txt` | Export format (see table below) |
| `status` | no | `active` | `active` excludes archived; `all` includes them |

## Formats

| `format` | Content | Use case |
|----------|---------|----------|
| `llms-txt` (default) | Grouped summary, one line per page | `llms.txt` ecosystem publishing, offline orientation |
| `llms-full` | Summary + full body per page, separated by `---` | Long-context offline analysis |
| `json` | JSON array of all page metadata + body | Batch processing scripts |

## Path resolution

`path` is resolved relative to the wiki root when not absolute.
Default when `path` is omitted: `llms.txt` at the wiki repository root
(`<wiki-root>/llms.txt`). This file can be committed to git, served by
Hugo, or picked up by external `llms.txt` ecosystem tools without the
caller needing to know the filesystem path.

## Output: `llms-txt`

Wiki name header, total page count, pages grouped by type (count desc),
one line per page with summary:

```markdown
# research

42 pages

## concept (18)

- [Mixture of Experts](wiki://research/concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Scaling Laws](wiki://research/concepts/scaling-laws): Empirical laws relating model size, data, and compute to performance.

## paper (14)

- [Switch Transformer](wiki://research/sources/switch-transformer-2021): Scales to trillion parameters using sparse MoE routing.
```

Within each type group, pages are ordered by `confidence` desc, then title asc.

## Output: `llms-full`

Same as `llms-txt` but each entry is preceded by `---` and followed by
the full page body (frontmatter stripped):

```markdown
# research

42 pages

---

# [Mixture of Experts](wiki://research/concepts/mixture-of-experts)

_Sparse routing of tokens to expert subnetworks._

Mixture of Experts (MoE) is a technique that routes tokens to a subset of
expert subnetworks...
```

## Output: `json`

JSON array of page objects. Each object includes metadata and body:

```json
[
  {
    "slug": "concepts/mixture-of-experts",
    "uri": "wiki://research/concepts/mixture-of-experts",
    "title": "Mixture of Experts",
    "type": "concept",
    "status": "active",
    "confidence": 0.9,
    "summary": "Sparse routing of tokens to expert subnetworks.",
    "body": "Mixture of Experts (MoE) is a technique..."
  }
]
```

## Response (MCP tool)

```json
{
  "path": "/home/user/wiki/llms.txt",
  "pages_written": 42,
  "bytes": 18340,
  "format": "llms-txt"
}
```

## Sorting

Pages are sorted: type groups by page count desc (largest groups first),
within group by `confidence` desc, then title asc. Ensures the most
populated and highest-confidence content appears first.
