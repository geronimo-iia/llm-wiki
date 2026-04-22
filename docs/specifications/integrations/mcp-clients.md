---
title: "MCP Clients"
summary: "Config snippets for Cursor, VS Code, Windsurf, and generic MCP clients."
read_when:
  - Connecting an editor or agent to the wiki via MCP
status: ready
last_updated: "2025-07-21"
---

# MCP Clients

Any MCP-compatible client connects to llm-wiki by pointing at
`llm-wiki serve`. stdio is always active — no flags needed. All
registered wikis are mounted at startup.

For session-oriented streaming in Zed or VS Code, use ACP instead. See
[acp-transport.md](acp-transport.md).

For Claude Code, use the `llm-wiki-skills` plugin — it starts the
server and provides skills.


## Cursor

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


## Windsurf

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


## Generic MCP Client (stdio)

```
command: llm-wiki
args:    ["serve"]
```

## Generic MCP Client (HTTP)

```
command: llm-wiki
args:    ["serve", "--http", ":8080"]
endpoint: http://localhost:8080/mcp
```


## Notes

- All clients share the same tool surface — see
  [tools/overview.md](../tools/overview.md)
- `llm-wiki serve` with no flags starts stdio only
- `wiki://` URIs in prompts reference pages directly
- Use `--wiki <name>` or `wiki://<name>/<slug>` to target specific wikis
