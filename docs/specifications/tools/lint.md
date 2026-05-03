---
title: "Lint"
summary: "wiki_lint MCP tool — 8 rules (5 index-based, 3 graph-structural)."
read_when:
  - Checking wiki health programmatically
  - Running CI gates
  - Understanding what wiki_lint returns
status: ready
last_updated: "2026-05-04"
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

Runs lint rules against the tantivy index and wiki graph. The 5 index-based
rules are pure index queries — no file I/O. The 3 structural rules
(`articulation-point`, `bridge`, `periphery`) build the wiki graph on demand
and are safe to run in CI (cached after first build).

Returns a JSON object:

```json
{
  "wiki": "research",
  "total": 3,
  "errors": 2,
  "warnings": 1,
  "findings": [
    {
      "slug":     "concepts/moe",
      "rule":     "broken-link",
      "severity": "error",
      "message":  "broken link in body_links: concepts/ghost",
      "path":     "/path/to/wiki/concepts/moe.md"
    }
  ]
}
```

Each finding includes `path` — the absolute filesystem path to the offending
file. Use it to `Edit` the file directly without a follow-up `wiki_resolve` call.

Empty `findings` array = clean wiki. CLI exits non-zero when any
`error` finding exists.

## Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `rules` | string | all | Comma-separated rule names: `orphan`, `broken-link`, `broken-cross-wiki-link`, `missing-fields`, `stale`, `unknown-type`, `articulation-point`, `bridge`, `periphery` |
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
| `articulation-point` | warning | Page whose removal disconnects the graph (undirected view) |
| `bridge` | warning | Link whose removal disconnects the graph (undirected view) |
| `periphery` | warning | Most structurally isolated page — eccentricity equals diameter; skipped when `local_count > graph.max_nodes_for_diameter` |

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

### articulation-point

Builds a symmetrized undirected view of the wiki graph (external placeholder
nodes excluded), then runs Tarjan DFS to find articulation points — nodes whose
removal increases the number of connected components.

A high articulation-point count signals fragile graph topology. Fix by adding
alternative link paths that bypass each flagged page.

### bridge

Same undirected graph as `articulation-point`. Reports edges whose removal
disconnects the graph.

Fix by creating at least one parallel path between the two components on either
side of each bridge.

### periphery

Runs BFS from every local node on the directed `WikiGraph` to compute
eccentricities. Pages with eccentricity equal to the diameter are in the
periphery — maximally isolated from the rest of the graph.

Skipped entirely when `local_count > graph.max_nodes_for_diameter` (default 2000).
Use `--rules periphery` to confirm whether it ran or was skipped.

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
         [--rules orphan,stale,articulation-point]  # subset of rules
         [--severity error]         # filter output
         [--format json|text]       # default: text
         [--wiki <name>]
```

CLI exits non-zero when any `error` finding exists. Suitable as a
CI gate:

```bash
llm-wiki lint --severity error && echo "wiki is clean"
```
