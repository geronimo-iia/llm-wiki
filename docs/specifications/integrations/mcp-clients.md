---
title: "MCP Clients"
summary: "How to connect any MCP-compatible editor or agent to llm-wiki — configuration snippets for Cursor, VS Code, Windsurf, and generic MCP clients."
read_when:
  - Connecting a new editor or agent to the wiki via MCP
  - Looking up the config snippet for a specific tool
  - Understanding which transport to use for a given client
status: active
last_updated: "2025-07-15"
---

# MCP Clients

Any MCP-compatible client connects to llm-wiki by pointing at `wiki serve`.
stdio is always active — no flags needed. The wiki mounts all registered
wikis at startup and exposes all tools and resources.

For session-oriented streaming workflows in Zed or VS Code, use the ACP
transport instead. See [acp-transport.md](acp-transport.md).

---

## Cursor

Add to `.cursor/mcp.json` at the project or user level:

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

All registered wikis are available. Use `wiki://research/<slug>` URIs in
prompts to reference pages directly.

---

## VS Code (Copilot / MCP extension)

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

---

## Windsurf

Add to the Windsurf MCP config (global or workspace):

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

---

## Generic MCP client

Any client that supports stdio MCP servers:

```
command: wiki
args:    ["serve"]
```

For SSE-based clients (remote or multi-client):

```
command: wiki
args:    ["serve", "--sse", ":8080"]
endpoint: http://localhost:8080/sse
```

---

## Notes

- All clients share the same tool surface — see [features.md](../features.md)
  for the full MCP tools table
- `wiki serve` with no flags starts stdio only — safe to run as a background
  process for any client
- For Claude Code, use the plugin instead — see
  [claude-plugin.md](claude-plugin.md)
- For Zed with streaming workflows, use ACP — see
  [acp-transport.md](acp-transport.md)
