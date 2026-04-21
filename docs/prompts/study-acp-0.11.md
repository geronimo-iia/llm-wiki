# Study: agent-client-protocol 0.11 Migration

## Architecture Change

The 0.11 SDK is a complete rewrite. The old `Agent` trait with
`initialize`/`new_session`/`prompt`/`cancel` methods is gone. The new
API uses a **builder pattern with message handlers**:

```rust
Agent.builder()
    .name("llm-wiki")
    .on_receive_request(async |req: InitializeRequest, responder, _cx| {
        responder.respond(InitializeResponse::new(req.protocol_version)
            .agent_capabilities(AgentCapabilities::new()))
    }, on_receive_request!())
    .on_receive_request(async |req: NewSessionRequest, responder, _cx| {
        responder.respond(NewSessionResponse::new(SessionId::new("session-1")))
    }, on_receive_request!())
    .on_receive_request(async |req: PromptRequest, responder, connection| {
        // Stream notifications via connection.send_notification(...)
        responder.respond(PromptResponse::new(StopReason::EndTurn))
    }, on_receive_request!())
    .connect_to(ByteStreams::new(stdout, stdin))
    .await
```

## Key Differences from 0.10

| 0.10 | 0.11 |
|------|------|
| `#[async_trait] impl Agent for WikiAgent` | `Agent.builder().on_receive_request(...)` |
| `AgentSideConnection::new(agent, out, in)` | `Agent.builder().connect_to(ByteStreams::new(...))` |
| `self.send_notification(notif)` | `connection.send_notification(notif)?` |
| `mpsc::unbounded_channel` for notifications | Direct `connection.send_notification` |
| `LocalSet` + `spawn_local` | Not needed — builder handles async |
| Types at `acp::SessionId`, `acp::PromptRequest` | Types at `schema::SessionId`, `schema::PromptRequest` |

## Type Mapping

All types moved to `agent_client_protocol::schema::*`:

| Old (0.10) | New (0.11) |
|------------|-----------|
| `acp::InitializeRequest` | `schema::InitializeRequest` |
| `acp::InitializeResponse` | `schema::InitializeResponse` |
| `acp::NewSessionRequest` | `schema::NewSessionRequest` |
| `acp::NewSessionResponse` | `schema::NewSessionResponse` |
| `acp::PromptRequest` | `schema::PromptRequest` |
| `acp::PromptResponse` | `schema::PromptResponse` |
| `acp::SessionNotification` | `schema::SessionNotification` |
| `acp::SessionUpdate` | `schema::SessionUpdate` |
| `acp::SessionId` | `schema::SessionId` |
| `acp::StopReason` | `schema::StopReason` |
| `acp::ContentBlock::Text(TextContent)` | `schema::SessionUpdate::Text(TextUpdate)` |
| `acp::ToolCall` | `schema::SessionUpdate::ToolCall(...)` |
| `acp::ToolCallUpdate` | `schema::SessionUpdate::ToolCallUpdate(...)` |
| `acp::AgentCapabilities` | `schema::AgentCapabilities` |
| `acp::ProtocolVersion::LATEST` | `schema::ProtocolVersion::V1` |
| `acp::Error` | `agent_client_protocol::Error` |
| `acp::Agent` (trait) | `Agent` (role marker struct) |
| `acp::Client` (trait) | `Client` (role marker struct) |

## Streaming Notifications

Old (0.10):
```rust
let notif = acp::SessionNotification::new(session_id, update);
self.send_notification(notif).await?;
```

New (0.11):
```rust
connection.send_notification(SessionNotification {
    session_id: session_id.clone(),
    update: SessionUpdate::Text(TextUpdate { text: "...".into(), .. }),
    meta: None,
})?;
```

Note: `send_notification` is synchronous (queues the message), not async.

## Connection Model

Old: `AgentSideConnection` with manual `LocalSet` + `spawn_local`:
```rust
let local_set = tokio::task::LocalSet::new();
local_set.run_until(async {
    let (conn, handle_io) = AgentSideConnection::new(agent, out, in, |fut| {
        tokio::task::spawn_local(fut);
    });
    // ...
    handle_io.await
}).await
```

New: Builder connects directly:
```rust
Agent.builder()
    .name("llm-wiki")
    // ... handlers ...
    .connect_to(ByteStreams::new(stdout.compat_write(), stdin.compat()))
    .await
```

No `LocalSet`, no `spawn_local`, no manual channel wiring.

## rmcp Integration (agent-client-protocol-rmcp)

The `McpServer::from_rmcp` bridge wraps our existing rmcp `ServerHandler`
as an ACP MCP server. This means we could potentially:

1. Keep our `McpServer` (rmcp `ServerHandler`) unchanged
2. Wrap it with `McpServer::from_rmcp("llm-wiki", || McpServer::new(...))`
3. Register it as a handler in the Agent builder

This would give us MCP tools exposed through ACP automatically —
no separate MCP stdio server needed when running in ACP mode.

**However**: this is a Proxy pattern (MCP server injected into sessions),
not an Agent pattern. Our current architecture is Agent (handles prompts
directly). We need to decide:

- **Option A**: Keep as Agent, manually handle prompts + stream notifications
- **Option B**: Use Proxy pattern with rmcp integration — MCP tools
  auto-injected into sessions, agent handles prompts

Option A is closer to our current design. Option B is more powerful
but changes the architecture.

## Migration Plan for llm-wiki

### Phase 1: Direct port (Option A)

Rewrite `src/acp.rs` using the builder pattern:

```rust
pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let mgr = manager.clone();
    Agent.builder()
        .name("llm-wiki")
        .on_receive_request(async move |req: InitializeRequest, responder, _cx| {
            responder.respond(
                InitializeResponse::new(req.protocol_version)
                    .agent_capabilities(AgentCapabilities::new())
            )
        }, on_receive_request!())
        .on_receive_request({
            let mgr = mgr.clone();
            async move |req: NewSessionRequest, responder, _cx| {
                // Create session, store in shared state
                let id = format!("session-{}", chrono::Utc::now().timestamp_millis());
                responder.respond(NewSessionResponse::new(SessionId::new(id)))
            }
        }, on_receive_request!())
        .on_receive_request({
            let mgr = mgr.clone();
            async move |req: PromptRequest, responder, connection| {
                // Dispatch workflow, stream via connection.send_notification(...)
                let text = extract_prompt_text(&req);
                let (workflow, query) = dispatch_workflow(&text);
                // ... step_search, step_read, step_report_results ...
                responder.respond(PromptResponse::new(StopReason::EndTurn))
            }
        }, on_receive_request!())
        .on_receive_dispatch(async |msg: Dispatch, cx: ConnectionTo<Client>| {
            msg.respond_with_error(util::internal_error("not implemented"), cx)
        }, on_receive_dispatch!())
        .connect_to(ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {e}"))
}
```

### What stays the same

- `WikiAgent` session management (HashMap of sessions)
- `dispatch_workflow` / `extract_prompt_text` logic
- `step_search`, `step_read`, `step_report_results` (change notification API)
- `resolve_wiki_name` logic

### What changes

- No `Agent` trait impl — builder pattern instead
- No `AgentSideConnection` — `Agent.builder().connect_to()`
- No `LocalSet` / `spawn_local` — builder handles async
- No `mpsc::unbounded_channel` for notifications — direct `connection.send_notification`
- Notification types: `SessionUpdate::Text(TextUpdate { ... })` instead of
  `SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(...)))`
- `ToolCall` / `ToolCallUpdate` types may have different structure

### Dependencies

```toml
agent-client-protocol = "0.11"
agent-client-protocol-tokio = "0.11"  # if needed for spawn utilities
# agent-client-protocol-rmcp not needed for Phase 1
```
