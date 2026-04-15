---
title: "Overview"
summary: "What llm-wiki is, the problem it solves, and the core model."
read_when:
  - Understanding what llm-wiki is and why it exists
  - Explaining the project to a new contributor or LLM
  - Understanding the relationship between the wiki engine and the LLM
status: active
last_updated: "2025-07-15"
---

# llm-wiki

llm-wiki is a git-backed wiki engine. It turns a folder of Markdown files into
a searchable, structured knowledge base — accessible from the command line, from
any MCP-compatible agent via MCP, and from any IDE via ACP.

The engine has no LLM dependency. It manages files, git history, full-text
search, and knowledge structure. The LLM is always external.

---

## The Problem It Solves

Most AI knowledge tools use RAG: upload documents, ask a question, the system
retrieves relevant text and generates an answer. Each query starts from scratch.
Knowledge does not accumulate.

llm-wiki implements a Dynamic Knowledge Repository (DKR): process sources at
ingest time, not query time. The LLM reads each source, integrates it into the
existing wiki — updating concept pages, creating source summaries, flagging
contradictions — and commits the result. Knowledge compounds with every
addition. The wiki grows smarter over time without re-deriving anything.

| | Traditional RAG | llm-wiki (DKR) |
|--|-----------------|----------------|
| When knowledge is built | At query time, per question | At ingest time, once per source |
| Cross-references | Discovered ad hoc or missed | Pre-built, continuously maintained |
| Contradiction detection | Never | Flagged at ingest time |
| Knowledge accumulation | None — resets each query | Compounds over time |
| Activity log | None | Git history (semantic commits) |
| Data ownership | Provider systems | Your files, your git repo |

---

## The Four Layers

A wiki repository is a Dynamic Knowledge Repository (DKR). The only
structure the engine enforces is the flow from inbox to archive to knowledge:

```
my-wiki/
├── schema.md   ← wiki schema: categories, ingest rules, lint conventions
├── inbox/      ← Layer 1: drop zone          (human puts files here)
├── raw/        ← Layer 2: immutable archive  (moved here after ingest)
├── wiki/       ← Layer 3: compiled knowledge (LLM reads and writes here)
└── .wiki/      ← Layer 4: engine metadata    (engine manages this)
```

The human drops files in `inbox/`. The LLM ingests them and moves them to
`raw/`. The LLM maintains `wiki/`. The engine owns `.wiki/`. Git history is
the activity log. The search engine is the index.

`schema.md` is the only configuration the LLM needs. It defines how *this
wiki instance* is organized — categories, ingest depth, lint rules, domain
conventions. The engine ships a default `schema.md` template; the owner
customizes it. The MCP server injects it at session start so the LLM always
operates with the correct conventions for this wiki.

The engine enforces nothing about categories. `inbox/` → `raw/` → `wiki/` is
the only flow that matters — everything else is the wiki owner's choice,
expressed in `schema.md`.

---

## The Model

```
Human drops file in inbox/       LLM ingests it
─────────────────────────        ──────────────────────────────────────────────
inbox/my-article.md         →   reads schema.md (knows this wiki's conventions)
                                 reads inbox file
                                 writes wiki pages per schema conventions
                                 flags contradictions
                                 moves inbox/my-article.md → raw/my-article.md
                                 git commit: ingest(<slug>)
```

The LLM writes directly to `wiki/` via MCP tools. The engine handles file
operations, slug resolution, frontmatter merging, and git commits. The two
are independent — the engine works without an LLM, the LLM works through
the engine's MCP interface.

---

## What It Is

A Rust CLI and MCP/ACP server that manages a wiki repository:

- **Ingest** — add files, folders, or LLM-produced enrichments to the wiki
- **Search** — full-text BM25 search across all pages
- **Read** — fetch the full content of a single page by slug or `wiki://` URI
- **List** — paginated enumeration of pages with type and status filters
- **Lint** — audit the wiki for orphans, missing stubs, empty sections
- **Graph** — visualize the concept graph as Mermaid or DOT
- **Serve** — expose the wiki as an MCP server (stdio + SSE) or ACP agent
- **Index** — manage the tantivy search index explicitly

The wiki is plain Markdown files in a git repository. No database. No
proprietary format. Any tool that reads Markdown can read the wiki.

---

## What It Is Not

- Not an LLM — makes no AI calls
- Not a RAG system — does not retrieve and generate on demand
- Not a note-taking app — it is an engine, you bring your own interface
- Not a static site generator — it is a knowledge base, not a website

---

## Core Concepts

**Wiki root** — the git repository directory. All pages, assets, and indices
are relative to it. One wiki = one git repo.

**Page** — a Markdown file with YAML frontmatter. Either a flat `.md` file
(no assets) or a bundle folder with `index.md` and co-located assets.

**Slug** — the stable address of a page. Derived from its path relative to
the wiki root, without extension. `concepts/mixture-of-experts` resolves to
either `concepts/mixture-of-experts.md` or `concepts/mixture-of-experts/index.md`.

**Section** — a directory that groups related pages, always with an `index.md`.

**`wiki://` URI** — the portable reference format for pages.
`wiki://research/concepts/mixture-of-experts` or `wiki://concepts/mixture-of-experts`
for the default wiki.

**Enrichment** — LLM-produced metadata (claims, confidence, tags, sources)
merged into existing page frontmatter. The LLM annotates pages; it does not
author them.

---

## Epistemic Model

The DKR model requires only one flow:

```
inbox/  → waiting to be processed (human drop zone)
raw/    → what we received        (immutable archive, never indexed)
wiki/   → what we derived         (compiled knowledge, LLM-maintained)
```

Within `wiki/`, the page `type` frontmatter field carries the epistemic role
(`concept`, `source-summary`, `query-result`). Directory structure is the
wiki owner's choice, defined in `schema.md`. See
[epistemic-model.md](epistemic-model.md) for the full rationale.

---

## Multi-Wiki

A single `wiki` process manages multiple git repositories registered in
`~/.wiki/config.toml`. All CLI commands and MCP tools accept `--wiki <name>`.
Pages are addressed as `wiki://<name>/<slug>` or `wiki://<slug>` for the
default wiki.
