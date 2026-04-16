# ACP SDK Usage Reference

Reference for `agent-client-protocol` v0.10 / `agent-client-protocol-schema`
v0.11 as used in llm-wiki.

---

## Crates

| Crate | Version | Role |
|-------|---------|------|
| `agent-client-protocol` | 0.10 | Agent trait, connection, session management |
| `agent-client-protocol-schema` | 0.11 | Types: SessionUpdate, ToolCall, ContentChunk, etc. |

The schema crate is re-exported through `agent-client-protocol`.

---

## Agent Trait

The `Agent` trait is the main integration point. Implement it on a struct
that holds your state:

```rust
#[async_trait::async_trait(?Send)]
impl acp::Agent for WikiAgent {
    async fn initialize(&self, req: InitializeRequest) -> Result<InitializeResponse, Error>;
    async fn authenticate(&self, req: AuthenticateRequest) -> Result<AuthenticateResponse, Error>;
    async fn new_session(&self, req: NewSessionRequest) -> Result<NewSessionResponse, Error>;
    async fn load_session(&self, req: LoadSessionRequest) -> Result<LoadSessionResponse, Error>;
    async fn list_sessions(&self, req: ListSessionsRequest) -> Result<ListSessionsResponse, Error>;
    async fn prompt(&self, req: PromptRequest) -> Result<PromptResponse, Error>;
    async fn cancel(&self, notif: CancelNotification) -> Result<(), Error>;
}
```

Note: `?Send` — the agent runs on a `tokio::task::LocalSet`, not a
multi-threaded runtime. This is required by the SDK's connection model.

---

## Streaming via SessionNotification

The agent streams events to the client by sending `SessionNotification`
messages through the connection. Each notification wraps a `SessionUpdate`
variant.

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
ToolCall::new(
    ToolCallId::new("search-001"),
    "Searching for: MoE scaling",
)
.kind(ToolKind::Search)
.status(ToolCallStatus::InProgress)
```

Fields:
- `tool_call_id: ToolCallId` — unique ID within the session
- `title: String` — human-readable description
- `kind: ToolKind` — icon hint: `Search`, `Read`, `Edit`, `Execute`, etc.
- `status: ToolCallStatus` — `Pending`, `InProgress`, `Completed`, `Failed`
- `content: Vec<ToolCallContent>` — output content
- `locations: Vec<ToolCallLocation>` — affected file paths
- `raw_input: Option<Value>` — tool input parameters
- `raw_output: Option<Value>` — tool output

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

All fields in `ToolCallUpdateFields` are optional — only include what changed.
Collections (`content`, `locations`) are overwritten, not extended.

### ToolKind

```rust
pub enum ToolKind {
    Read,       // reading files or data
    Edit,       // modifying files
    Delete,     // removing files
    Move,       // renaming/moving
    Search,     // searching
    Execute,    // running commands
    Think,      // internal reasoning
    Fetch,      // external data retrieval
    SwitchMode, // session mode change
    Other,      // default
}
```

### ToolCallStatus

```rust
pub enum ToolCallStatus {
    Pending,     // not started (awaiting input or approval)
    InProgress,  // currently running
    Completed,   // finished successfully
    Failed,      // finished with error
}
```

---

## Sending Notifications

The agent sends notifications through the `AgentSideConnection`:

```rust
let notif = SessionNotification::new(
    session_id.clone(),
    SessionUpdate::AgentMessageChunk(ContentChunk::new(
        ContentBlock::Text(TextContent::new("Hello")),
    )),
);
conn.session_notification(notif).await?;
```

In llm-wiki, the connection is not directly accessible from the `Agent`
trait methods. Instead, notifications are sent through an
`mpsc::UnboundedSender` channel that bridges the agent to the connection:

```rust
pub struct WikiAgent {
    update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
}

impl WikiAgent {
    async fn send_message(&self, session_id: &SessionId, text: &str) -> Result<(), Error> {
        let notif = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                ContentBlock::Text(TextContent::new(text)),
            )),
        );
        let (tx, rx) = oneshot::channel();
        self.update_tx.send((notif, tx)).map_err(|_| Error::internal_error())?;
        rx.await.map_err(|_| Error::internal_error())
    }
}
```

The same pattern applies for `ToolCall` and `ToolCallUpdate` — wrap in
`SessionNotification` with the appropriate `SessionUpdate` variant and send
through the channel.

---

## Streaming Pattern: Tool Call Lifecycle

The canonical pattern for a tool call with streaming:

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
pub async fn serve_acp(global: Arc<GlobalConfig>) -> Result<()> {
    let outgoing = tokio::io::stdout().compat_write();
    let incoming = tokio::io::stdin().compat();

    let local_set = tokio::task::LocalSet::new();
    local_set.run_until(async move {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let agent = WikiAgent::new(global, tx);

        let (conn, handle_io) =
            AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                tokio::task::spawn_local(fut);
            });

        // Bridge: drain channel → send notifications via connection
        tokio::task::spawn_local(async move {
            while let Some((notif, tx)) = rx.recv().await {
                if let Err(e) = conn.session_notification(notif).await {
                    eprintln!("ACP notification error: {e}");
                    break;
                }
                tx.send(()).ok();
            }
        });

        handle_io.await
    }).await?;
    Ok(())
}
```

Key constraints:
- `LocalSet` required — the agent is `!Send`
- `spawn_local` for all async tasks
- `oneshot` channel per notification for backpressure
