---
title: "Serve"
summary: "Start the wiki MCP/ACP server — serves all registered wikis, supports stdio, SSE, and ACP transports simultaneously."
read_when:
  - Implementing or extending the serve command
  - Understanding transport modes and multi-wiki exposure
  - Configuring wiki serve defaults
status: draft
last_updated: "2025-07-15"
---

# Serve

`wiki serve` starts the wiki server. It mounts all registered wikis at startup
and exposes them via MCP tools and resources. stdio is always active. SSE and
ACP are opt-in and can run simultaneously.

---

## 1. Transports

| Transport | Protocol | Use case |
|-----------|----------|----------|
| stdio | MCP | Claude Code, local agents, batch pipelines — always on |
| SSE | MCP | Remote agents, multi-client access |
| ACP | ACP | Zed / VS Code agent panel — streaming, session-oriented |

All active transports share the same wiki engine and spaces. A request on
any transport sees the same pages and state.

---

## 2. Multi-Wiki

All wikis registered in `~/.wiki/config.toml` are mounted at startup. No
`--wiki` flag — the server is a spaces-wide service.

MCP resources are namespaced by wiki name:

```
wiki://research/concepts/mixture-of-experts
wiki://work/concepts/transformer-scaling
```

MCP tools accept an optional `wiki` parameter to target a specific wiki.
When omitted, the default wiki (`global.default_wiki`) is used.

---

## 3. CLI Interface

```
wiki serve
          [--sse [:<port>]]    # enable SSE transport (default port: from config)
          [--acp]              # enable ACP transport
          [--dry-run]          # print what would be started, no server
```

### Examples

```bash
wiki serve                     # stdio only
wiki serve --sse               # stdio + SSE on default port
wiki serve --sse :9090         # stdio + SSE on port 9090
wiki serve --acp               # stdio + ACP
wiki serve --sse --acp         # stdio + SSE + ACP
wiki serve --sse :8080 --acp   # all three, SSE on 8080
```

---

## 4. Startup Sequence

```
1. Load ~/.wiki/config.toml — spaces + global config
2. Mount all registered wikis
3. Check index staleness for each wiki (warn if stale, auto-rebuild if config says so)
4. Start stdio MCP server (always)
5. If --sse: start SSE listener on configured port
6. If --acp: start ACP stdio server
7. Log: "wiki serve — N wikis mounted [stdio] [sse :8080] [acp]"
```

---

## 5. Config Defaults

```toml
[serve]
sse      = false    # enable SSE by default
sse_port = 8080     # SSE port
acp      = false    # enable ACP by default
```

CLI flags override config per-invocation.
