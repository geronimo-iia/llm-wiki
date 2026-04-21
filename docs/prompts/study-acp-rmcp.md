# Study: agent-client-protocol-rmcp Integration

## Context

`agent-client-protocol-rmcp` 0.11 bridges rmcp `ServerHandler`
implementations into ACP sessions. It wraps our existing `McpServer`
(rmcp) so that MCP tools are automatically available to ACP clients
without duplicating tool definitions.

Current state: llm-wiki runs MCP and ACP as separate transports.
MCP tools are not accessible from ACP sessions — the ACP agent
handles prompts manually via `run_research`.

## How the Bridge Works

```rust
use agent_client_protocol::mcp_server::McpServer;
use agent_client_protocol_rmcp::McpServerExt;

let mcp_server = McpServer::from_rmcp("llm-wiki", || crate::mcp::McpServer::new(manager.clone()));
```

`from_rmcp` takes a factory closure that creates a fresh rmcp service
per MCP connection. Internally it:

1. Creates a tokio duplex stream
2. Connects the rmcp service to one end
3. Connects the ACP MCP protocol to the other end
4. Handles `list_tools` / `call_tool` transparently

## Integration Patterns

### Pattern A: Global MCP server (Proxy)

Inject the MCP server into all sessions via `with_mcp_server`:

```rust
Proxy.builder()
    .with_mcp_server(mcp_server)
    .connect_to(conductor)
    .await
```

**Problem**: This is a Proxy pattern. Requires a conductor to
orchestrate client → proxy → agent. We are the agent — there's
no upstream agent to proxy to.

### Pattern B: Per-session MCP server (Proxy)

Create MCP server per session with workspace context:

```rust
Proxy.builder()
    .on_receive_request_from(Client, async |req: NewSessionRequest, responder, cx| {
        let mcp = McpServer::from_rmcp("llm-wiki", || McpServer::new(...));
        cx.build_session_from(req)
            .with_mcp_server(mcp)?
            .on_proxy_session_start(responder, async |session_id| { Ok(()) })
    }, on_receive_request!())
    .connect_to(conductor)
    .await
```

**Same problem**: Proxy pattern, needs conductor.

### Pattern C: Agent with embedded MCP (hybrid)

Register MCP server as a handler in the Agent builder:

```rust
Agent.builder()
    .with_mcp_server(mcp_server)  // if supported
    .on_receive_request(...)
    .connect_to(transport)
    .await
```

**Unknown**: Need to verify if `Agent.builder()` supports
`with_mcp_server`. The cookbook only shows it on `Proxy.builder()`.

## Investigation Steps

- [ ] Check if `Builder::with_mcp_server` is available on `Agent` role
  (read `src/agent-client-protocol/src/mcp_server/mod.rs`)
- [ ] If not, check if `with_handler` can accept an `McpServer`
- [ ] Read how the conductor unwraps `SuccessorMessage` envelopes —
  could we skip the conductor if we handle the envelope ourselves?
- [ ] Check if any ACP client (Zed, VS Code) sends MCP tool calls
  to agents directly, or only through proxy chains
- [ ] Test: does Zed's agent panel discover MCP tools from an ACP
  agent that advertises them in `NewSessionResponse.mcp_servers`?

## Decision Criteria

| Criterion | Weight |
|-----------|--------|
| No architecture change (stay Agent, no conductor) | High |
| MCP tools accessible from ACP sessions | Medium |
| No tool definition duplication | Medium |
| Minimal new dependencies | Low |

## Possible Outcomes

1. **Adopt**: `Agent.builder()` supports `with_mcp_server` → use it
2. **Defer**: Only Proxy supports it → not worth the architecture change
3. **Alternative**: Agent manually handles MCP protocol messages via
   `on_receive_dispatch` — fragile, not recommended

## Dependencies

```toml
agent-client-protocol-rmcp = "0.11"  # only if adopting
```
