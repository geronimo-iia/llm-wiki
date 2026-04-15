# llm-wiki

llm-wiki is a git-backed knowledge base engine. It turns a folder of Markdown
files into a searchable, structured, contradiction-aware wiki — accessible from
the command line, from Claude Code via MCP, and from any IDE via ACP.

The engine has no LLM dependency. It manages files, git history, full-text
search, and knowledge structure. The LLM is always external.

---

## The Problem It Solves

Most AI knowledge tools use RAG: upload documents, ask a question, the system
retrieves relevant text and generates an answer. Each query starts from scratch.
Knowledge does not accumulate. Contradictions between sources are missed.

llm-wiki takes the opposite approach, grounded in Karpathy's LLM Wiki concept:
process sources at ingest time, not query time. Build a persistent, structured
knowledge base that grows smarter with every addition.

| | Traditional RAG | llm-wiki |
|--|-----------------|---------|
| When knowledge is built | At query time, per question | At ingest time, once per source |
| Cross-references | Discovered ad hoc or missed | Pre-built, continuously maintained |
| Contradictions | Often overlooked | First-class knowledge nodes |
| Knowledge accumulation | None — resets each query | Builds over time |
| Data ownership | Provider systems | Your files, your git repo |

---

## What It Is

A Rust CLI and MCP server that manages a wiki repository:

- **Ingest** — add files, folders, or LLM-produced enrichments to the wiki
- **Search** — full-text BM25 search across all pages
- **Context** — find the most relevant pages for a question, return references
- **Read** — fetch the full content of a single page
- **Lint** — audit the wiki for orphans, missing stubs, active contradictions
- **Graph** — visualize the concept graph as DOT or Mermaid
- **Serve** — expose the wiki as an MCP server (stdio or SSE) or ACP agent

The wiki is plain Markdown files in a git repository. No database. No
proprietary format. Any tool that reads Markdown can read the wiki.

---

## What It Is Not

- Not an LLM. It makes no AI calls.
- Not a RAG system. It does not retrieve and generate on demand.
- Not a note-taking app. It is an engine — you bring your own interface.
- Not a static site generator. It is a knowledge base, not a website.

---

## Core Concepts

**Wiki root** — the git repository directory. All pages, assets, and indices
are relative to it. One wiki = one git repo.

**Page** — a Markdown file with YAML frontmatter. Either a flat `.md` file
(no assets) or a bundle folder with `index.md` and co-located assets.

**Slug** — the stable address of a page. Derived from its path relative to
the wiki root, without extension. `concepts/mixture-of-experts` resolves to
either `concepts/mixture-of-experts.md` or `concepts/mixture-of-experts/index.md`.

**Category** — the first path segment of a slug. Five fixed categories
(`concepts/`, `sources/`, `contradictions/`, `queries/`, `raw/`) plus
user-defined prefixes for structured content like skills or guides.

**Enrichment** — LLM-produced metadata (claims, confidence, tags, contradictions)
merged into existing page frontmatter. The LLM annotates pages; it does not
author them.

---

## The Five Documents

| Document | What it covers |
|----------|---------------|
| [epistemic-model.md](epistemic-model.md) | Why the five categories exist and what each one means |
| [ingest-model.md](ingest-model.md) | How content enters the wiki — three modes, assets, enrichment |
| [retrieval-model.md](retrieval-model.md) | How content is found and read — search, context, wiki read |
| [llm-integration.md](llm-integration.md) | How an LLM uses the wiki — workflows, MCP, ACP, enrichment contract |
| [features.md](features.md) | Complete feature reference |

For implementation details, see [design/](design/).
