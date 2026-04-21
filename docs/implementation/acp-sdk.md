# ACP SDK Usage Reference

Reference for `agent-client-protocol` v0.11 as used in llm-wiki.

---

## Crate

| Crate | Version | Role |
|-------|---------|------|
| `agent-client-protocol` | 0.11 | Builder API, connection, schema types |

The schema crate (`agent-client-protocol-schema` 0.12) is re-exported
through `agent_client_protocol::schema::*`.

---

## Agent Builder

The 0.11 SDK uses a builder pattern instead of a trait. Register
handlers for each message type:

```rust
Agent.builder()
    .name("llm-wiki")
    .on_receive_request(
        async move |req: InitializeRequest, responder, _cx| {
            responder.respond(
                InitializeResponse::new(req.protocol_version)
                    .agent_capabilities(AgentCapabilities::new()),
            )
        },
        on_receive_request!(),
    )
    .on_receive_dispatch(
        async move |msg: Dispatch, cx: ConnectionTo<Client>| {
            msg.respond_with_error(util::internal_error("not supported"), cx)
        },
        on_receive_dispatch!(),
    )
    .connect_to(ByteStreams::new(stdout, stdin))
    .await
```

Each handler receives:
- The typed request/notification
- A `Responder` (for requests) to send the response
- A `ConnectionTo<Client>` to send notifications back

No `LocalSet`, no `spawn_local`, no `!Send` constraint.

---

## Streaming via SessionNotification

Send `SessionNotification` directly through `ConnectionTo<Client>`.
The call is synchronous (queues the message):

```rust
cx.send_notification(SessionNotification::new(
    session_id.clone(),
    SessionUpdate::AgentMessageChunk(ContentChunk::new(
        ContentBlock::Text(TextContent::new("Hello")),
    )),
))?;
```

### SessionUpdate Variants (relevant subset)

| Variant | Purpose | When to use |
|---------|---------|-------------|
| `AgentMessageChunk(ContentChunk)` | Stream text to the user | Progress messages, final answers |
| `AgentThoughtChunk(ContentChunk)` | Stream internal reasoning | Optional, for transparency |
| `ToolCall(ToolCall)` | Announce a tool invocation | Before executing a tool |
| `ToolCallUpdate(ToolCallUpdate)` | Update tool status/output | After tool completes or fails |
| `SessionInfoUpdate(SessionInfoUpdate)` | Update session metadata | Title changes, etc. |

### ContentChunk

Wraps a single `ContentBlock`:

```rust
ContentChunk::new(ContentBlock::Text(TextContent::new("Searching...")))
```

### ToolCall

Announces a new tool invocation. Visible in the IDE as a collapsible step:

```rust
ToolCall::new(ToolCallId::new("search-001"), "Searching for: MoE scaling")
    .kind(ToolKind::Search)
    .status(ToolCallStatus::InProgress)
```

### ToolCallUpdate

Updates an existing tool call by ID:

```rust
ToolCallUpdate::new(
    ToolCallId::new("search-001"),
    ToolCallUpdateFields::new()
        .status(ToolCallStatus::Completed)
        .content(vec!["Found 3 results".into()]),
)
```

---

## Streaming Pattern: Tool Call Lifecycle

```
1. SessionUpdate::ToolCall(new(id, title).kind(K).status(InProgress))
2. ... execute the tool ...
3. SessionUpdate::ToolCallUpdate(new(id, fields.status(Completed).content([...])))
```

On error:

```
1. SessionUpdate::ToolCall(new(id, title).kind(K).status(InProgress))
2. ... tool fails ...
3. SessionUpdate::ToolCallUpdate(new(id, fields.status(Failed).content(["error: ..."])))
```

---

## Connection Setup

```rust
pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));

    Agent.builder()
        .name("llm-wiki")
        // ... handlers capture manager + sessions via Arc::clone ...
        .connect_to(ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {e}"))
}
```

The future resolves when the transport closes. In `server.rs` it
runs as a `tokio::spawn` task with `tokio::select!` for shutdown.

---

## Companion Crates (not used yet)

| Crate | Purpose | Status |
|-------|---------|--------|
| `agent-client-protocol-rmcp` | Bridge rmcp `ServerHandler` into ACP sessions | Evaluated, not adopted — see [study-acp-rmcp.md](../prompts/study-acp-rmcp.md) |
| `agent-client-protocol-tokio` | Tokio spawn utilities | Not needed with 0.11 builder |
| `agent-client-protocol-conductor` | Proxy chain orchestration | Not applicable (we're an Agent, not a Proxy) |
