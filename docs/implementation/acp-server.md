---
title: "ACP Server Implementation"
summary: "Builder-pattern agent, session management, streaming helpers, and prompt dispatch."
status: ready
last_updated: "2025-07-21"
---

# ACP Server Implementation

Implementation reference for the ACP transport. Not a specification —
see [acp-transport.md](../specifications/integrations/acp-transport.md)
for the design.

## Overview

The ACP server runs as a `tokio::spawn` task alongside the MCP
stdio/SSE transports. It uses the `Agent.builder()` pattern from
`agent-client-protocol` 0.11, registering request/notification
handlers as closures that capture shared state via `Arc`.

Decision: [acp-builder-pattern.md](../decisions/acp-builder-pattern.md)

## Module Structure

```
src/acp/
├── mod.rs        — AcpSession, Sessions type, dispatch_workflow, extract_prompt_text, make_tool_id
├── helpers.rs    — send_text, send_tool_call, send_tool_result, resolve_wiki_name, session_cwd, clear_active_run
├── research.rs   — step_search, step_read, step_report_results, run_research
└── server.rs     — serve_acp (builder wiring)
```

## Session

```rust
pub struct AcpSession {
    pub id: String,
    pub label: Option<String>,
    pub wiki: Option<String>,
    pub created_at: u64,
    pub active_run: Option<String>,
}

type Sessions = Arc<Mutex<HashMap<String, AcpSession>>>;
```

Sessions are in-memory only — lost on restart.

## Builder Handlers

```
Agent.builder()
  ├── on_receive_request(InitializeRequest)        → capabilities + agent info
  ├── on_receive_request(NewSessionRequest)        → create session
  ├── on_receive_request(LoadSessionRequest)       → check session exists
  ├── on_receive_request(ListSessionsRequest)      → list sessions
  ├── on_receive_request(PromptRequest)            → dispatch workflow, stream, respond
  ├── on_receive_notification(CancelNotification)  → clear active run
  ├── on_receive_dispatch(Dispatch)                → reject unknown
  └── connect_to(ByteStreams::new(stdout, stdin))
```

Each handler captures `Arc<WikiEngine>` and `Sessions` by clone.

## Streaming Helpers

Three free functions in `helpers.rs`:

| Helper             | Event type          | When                              |
| ------------------ | ------------------- | --------------------------------- |
| `send_text`        | `AgentMessageChunk` | Progress text, final summary      |
| `send_tool_call`   | `ToolCall`          | Announce a tool invocation        |
| `send_tool_result` | `ToolCallUpdate`    | Report tool completion or failure |

All take `&ConnectionTo<Client>` + `&SessionId`. Notifications are
synchronous (queued by the SDK, not async).

Tool call IDs: `{workflow}-{step}-{timestamp_ms}`.

## Prompt Dispatch

`llm-wiki:` prefix convention:

```
llm-wiki:research what is MoE?    → research workflow
llm-wiki:ingest                   → stream ingest skill instructions
what do we know about MoE?        → fallback to research
```

Parsing: strip `llm-wiki:` prefix, split on first space into
(workflow, text). No prefix → default to `research`.

### Engine-executed workflows

| Workflow   | What it does                                        |
| ---------- | --------------------------------------------------- |
| `research` | `wiki_search` + `wiki_content_read`, stream results |

### Skill-delegated workflows (future)

| Workflow      | What it streams                |
| ------------- | ------------------------------ |
| `ingest`      | Ingest skill instructions      |
| `crystallize` | Crystallize skill instructions |

## Connection Setup

```rust
pub async fn serve_acp(manager: Arc<WikiEngine>) -> Result<()> {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));

    Agent.builder()
        .name("llm-wiki")
        // ... handlers ...
        .connect_to(ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {e}"))
}
```

No `LocalSet`, no `spawn_local`, no dedicated thread. The builder
is `Send` — runs on the main tokio runtime via `tokio::spawn`.

## Crate

```toml
agent-client-protocol = "0.11"
```
