---
title: "ACP Server Implementation"
summary: "WikiAgent, session management, streaming helpers, and prompt dispatch."
status: ready
last_updated: "2025-07-17"
---

# ACP Server Implementation

Implementation reference for the ACP transport. Not a specification —
see [acp-transport.md](../specifications/integrations/acp-transport.md)
for the design.

## Overview

The ACP server runs as a dedicated thread alongside the MCP stdio/SSE
transports. It implements the `Agent` trait from the
`agent-client-protocol` crate, handling sessions and streaming
workflow results back to the IDE.

## WikiAgent

The core struct implementing the ACP `Agent` trait:

```rust
struct WikiAgent {
    engine: Arc<RwLock<EngineState>>,
    sessions: Mutex<HashMap<String, AcpSession>>,
    update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
}

struct AcpSession {
    id: String,
    label: Option<String>,
    wiki: Option<String>,
    created_at: u64,
    active_run: Option<String>,
}
```

`WikiAgent` holds a reference to the shared `EngineState` (same as MCP
tools) and manages its own session state. Sessions are in-memory only
— lost on restart.

## Agent Trait

| Method          | Purpose                                                 |
| --------------- | ------------------------------------------------------- |
| `initialize`    | Return capabilities, agent info                         |
| `new_session`   | Create a session, optionally targeting a wiki           |
| `load_session`  | Resume an existing session                              |
| `list_sessions` | List active sessions                                    |
| `prompt`        | Receive user message, dispatch workflow, stream results |
| `cancel`        | Cancel active run                                       |

## Streaming Helpers

Three helpers for sending events back to the IDE:

| Helper             | Event type          | When                              |
| ------------------ | ------------------- | --------------------------------- |
| `send_message`     | `AgentMessageChunk` | Progress text, final summary      |
| `send_tool_call`   | `ToolCall`          | Announce a tool invocation        |
| `send_tool_result` | `ToolCallUpdate`    | Report tool completion or failure |

Tool call IDs follow the convention: `{workflow}-{step}-{timestamp_ms}`.

All helpers go through a channel (`update_tx`) to the connection
handler, which sends the notification over stdio. This decouples the
workflow logic from the transport.

## Prompt Dispatch

The current code uses keyword matching on prompt text. This is fragile
and will be replaced.

### New dispatch

Use the `llm-wiki:` prefix convention:

```
llm-wiki:research what is MoE?    -> research workflow
llm-wiki:ingest                   -> stream ingest skill instructions
what do we know about MoE?        -> fallback to research
```

Parsing: strip `llm-wiki:` prefix, split on first space into
(workflow, text). No prefix → keyword fallback → default to research.

### Engine-executed workflows

The ACP agent calls engine functions directly and streams results:

| Workflow   | What it does                                        |
| ---------- | --------------------------------------------------- |
| `research` | `wiki_search` + `wiki_content_read`, stream results |

### Skill-delegated workflows

The ACP agent streams skill instructions. The IDE's LLM reads them
and executes using MCP tools:

| Workflow      | What it streams                |
| ------------- | ------------------------------ |
| `ingest`      | Ingest skill instructions      |
| `crystallize` | Crystallize skill instructions |

Skill instructions come from the `llm-wiki-skills` plugin, not from
the engine binary.

## Connection Setup

The ACP server uses `agent-client-protocol-tokio` for stdio transport:

1. Create a `LocalSet` (ACP agent is `!Send`)
2. Create the notification channel (`mpsc::unbounded_channel`)
3. Build `WikiAgent` with the channel sender
4. Create `AgentSideConnection` with stdin/stdout
5. Spawn the notification forwarder
6. Run the connection handler

The `!Send` constraint means the entire ACP runtime runs on a
dedicated OS thread with its own tokio `LocalSet`. See
[server.md](../specifications/engine/server.md) for transport
supervision.

## Existing Code

The current `src/acp.rs` is partially reusable:

| Component           | Reusable | Notes                                                                |
| ------------------- | -------- | -------------------------------------------------------------------- |
| `WikiAgent` struct  | mostly   | Replace `GlobalConfig` + `Vec<WikiEntry>` with `Arc<RwLock<EngineState>>` |
| `AcpSession` struct | yes      | As-is                                                                |
| Streaming helpers   | yes      | `send_message`, `send_tool_call`, `send_tool_result`                 |
| `make_tool_id`      | yes      | As-is                                                                |
| `Agent` trait impl  | mostly   | Update tool names, remove `INSTRUCTIONS` injection                   |
| `run_research`      | mostly   | Use engine instead of direct function calls                          |
| `run_lint`          | remove   | Lint moved to skills                                                 |
| `dispatch_workflow` | rewrite  | Replace keyword matching with prefix dispatch                        |
| `serve_acp`         | yes      | Connection setup is correct                                          |

### Changes needed

- Replace direct `crate::search::search` / `crate::markdown::read_page`
  calls with `EngineState` methods
- Remove `INSTRUCTIONS` injection at `initialize` — skills handle this
- Remove `run_lint` — lint is a skill
- Add prefix-based dispatch (`llm-wiki:` convention)
- Remove ingest/crystallize placeholder strings — stream skill
  instructions instead (or delegate to IDE)

## Crates

```toml
agent-client-protocol       = "0.10"
agent-client-protocol-tokio = "0.1"
```

Reference: https://docs.rs/agent-client-protocol/latest/
