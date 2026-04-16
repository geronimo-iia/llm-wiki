# llm-wiki

A git-backed wiki engine that turns a folder of Markdown files into a
searchable, structured knowledge base. Accessible from the command line,
from any MCP-compatible agent, or from any IDE via ACP. The engine has no
LLM dependency — it manages files, git history, full-text search, and
knowledge structure. The LLM is always external.

## Quick Start

```bash
# Install (cargo)
cargo install llm-wiki

# Initialize a wiki
wiki init ~/wikis/research --name research

# Start the MCP server
wiki serve
```

Connect an MCP client (see below), then use the wiki tools to create pages,
ingest sources, search, and build knowledge.

## Core Concepts

- **Wiki** — a git repository with `inbox/`, `raw/`, and `wiki/` directories.
  One wiki = one git repo.
- **Page** — a Markdown file with YAML frontmatter. Either a flat `.md` file
  or a bundle folder with `index.md` and co-located assets.
- **Slug** — the stable address of a page, derived from its path relative to
  `wiki/` without extension. `concepts/mixture-of-experts` resolves to either
  `concepts/mixture-of-experts.md` or `concepts/mixture-of-experts/index.md`.
- **`wiki://` URI** — portable reference format. `wiki://research/concepts/moe`
  addresses a page in the `research` wiki. `wiki://concepts/moe` uses the
  default wiki.
- **Ingest** — validate, commit, and index files already in the wiki tree.
  Authors write directly into `wiki/`, then run `wiki ingest`.
- **Search** — full-text BM25 search via tantivy, returning ranked results
  with `wiki://` URIs.

## MCP Client Setup

### Claude Code

Use the plugin:

```bash
claude plugin add /path/to/llm-wiki
```

### Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "wiki": {
      "command": "wiki",
      "args": ["serve"]
    }
  }
}
```

### VS Code

Add to `.vscode/mcp.json`:

```json
{
  "servers": {
    "wiki": {
      "type": "stdio",
      "command": "wiki",
      "args": ["serve"]
    }
  }
}
```

### Windsurf

Add to the Windsurf MCP config:

```json
{
  "mcpServers": {
    "wiki": {
      "command": "wiki",
      "args": ["serve"]
    }
  }
}
```

## CLI Reference

See [docs/specifications/commands/cli.md](docs/specifications/commands/cli.md)
for the full command reference. Summary:

```
wiki init <path> --name <name>       Initialize a new wiki
wiki new page <uri> [--bundle]       Create a page with scaffolded frontmatter
wiki new section <uri>               Create a section
wiki ingest <path> [--dry-run]       Validate, commit, and index
wiki search "<query>"                Full-text BM25 search
wiki read <slug|uri>                 Fetch page content
wiki list [--type] [--status]        Paginated page listing
wiki lint                            Structural audit
wiki lint fix                        Auto-fix missing stubs and empty sections
wiki graph [--format mermaid|dot]    Concept graph
wiki index rebuild                   Rebuild search index
wiki index status                    Check index health
wiki config get|set|list             Read/write configuration
wiki spaces list|remove|set-default  Manage wiki spaces
wiki serve [--sse] [--acp]           Start MCP/ACP server
wiki instruct [<workflow>]           Print workflow instructions
```

## How It Works

llm-wiki implements a Dynamic Knowledge Repository (DKR). Instead of
retrieving and generating answers at query time (RAG), knowledge is built
at ingest time. The LLM reads each source, integrates it into the wiki —
updating concept pages, creating source summaries, flagging contradictions —
and commits the result. Knowledge compounds with every addition. The wiki
is plain Markdown in a git repository; any tool that reads Markdown can
read the wiki.

## License

MIT OR Apache-2.0
