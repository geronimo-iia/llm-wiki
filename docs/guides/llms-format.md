---
title: "LLM-Optimized Output"
summary: "When and how to use format: llms on wiki_list, wiki_search, wiki_graph, and wiki_export."
read_when:
  - Orienting an LLM agent before acting on the wiki
  - Choosing between format options for a tool call
  - Publishing a wiki snapshot for external LLM tools
status: ready
last_updated: "2026-04-27"
---

# LLM-Optimized Output

`format: "llms"` is a rendering mode available on `wiki_list`,
`wiki_search`, and `wiki_graph`. It produces output shaped for direct
LLM consumption — grouped, annotated, and compact — instead of machine-
structured JSON or a diagram syntax requiring a renderer.

`wiki_export` writes the same rendering to a file for the `llms.txt`
ecosystem, offline analysis, or CI auditing. It is the file-production
counterpart to the session-use `format: "llms"`.

## The `llms.txt` standard

`llms.txt` is a community convention proposed by Jeremy Howard
(Answer.AI / fast.ai) for making project documentation accessible to
LLMs. The canonical specification and reference implementation are at:

- Spec: <https://llmstxt.org>
- Reference implementation: <https://github.com/AnswerDotAI/llms-txt>

The convention places a Markdown file at `<site>/llms.txt` — analogous
to `robots.txt` — containing a project name (H1), an optional summary
blockquote, and file lists pointing to key documentation pages. An
extended variant, `llms-full.txt`, inlines the full content of those
pages for long-context consumption.

`wiki_export` produces files that follow this structure: `llms-txt`
generates the summary listing (equivalent to `llms.txt`), `llms-full`
generates the inlined-bodies variant (equivalent to `llms-full.txt`).
The exported file can be committed to the wiki repository root and
served by Hugo or any static host, making the wiki's knowledge directly
available to tools like Cursor, Perplexity, and other LLM clients that
support the `llms.txt` convention.

## The orientation problem

Skills that need to act on an existing wiki before writing spend tool
calls building a map:

- `wiki_search(query: "X")` × N — misses pages that don't match the query
- `wiki_list(page_size: 100)` — paginated slugs with no content signal
- `wiki_graph()` → parse Mermaid manually — indirect and error-prone

None of these produce a complete, directly readable map in one call.
`format: "llms"` solves this.

## When to use each format

### `wiki_list(format: "llms")`

Use when the goal is **"what does this wiki contain?"** before deciding
what to create or update.

One call returns all pages on the current page, grouped by type with
summaries. Pagination is unchanged — call `page: 2` etc. if the wiki
is large.

**Use it in:** crystallize (before searching for an existing home),
ingest (first-file orientation), lint (gap analysis before judgment-
based checks), research (broad coverage questions).

**Don't use it when:** you have a specific topic to find — use
`wiki_search` instead.

### `wiki_search(format: "llms")`

Use when you want search results **without excerpt noise** — compact
`- [title](uri): summary` lines instead of the scored excerpt block.

Useful when feeding results into a prompt where the score and HTML
excerpt add no value and consume tokens.

**Use it in:** research (when you want a clean list of candidates to
read, not scored excerpts), crystallize (targeted follow-up after
orientation).

### `wiki_graph(format: "llms")`

Use when you need to **interpret graph structure** — clusters, hubs,
isolated nodes, relation counts — without parsing Mermaid syntax.

The output is a natural language paragraph + bullet summary that
surfaces what a visual graph would show but requires no renderer.

**Use it in:** graph skill's "Interpret the graph" step; any time
you want a structural digest rather than a diagram.

**Use `mermaid` or `dot` when:** the goal is a renderable visualization
for a user or document.

### `wiki_export`

Use when you need a **file** — for the `llms.txt` ecosystem, offline
analysis, or CI auditing. Response is a report, not the content.

```
wiki_export(wiki: "research")                           # → <wiki-root>/llms.txt
wiki_export(wiki: "research", format: "llms-full")      # with full bodies
wiki_export(wiki: "research", format: "json")           # JSON array
wiki_export(wiki: "research", status: "all")            # include archived
```

This is not a session tool — it writes to disk and is most useful for
publishing, sharing, or batch processing outside the session.

## Output formats at a glance

| Tool | `format: "llms"` output | Use case |
|------|------------------------|----------|
| `wiki_list` | Pages grouped by type, one line each with summary | Pre-action orientation |
| `wiki_search` | `- [title](uri): summary` per result, no score | Clean candidate list |
| `wiki_graph` | Natural language: clusters, hubs, relations, isolated | Graph interpretation |
| `wiki_export` (llms-txt) | Same as list but all pages, with wiki header | llms.txt publishing |
| `wiki_export` (llms-full) | llms-txt + full bodies separated by `---` | Long-context analysis |
| `wiki_export` (json) | JSON array with metadata + body | Batch processing |

## `format: "llms"` output structure

### `wiki_list(format: "llms")`

```markdown
## concept (18)

- [Mixture of Experts](wiki://research/concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Scaling Laws](wiki://research/concepts/scaling-laws): Empirical laws relating model size, data, and compute.
- ~~[Old Concept](wiki://research/concepts/old): Superseded.~~

## paper (14)

- [Switch Transformer](wiki://research/sources/switch-transformer-2021): Scales to trillion parameters using MoE.
```

- Groups ordered by page count desc
- Within group: confidence desc, then title asc
- Archived pages shown with `~~strikethrough~~`
- Pagination footer when more than one page

### `wiki_search(format: "llms")`

```markdown
- [Mixture of Experts](wiki://research/concepts/mixture-of-experts): Sparse routing of tokens to expert subnetworks.
- [Switch Transformer](wiki://research/sources/switch-transformer-2021): Scales to trillion parameters using MoE.
```

### `wiki_graph(format: "llms")`

```markdown
The wiki graph has 42 nodes and 87 edges across 5 type groups.

**concept** (18 nodes): Agent, Context Window, Mixture of Experts, ...
**paper** (14 nodes): Karpathy LLM Wiki, Switch Transformer, ...

Key hubs: Mixture of Experts (12 edges), Scaling Laws (9 edges)

**Edges by relation:**
- `fed-by` (32)
- `depends-on` (28)
- `links-to` (9)

**Isolated nodes (3):** draft-stub-xyz, tangent-note-abc, orphan-page
```

## `summary` in the index

The `summary` frontmatter field is stored in the tantivy index (TEXT +
STORED), so `format: "llms"` output is produced entirely from the
index — no disk reads per page. Pages without a `summary` field appear
as `- [title](uri)` without the colon annotation.

Setting a concise `summary` on pages you own improves orientation
quality for all callers.
