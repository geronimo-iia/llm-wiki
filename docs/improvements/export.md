---
title: "llms Format + wiki_export"
summary: "Add format: llms to wiki_list, wiki_search, wiki_graph for LLM-readable tool responses; wiki_export MCP tool writes a file (all pages, no pagination)."
status: proposed
last_updated: "2026-04-27"
depends_on: confidence
---

# `llms` Format + `wiki_export`

## Problem

Every skill that needs to orient itself before acting burns multiple tool
calls to build a map of what already exists:

- **crystallize** — `wiki_search(query)` × N to check for existing pages
  before deciding create vs. update
- **ingest** — `wiki_search(query)` × 2–4 to find integration points
- **research** — `wiki_search` then graph exploration to assess coverage
- **lint/graph** — `wiki_list(page_size: 100)` to enumerate all pages for
  structural analysis

None of these produce a complete, LLM-readable map. A search misses pages
that don't match the query. A list returns paginated slugs with no content
signal. The current `text` and `json` formats are machine-structured but
not optimized for direct LLM consumption.

A second, distinct problem: there is no way to write a portable snapshot
of the wiki to a file — for the `llms.txt` ecosystem (Cursor, Perplexity,
external LLM tools), for offline analysis, or for CI auditing.

## Goals

1. **`format: "llms"`** on existing tools (`wiki_list`, `wiki_search`,
   `wiki_graph`): a new rendering mode that produces LLM-optimized output
   in the tool response. Pagination and filtering unchanged. Useful during
   a session when the LLM needs a richer orientation step.

2. **`wiki_export(path: "...")`**: a new MCP tool and CLI command that
   writes the full wiki to a file — all pages, no pagination. Response is
   a confirmation report (`path`, `pages_written`, `bytes`), not the
   content. This is the `llms.txt` publishing and audit path.

These are two different things with different purposes. `format: "llms"`
is for session use. `wiki_export` is for file production.

## Solution

### 1. `format: "llms"` on existing tools

**`wiki_list(format: "llms")`**

Returns pages grouped by type, one line per page, with summary. Pagination
unchanged — the LLM calls `wiki_list(format: "llms", page: 1)` etc.

```markdown
## Concepts (12)

- [Agent](wiki://concepts/agent): Autonomous entity that perceives, reasons, and acts.
- [Context Window](wiki://concepts/context-window): Fixed token budget available to a model.
- ...

## Sources (8)

- [Karpathy LLM Wiki](wiki://sources/karpathy-llm-wiki): Original design doc for the session-to-wiki pattern.
- ...
```

Ordering within each group: confidence descending (once improvement #1 is
indexed), then title alphabetical. Types ordered by page count descending.
`archived` pages included but visually de-emphasized with `~~strikethrough~~`.

**`wiki_search(format: "llms")`**

Returns results in `llms-txt` style: `- [title](uri): summary` instead of
the current excerpt block. More compact; drops the BM25 score (not useful
to an LLM reader).

```markdown
- [Mixture of Experts](wiki://concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Switch Transformer](wiki://sources/switch-transformer-2021): Scales to trillion parameters using sparse MoE routing.
```

**`wiki_graph(format: "llms")`**

Natural language description of graph structure instead of Mermaid/DOT
code. Directly readable without a renderer.

```markdown
The wiki graph has 42 nodes and 87 edges across 5 clusters.

**Concepts** (18 nodes): Agent, Context Window, Mixture of Experts, ...
Key hubs: Mixture of Experts (12 edges), Scaling Laws (9 edges)

**Sources** (14 nodes): Karpathy LLM Wiki, Switch Transformer, ...

**Edges by relation:**
- `informs` (32): source pages feeding concept pages
- `depends-on` (28): concept dependency chains
- `fed-by` (18): concepts citing their source pages
- `links-to` (9): body wikilinks

**Isolated nodes (3):** draft-stub-xyz, tangent-note-abc, ...
```

This is what the graph skill's "Interpret the graph" section currently asks
the LLM to derive manually from Mermaid output — `format: "llms"` produces
it directly from the engine.

### 2. `wiki_export` tool

Writes the full wiki to a file. All pages, no pagination. Response is a
report, not the content.

```
wiki_export(wiki: "name")                                      — llms-txt, writes <wiki-root>/llms.txt
wiki_export(wiki: "name", path: "llms.txt")                    — explicit path (relative to wiki root)
wiki_export(wiki: "name", path: "/abs/path/llms.txt")          — absolute path
wiki_export(wiki: "name", format: "llms-full", path: "llms-full.txt") — with full page bodies
wiki_export(wiki: "name", format: "json",     path: "wiki.json")      — JSON array of all pages
wiki_export(wiki: "name", status: "all")                       — include archived pages
```

**Formats:**

| `format` | Content | Use case |
|---|---|---|
| `llms-txt` (default) | Grouped summary, one line per page | `llms.txt` ecosystem publishing, offline orientation |
| `llms-full` | Summary + full body per page, separated by `---` | Long-context offline analysis |
| `json` | JSON array of all page metadata + body | Batch processing scripts |

**Response (MCP tool):**
```json
{
  "path": "/home/user/wiki/llms.txt",
  "pages_written": 87,
  "bytes": 28419,
  "format": "llms-txt"
}
```

**`wiki` is required.** The engine needs to know which wiki root to resolve
the path against and which index to walk.

**Path resolution:** `path` is relative to the wiki root when not absolute.
Default when `path` is omitted: `llms.txt` at the wiki repository root
(`<wiki-root>/llms.txt`). This file can be committed to git, served by
Hugo, or picked up by external `llms.txt` ecosystem tools without the
caller needing to know the filesystem path.

**CLI:**
```
llm-wiki export --path llms.txt [--format llms-txt|llms-full|json] [--wiki name] [--status active|all]
```

Writes to `--path`. No stdout output (unlike `wiki_graph` which defaults
to stdout) — export output is too large to be useful in a terminal pipe.

### Implementation: `src/ops/export.rs`

1. Walk tantivy index via `AllQuery`; collect all pages as
   `(slug, uri, title, type, status, confidence, summary)`.
2. Apply status filter (default: exclude `archived`).
3. Sort: type groups by page count desc; within group by confidence desc,
   then title asc.
4. For `llms-full` and `json`: read page body from disk for each page.
5. Render to selected format.
6. Write to resolved path; return `ExportReport { path, pages_written, bytes }`.

The `llms` format renderer for `wiki_list` and `wiki_search` lives in
`src/search.rs` / `src/ops/list.rs` as a new `Format::Llms` variant —
same rendering logic, called from the existing result structs.

The `wiki_graph(format: "llms")` renderer lives in `src/graph.rs` as
a new `render_llms(graph: &WikiGraph) -> String` function alongside the
existing `render_mermaid` and `render_dot`.

## Skill updates

### All skills — orientation pattern

`wiki_list(format: "llms")` replaces multi-search orientation where the
goal is "what does the wiki contain?" rather than "find pages about X".

**crystallize** — add `wiki_list(format: "llms")` as the first step before
"Search for an existing home". One call produces the full type-grouped map;
targeted `wiki_search` only for candidates found there.

**ingest** — in step 2c "Find integration points", add `wiki_list(format: "llms")`
as the recommended first call for whole-wiki orientation when processing the
first file in a session. Subsequent files in the same session can skip it.

**research** — add `wiki_list(format: "llms")` as an optional first step
when the question is broad ("what does the wiki know about X?") or when
coverage assessment is the goal. For narrow queries, `wiki_search` alone
is sufficient.

**lint** — replace `wiki_list(page_size: 100)` with
`wiki_list(format: "llms")` for the structural enumeration step. The
type grouping and summaries are richer input for gap analysis.

**graph** — replace manual Mermaid interpretation guidance with
`wiki_graph(format: "llms")` for the "Interpret the graph" section. The
natural language output surfaces clusters, hubs, and isolated nodes
directly without requiring the LLM to parse Mermaid syntax.

## Branch & PR — `llm-wiki`

```bash
git checkout -b feat/llms-export
```

When implementation is complete and all tests pass:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

```bash
git push -u origin feat/llms-export
gh pr create \
  --title "feat: llms format on wiki_list/search/graph + wiki_export tool" \
  --milestone "v0.2.0" \
  --body "$(cat <<'EOF'
Implements imp-9 (llms format + wiki_export).

- Add format: "llms" to wiki_list, wiki_search, wiki_graph
- Add wiki_export MCP tool and CLI export subcommand
- Formats: llms-txt (default), llms-full, json
- Path resolution relative to wiki root; default llms.txt

Closes geronimo-iia/llm-wiki#22 (imp-9)

Spec: docs/improvements/export.md
EOF
)"
```

## Branch & PR — `llm-wiki-skills`

```bash
# in llm-wiki-skills repo
git checkout -b feat/llms-export
```

When done:

```bash
git push -u origin feat/llms-export
gh pr create \
  --repo geronimo-iia/llm-wiki-skills \
  --milestone "v0.4.0" \
  --title "feat: adopt llms format in crystallize, ingest, research, lint, graph skills" \
  --body "$(cat <<'EOF'
Aligns skill documentation with the llms format feature added in llm-wiki imp-9.

- crystallize/SKILL.md: wiki_list(format: "llms") as first orientation step
- ingest/SKILL.md: wiki_list(format: "llms") for first-file orientation
- research/SKILL.md: Orient section using wiki_list(format: "llms") for broad queries
- lint/SKILL.md: replace wiki_list(page_size: 100) with wiki_list(format: "llms")
- graph/SKILL.md: wiki_graph(format: "llms") for Interpret section

Companion to llm-wiki feat/llms-export (imp-9).
Closes geronimo-iia/llm-wiki-skills#1 (imp-9)
EOF
)"
```

> Merge timing: skills PR requires `llm-wiki` imp-9 to be merged first.

## Tasks

### Engine — `src/graph.rs`

- [ ] Add `pub fn render_llms(graph: &WikiGraph) -> String`: natural language
  summary — node count, edge count, cluster count (if `compute_communities`
  is available), nodes grouped by type, top hubs by degree, edge relation
  counts, isolated nodes list.
- [ ] Add `"llms"` as a valid `format` option in the graph tool/CLI; route
  to `render_llms`.

### Engine — `src/search.rs` / `src/ops/list.rs`

- [ ] Add `Format::Llms` variant to the list/search output format enum.
- [ ] Implement `llms` renderer for `ListResult`: group pages by type (count
  desc), one line per page `- [title](uri): summary`, `~~title~~` for
  archived.
- [ ] Implement `llms` renderer for `SearchResult`: `- [title](uri): summary`
  per result, no score, no excerpt block.
- [ ] Add `--format llms` to `llm-wiki list` and `llm-wiki search` CLI.

### Engine — `src/ops/export.rs`

- [ ] Create `src/ops/export.rs`; define `ExportOptions { wiki, path, format,
  status }` and `ExportFormat { LlmsTxt, LlmsFull, Json }`.
- [ ] Implement `fn export(engine, options) -> Result<ExportReport>`: walk
  index, filter, sort, render, write to resolved path.
- [ ] `llms-txt` renderer: same output as `Format::Llms` on `wiki_list` but
  unbounded (all pages), with wiki name + description header.
- [ ] `llms-full` renderer: `llms-txt` output + full page body per entry
  separated by `---`; read body from disk.
- [ ] `json` renderer: JSON array of `{ slug, uri, title, type, status,
  confidence, summary, body }`.
- [ ] Path resolution: relative paths resolved against wiki root.
- [ ] Return `ExportReport { path: String, pages_written: usize, bytes: usize, format: String }`.

### Engine — MCP + CLI

- [ ] Add `wiki_export` to `src/tools.rs` with parameters `wiki` (required),
  `path` (optional, default `"llms.txt"` relative to wiki root), `format`,
  `status`.
- [ ] Add `export` subcommand to `src/cli.rs` with `--wiki` (required),
  `--path` (optional, default `llms.txt` at wiki root), `--format`,
  `--status`.

### Spec docs

- [ ] Update `docs/specifications/tools/list.md`: add `llms` to `--format`
  options; document grouped output format.
- [ ] Update `docs/specifications/tools/search.md`: add `llms` to
  `--format` options; document `- [title](uri): summary` output.
- [ ] Update `docs/specifications/tools/graph.md`: add `llms` to
  `--format` options; document natural language output structure.
- [ ] Create `docs/specifications/tools/export.md`: document `wiki_export`
  parameters, formats, path resolution, response struct.

### Skill — `llm-wiki-skills/skills/crystallize/SKILL.md`

- [ ] Add `wiki_list(format: "llms")` as the first step in
  `## Search for an existing home`; retain `wiki_search` for targeted
  follow-up.
- [ ] Update `metadata.version` to `0.3.0` and `last_updated`.

### Skill — `llm-wiki-skills/skills/ingest/SKILL.md`

- [ ] In step 2c, add `wiki_list(format: "llms")` as the first-file
  orientation call; note subsequent files in the same session can skip it.
- [ ] Update `metadata.version` to `0.4.0` and `last_updated`.

### Skill — `llm-wiki-skills/skills/research/SKILL.md`

- [ ] Add `## Orient` section before `## Search`: use
  `wiki_list(format: "llms")` for broad/coverage questions; use
  `wiki_search` for narrow queries.
- [ ] Update `metadata.version` to `0.3.0` and `last_updated`.

### Skill — `llm-wiki-skills/skills/lint/SKILL.md`

- [ ] Replace `wiki_list(page_size: 100)` with `wiki_list(format: "llms")`
  for structural enumeration.
- [ ] Update `metadata.version` to `0.3.0` and `last_updated`.

### Skill — `llm-wiki-skills/skills/graph/SKILL.md`

- [ ] Replace manual Mermaid interpretation guidance in
  `## Interpret the graph` with `wiki_graph(format: "llms")` as the
  primary interpretation call; retain `wiki_graph(format: "mermaid")` for
  visualization use cases.
- [ ] Update `metadata.version` to `0.3.0` and `last_updated`.

### Tests

- [ ] `render_llms` on a graph with 3 type groups → correct section headers,
  hub list, relation counts.
- [ ] `Format::Llms` on `wiki_list`: pages grouped by type, archived
  page rendered with strikethrough.
- [ ] `wiki_export(format: "llms-txt")`: all pages written, response
  contains correct `pages_written` count.
- [ ] `wiki_export(format: "llms-full")`: each page entry followed by body
  and `---` separator.
- [ ] Path resolution: relative path resolves to wiki root.
- [ ] `wiki_export(status: "all")` includes archived pages;
  default excludes them.

### Issue update
After merging:
- Check off imp-9 in `geronimo-iia/llm-wiki#22` and `geronimo-iia/llm-wiki-skills#1`
- Mark `status: implemented` in this file
