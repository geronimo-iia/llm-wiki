# Specifications

Full specification of the llm-wiki project. These documents are the source of
truth for design decisions, contracts, and behavior.

---

## Start Here

| Document | What it covers |
|----------|---------------|
| [overview.md](overview.md) | What llm-wiki is, the core model, key concepts |
| [features.md](features.md) | Complete feature list by capability area |

---

## core/

Foundational data model and repository structure.

| Document | What it covers |
|----------|---------------|
| [repository-layout.md](core/repository-layout.md) | How pages and assets are organized on disk |
| [page-content.md](core/page-content.md) | Frontmatter schema, required fields, per-type conventions |
| [frontmatter-authoring.md](core/frontmatter-authoring.md) | LLM-facing reference for writing frontmatter |
| [epistemic-model.md](core/epistemic-model.md) | Why the type taxonomy carries epistemic distinctions |
| [source-classification.md](core/source-classification.md) | How source types work within the type taxonomy |

---

## commands/

One file per CLI command.

| Document | Command |
|----------|---------|
| [cli.md](commands/cli.md) | All commands, subcommands, and flags |
| [init.md](commands/init.md) | `wiki init` |
| [spaces.md](commands/spaces.md) | `wiki spaces` |
| [configuration.md](commands/configuration.md) | `wiki config` |
| [page-creation.md](commands/page-creation.md) | `wiki new page` / `wiki new section` |
| [read.md](commands/read.md) | `wiki read` |
| [list.md](commands/list.md) | `wiki list` |
| [search.md](commands/search.md) | `wiki search` |
| [graph.md](commands/graph.md) | `wiki graph` |
| [lint.md](commands/lint.md) | `wiki lint` |
| [index.md](commands/index.md) | `wiki index` |
| [serve.md](commands/serve.md) | `wiki serve` |
| [instruct.md](commands/instruct.md) | `wiki instruct` |

---

## pipelines/

Data processing and ingestion flows.

| Document | What it covers |
|----------|---------------|
| [ingest.md](pipelines/ingest.md) | Validate, commit, and index files in the wiki tree |
| [asset-ingest.md](pipelines/asset-ingest.md) | Co-located assets and bundle promotion |
| [crystallize.md](pipelines/crystallize.md) | Distilling chat sessions into wiki pages |

---

## llm/

LLM-facing behavior and workflows.

| Document | What it covers |
|----------|---------------|
| [session-bootstrap.md](llm/session-bootstrap.md) | How the LLM orients itself at session start |
| [backlink-quality.md](llm/backlink-quality.md) | Linking policy and missing connection detection |

See also [commands/instruct.md](commands/instruct.md) for the `wiki instruct` command that delivers these workflows.

---

## integrations/

External tool integrations.

| Document | What it covers |
|----------|---------------|
| [acp-transport.md](integrations/acp-transport.md) | ACP transport for Zed / VS Code |
| [claude-plugin.md](integrations/claude-plugin.md) | Claude Code plugin structure |
| [mcp-clients.md](integrations/mcp-clients.md) | Cursor, VS Code, Windsurf and generic MCP client config |

---

## Notes

The `design/` folder contains historical design documents. The specifications
folder is the source of truth for current contracts and behavior.

[rust-modules.md](rust-modules.md) is the canonical reference for the `src/`
module layout — consult it when deciding which module a new function belongs in.
