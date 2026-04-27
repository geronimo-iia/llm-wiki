# llm-wiki

A headless wiki engine for agents. 19 MCP tools. One Rust binary. No LLM inside.

**Build knowledge that compounds — not answers that evaporate.**

A git-backed Markdown wiki — searchable, typed, graph-linked. Accessible from
the command line, from any MCP-compatible agent, or from any IDE via ACP.

---

## The problem with RAG

Most AI knowledge tools retrieve and generate on every query. Each answer is
disposable — nothing is learned, nothing is kept. Ask the same question twice
and the LLM reasons from scratch.

llm-wiki implements a different pattern — the **Dynamic Knowledge Repository**
(DKR), introduced by Andrej Karpathy:

> Process sources at ingest time, not query time. The LLM integrates each
> source into the wiki — updating concept pages, creating source summaries,
> flagging contradictions — and commits the result. Knowledge compounds with
> every addition.

|                         | Traditional RAG       | llm-wiki (DKR)              |
| ----------------------- | --------------------- | --------------------------- |
| When knowledge is built | At query time         | At ingest time              |
| Cross-references        | Ad hoc or missed      | Pre-built, typed graph      |
| Knowledge accumulation  | Resets each query     | Compounds over time         |
| Audit trail             | None                  | Git history per page        |
| Data ownership          | Provider systems      | Your files, your git repo   |

---

## How it works

The engine is pure infrastructure. It manages files, git, full-text search,
and graph structure. The LLM is always external — it calls the engine's tools
via MCP, reads pages, writes pages, and commits knowledge. Intelligence flows
through skills, not the binary.

```
LLM agent
  │
  ├── wiki_search("mixture of experts")     → ranked results + facets
  ├── wiki_content_read("concepts/moe")     → full page + frontmatter
  ├── wiki_graph(root: "concepts/moe")      → typed graph in Mermaid/DOT
  ├── wiki_suggest("concepts/moe")          → pages worth linking
  ├── wiki_content_write(...)               → write synthesized knowledge
  └── wiki_ingest(path: "concepts/")        → validate, index, commit
```

A wiki page is a plain Markdown file with typed frontmatter:

```yaml
---
type: concept
title: Mixture of Experts
status: active
confidence: 0.9
tags: [routing, scaling, efficiency]
sources:
  - sources/switch-transformer-2021
  - sources/mixtral-2024
concepts:
  - concepts/sparse-routing
  - concepts/scaling-laws
---

Sparse routing of tokens to expert subnetworks...
```

The engine validates frontmatter against a JSON Schema, extracts typed graph
edges from `sources` and `concepts`, and indexes everything in tantivy. The
graph is live the moment a page is committed.

---

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.ps1 | iex

# Homebrew
brew install geronimo-iia/tap/llm-wiki

# Cargo
cargo install llm-wiki-engine
```

→ [All installation options](docs/guides/installation.md)

---

## Quick start

```bash
# Create a wiki space
llm-wiki spaces create ~/wikis/research --name research

# Start the MCP server
llm-wiki serve
```

Connect your agent or editor — VS Code, Cursor, Windsurf, Zed, Claude Code —
via the MCP config. The 19 tools are immediately available.

→ [Getting started guide](docs/guides/getting-started.md) · [IDE integration](docs/guides/ide-integration.md)

---

## What agents can do

| Tool | What it does |
| ---- | ------------ |
| `wiki_search` | BM25 full-text search across one or all wikis, with type/status/tag facets |
| `wiki_list` | Paginated page listing with filters; `format: "llms"` for LLM-readable output |
| `wiki_content_read` | Read a page with optional backlinks |
| `wiki_content_write` | Write a page (validates frontmatter against type schema) |
| `wiki_content_new` | Scaffold a new page from its type template |
| `wiki_ingest` | Validate a path, update the index, commit to git |
| `wiki_graph` | Typed concept graph — Mermaid, DOT, or natural-language `llms` format |
| `wiki_suggest` | Find pages worth linking by tag overlap, graph distance, BM25 similarity |
| `wiki_stats` | Wiki health: page counts, type distribution, staleness, graph density |
| `wiki_lint` | Deterministic quality rules: orphans, broken links, missing fields, stale pages |
| `wiki_export` | Write full wiki to `llms.txt` at wiki root — for ecosystem publishing or audit |
| `wiki_history` | Git commit history for a page, with rename following |
| `wiki_schema` | Show, validate, or template a type schema |
| `wiki_spaces_*` | Create, list, mount, unmount wiki spaces |

Full tool reference: [`docs/specifications/tools/`](docs/specifications/tools/)

---

## Skills

The engine exposes tools. Skills tell agents how to use them.

[llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills) is a
Claude Code plugin that ships ready-to-use workflows:

| Skill | What it does |
| ----- | ------------ |
| `crystallize` | Distil a session into durable wiki pages — decisions, findings, open questions |
| `ingest` | Process source files from `inbox/` into synthesized, cross-referenced pages |
| `research` | Search the wiki and synthesize an answer from existing knowledge |
| `lint` | Structural audit — orphans, broken links, schema issues, under-linked pages |
| `graph` | Explore and interpret the concept graph |

Skills are plain Markdown files — readable by the LLM, replaceable, forkable.
Write your own for your own workflows.

---

## Architecture

```
llm-wiki-engine          pure Rust binary — tools, git, index, graph
llm-wiki-skills          Claude Code plugin — workflow skills (Markdown)
llm-wiki-hugo-cms        Hugo scaffold — render the wiki as a website
```

The engine has no opinions about workflows, LLM providers, or interfaces.
Every LLM call happens outside the binary. Every workflow lives in a skill.
The separation means skills ship independently, the engine stays stable, and
nothing is coupled to a specific AI provider.

---

## Technology

Single Rust binary. No runtime, no database, no Docker.

| Component | Technology |
| --------- | ---------- |
| Search | [tantivy](https://crates.io/crates/tantivy) — BM25, Lucene-class performance |
| Git | [git2](https://crates.io/crates/git2) — libgit2 bindings |
| Graph | [petgraph](https://crates.io/crates/petgraph) — typed DiGraph |
| MCP | [rmcp](https://crates.io/crates/rmcp) — stdio + Streamable HTTP |
| ACP | [agent-client-protocol](https://crates.io/crates/agent-client-protocol) |

---

## Documentation

| | |
| - | - |
| [Getting started](docs/guides/getting-started.md) | End-to-end walkthrough |
| [Guides](docs/guides/README.md) | Installation, IDE, custom types, CI/CD, multi-wiki |
| [Specifications](docs/specifications/README.md) | Formal tool and model contracts |
| [Architecture](docs/overview.md) | Core concepts, project map |
| [Roadmap](docs/roadmap.md) | What shipped, what's next |
| [Decisions](docs/decisions/README.md) | Architectural decision records |

---

## Acknowledgments

- **[Andrej Karpathy](https://karpathy.ai/)** — for the
  [LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
  that defined the Dynamic Knowledge Repository pattern.
- **[vanillaflava](https://github.com/vanillaflava)** — for
  [llm-wiki-claude-skills](https://github.com/vanillaflava/llm-wiki-claude-skills),
  which turned the pattern into a practical skill architecture.

llm-wiki is a continuation of
[agent-foundation](https://github.com/geronimo-iia/agent-foundation).

---

## Contributing

[Contributing guide](CONTRIBUTING.md) · [Code of conduct](CODE_OF_CONDUCT.md) · [Security policy](SECURITY.md)

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
