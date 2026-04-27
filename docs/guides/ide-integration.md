# IDE Integration

llm-wiki exposes its tools via MCP (stdio or HTTP) and ACP. Any
MCP-compatible editor connects by pointing at `llm-wiki serve`.

## Quick Setup

### Claude Code

The `llm-wiki-skills` plugin provides both the MCP server and 11
workflow skills (bootstrap, ingest, crystallize, research, etc.):

```bash
claude plugin add /path/to/llm-wiki-skills
```

The plugin starts `llm-wiki serve` automatically. Skills are available
as slash commands — no MCP config needed.

See [llm-wiki-skills](https://github.com/geronimo-iia/llm-wiki-skills)
for the full skill list and setup instructions.

## VS Code

Add to `.vscode/mcp.json`:

```json
{
  "servers": {
    "llm-wiki": {
      "type": "stdio",
      "command": "llm-wiki",
      "args": ["serve"]
    }
  }
}
```

### Cursor

Add to `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "llm-wiki": {
      "command": "llm-wiki",
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
    "llm-wiki": {
      "command": "llm-wiki",
      "args": ["serve"]
    }
  }
}
```

### Zed (ACP)

```json
{
  "agent_servers": {
    "llm-wiki": {
      "type": "custom",
      "command": "llm-wiki",
      "args": ["serve", "--acp"],
      "env": {}
    }
  }
}
```

ACP provides streaming workflow steps visible in the agent panel.

## Verify the Connection

Once configured, ask the agent to call a wiki tool:

```
Search the wiki for "mixture of experts"
```

If the connection works, the agent calls `wiki_search` and returns
results. If not, check that `llm-wiki` is on your PATH:

```bash
which llm-wiki
llm-wiki --version
```

## Workflow Examples

### Create a wiki and start working

```
1. Initialize a wiki:     llm-wiki spaces create ~/wikis/research --name research
2. Start the server:      llm-wiki serve
3. Connect your IDE
4. Ask the agent:         "Create a concept page about transformer architecture"
```

The agent calls `wiki_content_new` to scaffold the page, then
`wiki_content_write` to fill it, then `wiki_ingest` to validate
and index.

### Research workflow

```
Agent: "Search the wiki for scaling laws"
  → wiki_search("scaling laws")
  → returns ranked results with excerpts

Agent: "Read the top result"
  → wiki_content_read("concepts/scaling-laws")
  → returns full page content

Agent: "Create a new concept page about chinchilla scaling"
  → wiki_content_new("concepts/chinchilla-scaling", type: "concept")
  → wiki_content_write("concepts/chinchilla-scaling", content)
  → wiki_ingest("concepts/chinchilla-scaling")
```

### Ingest a source

```
Agent: "I found a paper about MoE routing. Ingest it."
  → wiki_content_new("sources/moe-routing-2024", type: "paper")
  → wiki_content_write("sources/moe-routing-2024", frontmatter + summary)
  → wiki_ingest("sources/moe-routing-2024")
  → wiki_content_read("concepts/mixture-of-experts")
  → wiki_content_write("concepts/mixture-of-experts", updated with new source)
  → wiki_ingest("concepts/mixture-of-experts")
```

The concept page grows with each source. Knowledge compounds.

### Explore the graph

```
Agent: "Show me the concept graph around MoE"
  → wiki_graph(root: "concepts/moe", depth: 2)
  → returns Mermaid diagram with labeled edges

Agent: "Show only the fed-by relationships"
  → wiki_graph(root: "concepts/moe", relation: "fed-by")
```

### Cross-wiki search

```
Agent: "Search all wikis for transformer architecture"
  → wiki_search("transformer architecture", cross_wiki: true)
  → returns results from all registered wikis, ranked by score
```

## Multiple Wikis

All registered wikis are mounted at startup. Target a specific wiki
with the `wiki` parameter:

```
Agent: "Search the work wiki for project deadlines"
  → wiki_search("project deadlines", wiki: "work")
```

Or use `wiki://` URIs:

```
Agent: "Read wiki://research/concepts/moe"
  → wiki_content_read("wiki://research/concepts/moe")
```

## HTTP Transport (Remote)

For remote or multi-client access, start with HTTP:

```bash
llm-wiki serve --http :8080
```

Point the IDE at `http://localhost:8080/mcp` instead of stdio.

## Available Tools

Tools are available via MCP. See
[docs/specifications/tools/overview.md](../specifications/tools/overview.md)
for the full list.

## Custom Config Path

By default `llm-wiki serve` reads `~/.llm-wiki/config.toml`. Override
it in your MCP config using `--config` or `LLM_WIKI_CONFIG`:

```json
{
  "llm-wiki": {
    "command": "llm-wiki",
    "args": ["--config", "/path/to/config.toml", "serve"]
  }
}
```

Or via environment variable (useful when the same config applies to
multiple tools in the same session):

```json
{
  "llm-wiki": {
    "command": "llm-wiki",
    "args": ["serve"],
    "env": {
      "LLM_WIKI_CONFIG": "/path/to/config.toml"
    }
  }
}
```

This lets you run multiple isolated wiki environments on the same
machine — each with its own registered wikis.

## Live Indexing

Add `--watch` to any `serve` configuration for live indexing —
external edits are picked up automatically within ~500ms:

```json
"args": ["serve", "--watch"]
```

Or standalone: `llm-wiki watch`.
