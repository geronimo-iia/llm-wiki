# llm-wiki

**Build knowledge that compounds — not answers that evaporate.**

A git-backed wiki engine that turns a folder of Markdown files into a
searchable, structured knowledge base. Accessible from the command line,
from any MCP-compatible agent, or from any IDE via ACP.

The engine has no LLM dependency. It manages files, git history,
full-text search, and knowledge structure. The LLM is always external.

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/geronimo-iia/llm-wiki/main/install.ps1 | iex

# Or via cargo
cargo install llm-wiki-engine
```

→ [All installation options](docs/guides/installation.md)

## Quick Start

```bash
# Create a wiki
llm-wiki spaces create ~/wikis/research --name research

# Start the MCP server
llm-wiki serve
```

Connect your editor ([VS Code, Cursor, Windsurf, Zed, Claude Code](docs/guides/ide-integration.md)),
then use the tools to create pages, ingest sources, search, and build
knowledge.

→ [Getting started guide](docs/guides/getting-started.md)

## Why Not RAG?

Most AI knowledge tools retrieve and generate on every query. Knowledge
doesn't accumulate.

llm-wiki implements a **Dynamic Knowledge Repository** (DKR): process
sources at ingest time, not query time. The LLM integrates each source
into the wiki — updating concept pages, creating source summaries,
flagging contradictions — and commits the result. Knowledge compounds
with every addition.

|                         | Traditional RAG   | llm-wiki (DKR)            |
| ----------------------- | ----------------- | ------------------------- |
| When knowledge is built | At query time     | At ingest time            |
| Cross-references        | Ad hoc or missed  | Pre-built, maintained     |
| Knowledge accumulation  | Resets each query | Compounds over time       |
| Activity log            | None              | Git history               |
| Data ownership          | Provider systems  | Your files, your git repo |

## What It Does

- **Search** — full-text BM25 search across one or all wikis
- **Type system** — JSON Schema validation per page type, field aliasing,
  custom types via `schemas/`
- **Concept graph** — typed nodes, labeled edges (`fed-by`, `depends-on`,
  `cites`), Mermaid and DOT output
- **Git-backed** — every change is a commit, full history, diffable
- **Multi-wiki** — manage multiple wikis from one process
- **MCP + ACP** — stdio, HTTP, and ACP transports for any agent

## What It Is Not

- Not an LLM — makes no AI calls
- Not a RAG system — does not retrieve and generate on demand
- Not a note-taking app — it is an engine, you bring your own interface
- Not a static site generator — but [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) can render the wiki as a website

## Technology

Single Rust binary. No runtime, no database, no Docker.

| Component | Technology                                                              |
| --------- | ----------------------------------------------------------------------- |
| Search    | [tantivy](https://crates.io/crates/tantivy) (BM25, Lucene-class)        |
| Git       | [git2](https://crates.io/crates/git2) (libgit2)                         |
| Graph     | [petgraph](https://crates.io/crates/petgraph)                           |
| MCP       | [rmcp](https://crates.io/crates/rmcp) (stdio + Streamable HTTP)         |
| ACP       | [agent-client-protocol](https://crates.io/crates/agent-client-protocol) |

## Documentation

|                                                   |                                                    |
| ------------------------------------------------- | -------------------------------------------------- |
| [Getting started](docs/guides/getting-started.md) | End-to-end walkthrough                             |
| [Guides](docs/guides/README.md)                   | Installation, IDE, custom types, CI/CD, multi-wiki |
| [Specifications](docs/specifications/README.md)   | Formal design contracts                            |
| [Architecture](docs/overview.md)                  | Core concepts, project map                         |
| [Roadmap](docs/roadmap.md)                        | Development phases                                 |
| [Decisions](docs/decisions/README.md)             | Architectural decisions                            |

## Related Projects

| Repository                                                             | Description                                         |
| ---------------------------------------------------------------------- | --------------------------------------------------- |
| [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)     | Claude Code plugin — workflow skills for the engine |
| [llm-wiki-hugo-cms](https://github.com/geronimo-iia/llm-wiki-hugo-cms) | Hugo site scaffold — render wiki as a website       |

## Acknowledgments

- **[Andrej Karpathy](https://karpathy.ai/)** — for the
  [LLM Wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)
  that defined the Dynamic Knowledge Repository pattern.
- **[vanillaflava](https://github.com/vanillaflava)** — for
  [llm-wiki-claude-skills](https://github.com/vanillaflava/llm-wiki-claude-skills),
  which turned the pattern into a practical skill architecture.

llm-wiki is a continuation of
[agent-foundation](https://github.com/geronimo-iia/agent-foundation).

## Contributing

[Contributing guide](CONTRIBUTING.md) · [Code of conduct](CODE_OF_CONDUCT.md) · [Security policy](SECURITY.md)

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
