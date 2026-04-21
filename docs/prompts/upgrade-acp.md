# Upgrade: agent-client-protocol 0.10 → 0.11

## Decision

Phase 1 — Direct port using the **Agent pattern** (Option A).
Rewrite `src/acp.rs` with the builder API. No proxy, no conductor,
no `agent-client-protocol-rmcp`.

Study doc: [study-acp-0.11.md](study-acp-0.11.md)

## What Changes

The 0.11 SDK replaces the `Agent` trait with a builder pattern.
The connection model drops `LocalSet` + `spawn_local` entirely.

| 0.10 (current) | 0.11 (target) |
|---|---|
| `#[async_trait(?Send)] impl Agent for WikiAgent` | `Agent.builder().on_receive_request(...)` |
| `AgentSideConnection::new(agent, out, in, spawn)` | `Agent.builder().connect_to(ByteStreams::new(...))` |
| `self.send_notification(notif)` via mpsc channel | `connection.send_notification(notif)?` (sync, queues) |
| `LocalSet` + `spawn_local` | Not needed |
| `mpsc::unbounded_channel` + `oneshot` backpressure | Gone — direct `connection.send_notification` |
| Types at `acp::*` | Types at `schema::*` |
| `SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(...)))` | `SessionUpdate::Text(TextUpdate { text, .. })` |
| `acp::ProtocolVersion::LATEST` | `schema::ProtocolVersion::V1` |

## What Stays

- `AcpSession` struct — as-is
- `dispatch_workflow` / `extract_prompt_text` — as-is
- `step_search`, `step_read`, `step_report_results` — same logic, new notification API
- `resolve_wiki_name` — as-is
- `make_tool_id` — as-is
- `run_research` — same composition, new types

## Architecture

The `WikiAgent` struct stays but loses the `update_tx` channel.
Handlers receive `connection: ConnectionTo<Client>` directly and
call `connection.send_notification(...)` inline.

Shared state (`manager`, `sessions`) is captured by move into each
handler closure via `Arc`.

```
Agent.builder()
  ├── on_receive_request(InitializeRequest)   → respond with capabilities
  ├── on_receive_request(NewSessionRequest)   → create session, respond
  ├── on_receive_request(PromptRequest)       → dispatch workflow, stream, respond
  ├── on_receive_request(LoadSessionRequest)  → check session exists
  ├── on_receive_request(ListSessionsRequest) → list sessions
  ├── on_receive_notification(CancelNotification) → clear active run
  ├── on_receive_dispatch(Dispatch)           → reject unknown
  └── connect_to(ByteStreams::new(stdout, stdin))
```

## Dependencies

```toml
agent-client-protocol = "0.11"
# Remove: agent-client-protocol-tokio (no longer needed)
# Not needed: agent-client-protocol-rmcp (Phase 2)
# Remove: async-trait (no longer needed for ACP)
```

Check if `async-trait` is used elsewhere before removing.

## Files

| File | Change |
|---|---|
| `Cargo.toml` | Bump to 0.11, remove `agent-client-protocol-tokio` if unused |
| `src/acp.rs` | Full rewrite — builder pattern, no channel, no `LocalSet` |
| `src/server.rs` | Simplify ACP spawn — no dedicated thread/`LocalSet` needed |
| `tests/acp.rs` | Update for new API |
| `docs/implementation/acp-server.md` | Update |
| `docs/implementation/acp-sdk.md` | Update |

## Steps

- [ ] Create branch `feat/upgrade-acp`
- [ ] Bump `agent-client-protocol` to 0.11 in `Cargo.toml`
- [ ] Remove `agent-client-protocol-tokio` if only used for ACP
- [ ] Rewrite `src/acp.rs`:
  - [ ] Remove `WikiAgent.update_tx`, `send_notification` channel bridge
  - [ ] Move session state + manager into `Arc` for closure capture
  - [ ] Replace `Agent` trait impl with builder handlers
  - [ ] Update notification types (`SessionUpdate::Text(TextUpdate { .. })`)
  - [ ] Update `step_search`, `step_read`, `step_report_results` signatures
    to take `&ConnectionTo<Client>` instead of `&self`
- [ ] Simplify `src/server.rs` ACP transport:
  - [ ] Remove dedicated OS thread + `current_thread` runtime
  - [ ] `serve_acp` returns a future, `tokio::spawn` it alongside MCP
  - [ ] Wire shutdown signal into the builder (or `tokio::select!`)
- [ ] Update `tests/acp.rs`
- [ ] `cargo check && cargo test && cargo clippy`
- [ ] Manual: `llm-wiki serve --acp` works with Zed
- [ ] Update `docs/implementation/acp-server.md`
- [ ] Update `docs/implementation/acp-sdk.md`

## Skeleton

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_client_protocol::{
    Agent, Client, ConnectionTo, Dispatch,
    on_receive_request, on_receive_dispatch, on_receive_notification,
    ByteStreams,
    schema::{
        AgentCapabilities, InitializeRequest, InitializeResponse,
        NewSessionRequest, NewSessionResponse, SessionId,
        PromptRequest, PromptResponse, StopReason,
        LoadSessionRequest, LoadSessionResponse,
        ListSessionsRequest, ListSessionsResponse,
        CancelNotification, SessionNotification, SessionUpdate, TextUpdate,
        ProtocolVersion,
    },
};
use anyhow::Result;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::engine::WikiEngine;

pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let sessions: Arc<Mutex<HashMap<String, AcpSession>>> =
        Arc::new(Mutex::new(HashMap::new()));

    Agent.builder()
        .name("llm-wiki")

        // ── Initialize ──
        .on_receive_request({
            async move |req: InitializeRequest, responder, _cx| {
                responder.respond(
                    InitializeResponse::new(req.protocol_version)
                        .agent_capabilities(AgentCapabilities::new()),
                )
            }
        }, on_receive_request!())

        // ── NewSession ──
        .on_receive_request({
            let sessions = sessions.clone();
            async move |req: NewSessionRequest, responder, _cx| {
                let id = format!("session-{}", chrono::Utc::now().timestamp_millis());
                let wiki = req.meta.as_ref()
                    .and_then(|m| m.get("wiki"))
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let session = AcpSession {
                    id: id.clone(),
                    label: None,
                    wiki,
                    created_at: chrono::Utc::now().timestamp() as u64,
                    active_run: None,
                };
                sessions.lock().unwrap().insert(id.clone(), session);
                responder.respond(NewSessionResponse::new(SessionId::new(id)))
            }
        }, on_receive_request!())

        // ── Prompt ──
        .on_receive_request({
            let mgr = manager.clone();
            let sessions = sessions.clone();
            async move |req: PromptRequest, responder, connection: ConnectionTo<Client>| {
                let text = extract_prompt_text(&req);
                let (workflow, query) = dispatch_workflow(&text);
                let wiki_name = resolve_wiki_name(&mgr, &sessions, &req.session_id);
                let query_text = if query.is_empty() { &text } else { query };

                match workflow {
                    "research" => {
                        run_research(&mgr, &sessions, &connection, &req.session_id, query_text, &wiki_name).await?;
                    }
                    other => {
                        send_text(&connection, &req.session_id, &format!(
                            "Unknown workflow \"{other}\". Use `llm-wiki:research <query>`."
                        ))?;
                    }
                }

                responder.respond(PromptResponse::new(StopReason::EndTurn))
            }
        }, on_receive_request!())

        // ── Cancel ──
        .on_receive_notification({
            let sessions = sessions.clone();
            async move |notif: CancelNotification, _cx| {
                if let Ok(mut s) = sessions.lock() {
                    if let Some(sess) = s.get_mut(&notif.session_id.to_string()) {
                        sess.active_run = None;
                    }
                }
                Ok(())
            }
        }, on_receive_notification!())

        // ── Catch-all ──
        .on_receive_dispatch(
            async move |msg: Dispatch, cx: ConnectionTo<Client>| {
                msg.respond_with_error(
                    agent_client_protocol::util::internal_error("not supported"),
                    cx,
                )
            },
            on_receive_dispatch!(),
        )

        .connect_to(ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {e}"))
}

// send_text, send_tool_call, send_tool_result become free functions
// taking &ConnectionTo<Client> + &SessionId instead of &self.

fn send_text(
    cx: &ConnectionTo<Client>,
    session_id: &SessionId,
    text: &str,
) -> std::result::Result<(), agent_client_protocol::Error> {
    cx.send_notification(SessionNotification {
        session_id: session_id.clone(),
        update: SessionUpdate::Text(TextUpdate { text: text.into(), ..Default::default() }),
        meta: None,
    })
}
```

## Notes

- `send_notification` in 0.11 is **synchronous** (queues the message).
  No more `async` + `oneshot` backpressure dance.
- The `connection` parameter in `on_receive_request` handlers is
  `ConnectionTo<Client>` — it can send notifications and requests
  toward the client.
- `LoadSessionRequest` and `ListSessionsRequest` handlers omitted
  from skeleton for brevity — same pattern as 0.10, just different
  types.
- Shutdown: the builder's `connect_to` future resolves when the
  transport closes. Wrap in `tokio::select!` with the shutdown signal.
- The dedicated OS thread in `server.rs` may no longer be needed if
  the 0.11 builder is `Send`. Verify before removing.
