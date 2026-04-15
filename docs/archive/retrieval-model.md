# Retrieval Model

Finding content in the wiki is a two-step process: discover relevant pages
via `wiki context`, then fetch the ones you need via `wiki read`. The wiki
never floods a caller with content it did not ask for.

---

## The Core Principle

**References first, content on demand.**

`wiki context` returns a ranked list of page references — slug, URI, file
path, title, and relevance score. It never returns page bodies. The caller
decides which pages to read and fetches only those.

This keeps the caller's context window lean and gives full control over what
gets loaded.

---

## Search

```
wiki search "mixture of experts"
wiki search "mixture of experts" --top 10
wiki search --all "transformer scaling"   # across all registered wikis
```

Full-text BM25 search across all pages. Returns a ranked list of results
with slug, title, and score. The search index is built from page content and
frontmatter fields (title, tags, tldr, body).

The index lives in `.wiki/search-index/` — gitignored, rebuilt on demand.
A fresh clone runs `wiki search --rebuild-index` to become fully functional.

---

## Context

```
wiki context "how does MoE scaling work?"
wiki context "MoE scaling" --top-k 5
```

Runs a BM25 search against the question and returns a ranked reference list.
Contradiction pages are included when relevant — they are high-value context
that captures the structure of a knowledge domain.

Each result:

```
slug:  concepts/mixture-of-experts
uri:   wiki://research/concepts/mixture-of-experts
path:  /Users/.../concepts/mixture-of-experts.md
title: Mixture of Experts
score: 0.94
```

The URI (`wiki://`) is the stable address for MCP resource access. The path
is the absolute file path on disk for direct file reads.

---

## Read

```
wiki read concepts/mixture-of-experts
wiki read concepts/mixture-of-experts --body-only
```

Fetches the full content of a single page by slug. Resolves both flat files
(`concepts/foo.md`) and bundles (`concepts/foo/index.md`) transparently.

`--body-only` strips the frontmatter, returning only the Markdown body.

---

## Slug Resolution

A slug resolves to a file using two rules, checked in order:

1. `{slug}.md` — flat file
2. `{slug}/index.md` — bundle

The caller always uses the same slug regardless of which form is on disk.

---

## MCP Resources

Every page is accessible as an MCP resource:

```
wiki://research/concepts/mixture-of-experts
wiki://research/sources/switch-transformer-2021
wiki://research/contradictions/moe-scaling-efficiency
wiki://research/skills/semantic-commit/index
```

Bundle assets are also accessible:

```
wiki://research/concepts/mixture-of-experts/moe-routing.png
wiki://research/skills/semantic-commit/lifecycle.yaml
```

When a page is updated by ingest, the server emits a resource update
notification — clients automatically see fresh content.

---

## Multi-Wiki Search

With multiple wikis registered, `--all` fans out the search across all of
them and merges results ranked by relevance:

```
wiki search --all "transformer scaling"
wiki context --all "how does MoE work?"
```

Results include the wiki name so the caller knows which wiki each page
belongs to.

---

## Lint

```
wiki lint
```

Structural audit of the wiki. Reports:

- **Orphan pages** — pages with no inbound links
- **Missing stubs** — pages referenced but not yet created
- **Active contradictions** — contradiction pages awaiting enrichment
- **Orphan asset references** — pages referencing assets that no longer exist

Writes `LINT.md` and commits it. The external LLM reads `LINT.md` and
re-ingests enriched content as needed.

---

## Graph

```
wiki graph
wiki graph --format mermaid
```

Emits the concept graph as DOT or Mermaid. Nodes are pages; edges are
wikilinks and `related_concepts` frontmatter fields. Orphan pages appear
as isolated nodes. Hub pages appear larger.
