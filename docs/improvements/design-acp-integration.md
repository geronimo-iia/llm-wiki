---
title: "Design: ACP Integration — Smart Use of the Protocol"
summary: "Review of ACP capabilities against current llm-wiki usage. Identifies underused protocol features and proposes improvements: session state, proactive push, skill activation tracking, and cancellation."
read_when:
  - Planning ACP work beyond v0.3.0
  - Understanding what ACP enables that MCP cannot
  - Reviewing the session model and its limitations
status: proposal
last_updated: "2026-05-01"
---

# Design: ACP Integration — Smart Use of the Protocol

## What ACP Actually Is

ACP is a session-oriented, streaming NDJSON protocol over stdio. The key
properties that distinguish it from MCP:

| Property       | MCP                        | ACP                                               |
| -------------- | -------------------------- | ------------------------------------------------- |
| Model          | Stateless request/response | Stateful named sessions                           |
| Streaming      | Not visible to user        | Every step streams as events                      |
| Cancellation   | Not supported              | `cancel` message mid-workflow                     |
| Session memory | None                       | Server can accumulate per-session context         |
| Server push    | No                         | Server can send messages outside request/response |
| LLM agency     | LLM invokes tools freely   | Server runs fixed workflow, streams progress      |

The fundamental constraint: **in ACP, the LLM sends one prompt — the server
runs a fixed Rust workflow**. The LLM has no agency mid-workflow. For
multi-step agentic decisions, MCP is required.

The design opportunity: ACP sessions are stateful. Each session is a
long-lived object on the server. We are barely using this.

## Current State (v0.3.0)

### What we have

Six workflows dispatched by prefix (`research`, `lint`, `graph`, `ingest`,
`use`, `help`). Session carries only `wiki: Option<String>`. Each prompt
is handled independently — no state from prior prompts is used.

### What we underuse

| ACP feature        | Current usage          | Potential                                  |
| ------------------ | ---------------------- | ------------------------------------------ |
| `LoadSession`      | Accepted, no-op        | Restore session context across restarts    |
| `ListSessions`     | Returns in-memory list | —                                          |
| Session state      | Wiki name only         | Active skill, search history, recent slugs |
| Multi-turn context | None                   | Accumulate slug context across prompts     |
| Cancel             | Accepted, no-op        | Actually interrupt long ops                |
| Server push        | Not used               | Watcher events → active sessions           |

## Design Proposals

### 1. Real Cancellation

Current cancel handler is a stub. For long operations (ingest of a large
directory, full-wiki lint), cancellation matters.

ACP `cancel` sets a flag on the `AcpSession`. Workflows must check it
between steps. Pattern:

```rust
// In each step function:
if session.cancelled.load(Ordering::Relaxed) {
    send_text(cx, session_id, "Cancelled.")?;
    return Ok(());
}
```

`cancelled` is an `Arc<AtomicBool>` shared between the session and the cancel
handler. The cancel handler sets it; the workflow polls it at step boundaries.

This gives cooperative cancellation — not preemptive, but sufficient for
workflows that iterate over pages or findings.

### 2. Session Policy Management

Sessions currently accumulate indefinitely. A long-running server with
many IDE restarts leaks memory. Need a hard cap:

**Max sessions** — when reached, reject `NewSession` with a clear error.
Configurable in `[serve]`:

```toml
[serve]
acp_max_sessions = 20   # default: 20
```

**Active run protection** — never reject a `NewSession` when evicting would
drop a session with an active run. Track `active_run: bool` on `AcpSession`.

**`ListSessions` gains `active_run` field:**

```json
{
  "session_id": "abc123",
  "wiki": "research",
  "active_run": false
}
```

`serve.acp_max_sessions` is global-only (cannot be overridden per wiki).

`specifications/model/global-config.md` must document this key under `[serve]`.

### 3. Session Storage — Memory Only

All ACP reference implementations (Python, Rust, TypeScript SDKs) keep
sessions in memory only. The ACP spec has no persistence requirements —
sessions are explicitly designed as ephemeral per-connection objects.

llm-wiki follows the same approach. No disk writes, no `LoadSession`
restoration. Session state is lost on process restart.

Cost is negligible: restoring context is one command (`llm-wiki:use skills/research`).
The engine stays a dumb pipe with no I/O side effects from session management.

### 4. Proactive Push from the Filesystem Watcher

Today, `llm-wiki serve --watch` runs a filesystem watcher that auto-ingests
files dropped in `inbox/`. The watcher fires events internally, but they are
never surfaced to ACP clients.

Proposal: when a file is ingested via the watcher, push a notification to all
active ACP sessions targeting that wiki:

```
→ message: "📥 inbox/paper.pdf ingested → raw/paper.pdf (1 page indexed)"
```

Implementation: the watcher already holds a reference to `WikiEngine`. Add a
`Sessions` reference to the watcher context. On successful ingest, iterate
sessions, filter by wiki, call `send_text`.

This turns the IDE panel into a live feed — users see ingest results without
triggering a prompt.

Design constraint: push messages must not interfere with an active `Run`. Only
push when `session.active_run.is_none()`.

## What ACP Should Not Do

These would be mistakes:

**Write operations via ACP** — `wiki_content_write` and `wiki_content_new`
belong in MCP. ACP's fixed workflows can't make the multi-step decisions
required for good page authoring. Keep write ops in MCP where the LLM has
tool-call agency.

**Cross-wiki graph** — ACP sessions target one wiki. Cross-wiki graph
traversal is a deliberate MCP opt-in (performance). Don't add it to ACP.

**Session sharing between users** — sessions are per-process. Multi-user
scenarios belong in the HTTP MCP transport, not ACP.

**Streaming large diffs** — if ingest returns thousands of warnings, streaming
each line creates noise. Stream a summary, let `lint` handle the details.

## Implementation Priority

| Proposal                  | Value | Complexity | Priority |
| ------------------------- | ----- | ---------- | -------- |
| Real cancellation         | High  | Medium     | 1        |
| Session policy management | High  | Medium     | 2        |
| Proactive watcher push    | High  | Medium     | 3        |

Items 1, 2 are safety/correctness fixes — ship before watcher push.
Item 3 requires new wiring (watcher ↔ sessions).

## Documents to Update

| File                                           | What changes                                         |
| ---------------------------------------------- | ---------------------------------------------------- |
| `specifications/integrations/acp-transport.md` | Session state section, cancel semantics, push model, eviction policy |
| `specifications/model/global-config.md`        | Add `acp_max_sessions`, `acp_session_idle_minutes` under `[serve]`  |
| `implementation/engine.md`                     | `AcpSession` struct fields, eviction background task                |
| `docs/testing/validate-acp.md`                 | Test matrix for new workflows and session eviction                  |
