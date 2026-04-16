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
├── README.md   ← for humans (name, description, usage)
├── wiki.toml   ← per-wiki config (name, description, overrides)
├── schema.md   ← wiki schema: categories, ingest rules, lint conventions
├── inbox/      ← Layer 1: drop zone          (human puts files here)
├── raw/        ← Layer 2: immutable archive  (originals preserved here)
└── wiki/       ← Layer 3: compiled knowledge (authors write directly here)
```

The human drops files in `inbox/` for the LLM to process. The LLM reads
them, writes pages directly into `wiki/`, and runs `llm-wiki ingest` to validate,
commit, and index. Originals can be archived to `raw/`. Git history is the
activity log. Search indexes live in `~/.llm-wiki/indexes/<name>/`, not in the repo.

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
Human drops file in inbox/       LLM processes it
─────────────────────────        ──────────────────────────────────────────────
inbox/my-article.md         →   reads schema.md (knows this wiki's conventions)
                                 reads inbox file
                                 writes pages directly into wiki/ tree
                                 llm-wiki ingest → validate, index, commit (if auto_commit)
```

Authors (human or LLM) write directly into the wiki tree. The engine
validates, indexes, and optionally commits to git. The two are independent — the engine
works without an LLM, the LLM works through the engine's MCP interface.

---

## What It Is

A Rust CLI and MCP/ACP server that manages a wiki repository:

- **Ingest** — validate and index files already in the wiki tree; commit when `auto_commit` is on
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

**Write + Ingest** — the two-step pattern. The author writes a file into the
wiki tree, then `llm-wiki ingest` validates and indexes it. No file
movement — the file is already where it belongs.

---

## Epistemic Model

The DKR model has two physical layers and one epistemic axis:

```
inbox/  → waiting to be processed (human drop zone)
raw/    → what we received        (immutable archive, never indexed)
wiki/   → what we derived         (compiled knowledge, authors write here)
```

Within `wiki/`, the page `type` field is the epistemic axis:

- `concept` — what we know (synthesized knowledge)
- `paper`, `article`, `documentation`, etc. — what each source claims (provenance)
- `query-result` — what we concluded (reasoning output)

Folder structure is organizational, defined by `schema.md`. The engine
enforces nothing about folders inside `wiki/`. See
[epistemic-model.md](core/epistemic-model.md) for the full rationale.

---

## Multi-Wiki

A single `wiki` process manages multiple git repositories registered in
`~/.llm-wiki/config.toml`. All CLI commands and MCP tools accept `--wiki <name>`.
Pages are addressed as `wiki://<name>/<slug>` or `wiki://<slug>` for the
default wiki. See [spaces.md](commands/spaces.md).
