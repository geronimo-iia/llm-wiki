# Decision: rmcp 1.x — Streamable HTTP

## Problem

rmcp 0.1 pulled `paste` (unmaintained, RUSTSEC-2024-0436). The SSE
transport feature (`transport-sse-server`) was removed in rmcp 1.x,
replaced by Streamable HTTP.

## Decision

Upgrade to rmcp 1.x. Replace SSE with Streamable HTTP. Rename all
config keys from `sse` to `http`.

## Transport

| Before | After |
|---|---|
| `SseServer::serve(addr)` | `StreamableHttpService` + `axum::serve` |
| `transport-sse-server` feature | `transport-streamable-http-server` |
| `--sse` CLI flag | `--http` |
| `serve.sse` / `serve.sse_port` | `serve.http` / `serve.http_port` |

New config key: `serve.http_allowed_hosts` (default: localhost only).

`StreamableHttpServerConfig` uses `stateful_mode: true` (session per
client, equivalent to SSE behavior) and `CancellationToken` for
graceful shutdown.

## ACP Bridge — Deferred

`agent-client-protocol-rmcp` wraps our rmcp `ServerHandler` into ACP
sessions. Investigated whether `Agent.builder().with_mcp_server()`
could expose MCP tools in ACP sessions.

Result: `with_mcp_server` requires `Host::Counterpart: HasPeer<Agent>
+ HasPeer<Client>` — only satisfied by `Conductor`, not `Client`.
Does not compile on `Agent.builder()`. Proxy pattern + conductor
required — not worth the architecture change.

MCP and ACP remain separate transports.

## What changed

- `Cargo.toml`: rmcp 0.1 → 1, added `axum` 0.8
- `src/mcp/mod.rs`: `Param` → `Params`, `Option<>` on list methods,
  `get_peer`/`set_peer` removed (peer via `context.peer`),
  `rmcp::Error` → `rmcp::ErrorData`, constructors for non-exhaustive types
- `src/config.rs`: `sse` → `http`, added `http_allowed_hosts`
- `src/cli.rs`: `--sse` → `--http`
- `src/server.rs`: `SseServer` → `StreamableHttpService` + axum,
  `CancellationToken` for HTTP shutdown
- `src/main.rs`: `sse` → `http` in Serve dispatch
- `audit.toml`: `paste` ignore removed, `boxfnonce` added (ACP transitive)
