# Specifications

Full specification of the llm-wiki project. These documents are the source of
truth for design decisions, contracts, and behavior. The `design/` folder and
`dev/` folder will be rewritten to reference these specs.

---

## Start Here

| Document | What it covers |
|----------|---------------|
| [overview.md](overview.md) | What llm-wiki is, the core model, key concepts |
| [features.md](features.md) | Complete feature list by capability area |
| [cli.md](cli.md) | All commands, subcommands, and flags |
| [epistemic-model.md](epistemic-model.md) | Why the four default categories exist |

---

## Commands

| Document | Command |
|----------|---------|
| [init.md](init.md) | `wiki init` |
| [page-creation.md](page-creation.md) | `wiki new page` / `wiki new section` |
| [ingest.md](ingest.md) | `wiki ingest` |
| [read.md](read.md) | `wiki read` |
| [search.md](search.md) | `wiki search` |
| [list.md](list.md) | `wiki list` |
| [lint.md](lint.md) | `wiki lint` |
| [graph.md](graph.md) | `wiki graph` |
| [index.md](index.md) | `wiki index` |
| [serve.md](serve.md) | `wiki serve` |
| [instruct.md](instruct.md) | `wiki instruct` |
| [registry.md](registry.md) | `wiki registry` |
| [configuration.md](configuration.md) | `wiki config` + all config keys |

---

## Data Model and Layout

| Document | What it covers |
|----------|---------------|
| [repository-layout.md](repository-layout.md) | How pages and assets are organized on disk |
| [page-content.md](page-content.md) | Frontmatter schema, merge rules, body assembly |
| [asset-ingest.md](asset-ingest.md) | Co-located assets and bundle promotion |
| [source-classification.md](source-classification.md) | Configurable taxonomy for source types |

---

## Knowledge Quality

| Document | What it covers |
|----------|---------------|
| [session-bootstrap.md](session-bootstrap.md) | How the LLM orients at session start |
| [crystallize.md](crystallize.md) | Workflow for distilling chat sessions into wiki pages |
| [backlink-quality.md](backlink-quality.md) | Linking policy and missing connection detection |
| [frontmatter-authoring.md](frontmatter-authoring.md) | LLM-facing reference for writing frontmatter |

---

## Integrations

| Document | What it covers |
|----------|---------------|
| [acp-transport.md](acp-transport.md) | ACP transport for Zed / VS Code |
| [claude-plugin.md](claude-plugin.md) | Claude Code plugin structure |

---

## Notes

The `design/` folder contains historical design documents. The specifications
folder is the source of truth for current contracts and behavior.
