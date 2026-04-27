---
title: "Lint"
summary: "wiki_lint MCP tool — 5 deterministic index-based rules."
read_when:
  - Checking wiki health programmatically
  - Running CI gates
  - Understanding what wiki_lint returns
status: ready
last_updated: "2026-04-27"
---

# Lint

## wiki_lint

MCP tool: `wiki_lint`

```
wiki_lint()
wiki_lint(rules: "orphan,stale")     — subset of rules
wiki_lint(severity: "error")         — filter to errors only
wiki_lint(wiki: "name")              — target a specific wiki
```

Runs deterministic lint rules against the tantivy index. All rules are
pure index queries — no file I/O, no LLM involvement. Fast and safe
to run in CI.

Returns a JSON object:

```json
{
  "wiki": "research",
  "total": 3,
  "errors": 2,
  "warnings": 1,
  "findings": [
    {
      "slug": "concepts/moe",
      "rule": "broken-link",
      "severity": "error",
      "message": "broken link in body_links: concepts/ghost"
    }
  ]
}
```

Empty `findings` array = clean wiki. CLI exits non-zero when any
`error` finding exists.

## Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `rules` | string | all | Comma-separated rule names to run |
| `severity` | string | all | Filter output: `error` \| `warning` |
| `wiki` | string | default | Target wiki name |

## Rules

| Rule ID | Severity | Description |
|---------|----------|-------------|
| `orphan` | warning | Page has no incoming links and is not a section page |
| `broken-link` | error | A slug in `body_links` or frontmatter edge fields does not exist in the index |
| `missing-fields` | error | Required frontmatter fields (per type schema) are absent |
| `stale` | warning | `last_updated` older than threshold AND `confidence` below threshold |
| `unknown-type` | error | `type` field is not registered in the type registry |

### orphan

Builds a set of all slugs referenced in `body_links`, `sources`,
`concepts`, `document_refs`, and `superseded_by` across all pages.
Any page slug not in that set is an orphan. Section pages (`type:
section`) and root index pages are exempt.

### broken-link

For each page, checks every value in `body_links`, `sources`,
`concepts`, `document_refs`, and `superseded_by` against the index.
Cross-wiki `wiki://` URIs are skipped (those are imp-10's concern).

### missing-fields

Looks up the required fields for the page's type via the type registry,
then checks each field's presence in the stored document. Fields not
present in the index schema are skipped.

### stale

Parses `last_updated` as `YYYY-MM-DD`. A page is stale when **both**
conditions hold:

1. `last_updated` is older than `stale_days` (default: 90)
2. `confidence` is below `stale_confidence_threshold` (default: 0.4),
   or confidence is absent

A page that is old but has `confidence: 0.9` is **not** stale. A page
with no valid `last_updated` date is treated as infinitely old.

### unknown-type

Checks the `type` field value against `SpaceTypeRegistry::is_known()`.
Empty type fields are skipped.

## Configuration

`[lint]` in `config.toml` (global) or `wiki.toml` (per-wiki override):

```toml
[lint]
stale_days                 = 90    # days before a page is considered old
stale_confidence_threshold = 0.4   # confidence below this + old = stale
```

Per-wiki `[lint]` replaces the global value entirely (not merged).

## CLI

```
llm-wiki lint
         [--rules orphan,stale]     # subset of rules
         [--severity error]         # filter output
         [--format json|text]       # default: text
         [--wiki <name>]
```

CLI exits non-zero when any `error` finding exists. Suitable as a
CI gate:

```bash
llm-wiki lint --severity error && echo "wiki is clean"
```
