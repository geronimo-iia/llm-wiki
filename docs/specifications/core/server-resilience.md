---
title: "Server Resilience"
summary: "Failure isolation, transport supervision, and crash recovery guarantees for wiki serve."
read_when:
  - Implementing or extending transport supervision
  - Understanding what happens when a tool handler or transport crashes
  - Debugging server restarts or unexpected exits
  - Adding a new transport to wiki serve
status: draft
last_updated: "2025-07-17"
---

# Server Resilience

`wiki serve` is a long-running process. It must tolerate failures in
individual tool handlers and transports without crashing the entire
process. This document specifies the failure isolation boundaries,
supervision behavior, and recovery guarantees.

---

## 1. Failure Isolation

### Tool handler isolation

A panic or error in a single MCP/ACP tool handler must not crash the
server. The isolation boundary is `std::panic::catch_unwind` around the
tool dispatch function.

| Failure | Behavior |
|---------|----------|
| Tool handler returns `Err` | Error response returned to client, server continues |
| Tool handler panics | Panic caught, error response returned, server continues |
| Tool handler blocks indefinitely | Not handled (future: timeout) |

The catch_unwind boundary is at `tools::call()` for MCP and at the
workflow dispatch in `acp::prompt()` for ACP.

### Transport isolation

Each transport runs independently. A failure in one transport must not
affect others:

| Transport | Runtime | Isolation |
|-----------|---------|-----------|
| stdio MCP | Main tokio runtime | Process-level (if stdio dies, the process exits — this is correct, the client disconnected) |
| SSE MCP | Main tokio runtime | Retry loop on bind failure |
| ACP | Dedicated OS thread | Supervision loop with restart |

stdio is special: it's the primary transport, and a broken pipe means
the client (Claude Code, IDE) disconnected. Exiting is the correct
behavior.

---

## 2. Transport Supervision

### ACP supervision loop

The ACP thread runs a supervision loop. On crash, the transport is
restarted with exponential backoff:

```
attempt 1: immediate
attempt 2: 1s delay
attempt 3: 2s delay
attempt 4: 4s delay
...
attempt N: min(2^(N-2), 30) seconds delay
```

The loop exits on:
- Clean shutdown (`Ok(())`) — normal exit, no restart
- Max restarts reached — gives up, logs error

Defaults from `[serve]` config:
- `max_restarts = 10` — max restart attempts. `0` disables restart
  entirely (crash exits the process, matching pre-resilience behavior).
- `restart_backoff = 1` — initial backoff in seconds. Doubles on each
  restart, capped at 30 seconds.

Backoff cap: 30 seconds (hardcoded, not configurable — the initial
backoff is the tuning knob).

On each restart, the entire tokio runtime and `WikiAgent` are rebuilt.
This is required because the agent is `!Send` (ACP SDK constraint) and
cannot be moved across runtimes.

### SSE retry on bind failure

The SSE server retries port binding with exponential backoff:

```
attempt 1: immediate
attempt 2: 1s delay
attempt 3: 2s delay
...
attempt 5: give up with error
```

Max attempts: from `serve.max_restarts` (default 10). When
`max_restarts = 0`, a single bind failure exits immediately. Uses
`serve.restart_backoff` for initial delay, same 30s cap.

This handles the common case of a previous process still holding the
port during development.

Once bound successfully, the SSE server runs until `ctrl_c`. A runtime
error after successful bind is not retried (the server exits).

---

## 3. Observability

### Crash logging

Every transport crash and restart is logged at `error` level with
structured fields:

```
ERROR transport="acp" error="connection reset" restart=3 "transport crashed"
```

Every caught panic is logged at `error` level:

```
ERROR tool="wiki_search" "tool handler panicked"
```

### Heartbeat

The server emits a periodic heartbeat at `debug` level (every 60
seconds). This enables silence-based monitoring — if the heartbeat
stops, the server is hung.

Visible with `RUST_LOG=llm_wiki=debug`. Silent at the default `info`
level.

### Health endpoint

When SSE is enabled, a `/health` HTTP route returns `200 OK`. This
enables HTTP-based health checks from external monitors (load balancers,
container orchestrators).

Note: depends on the rmcp `SseServer` API exposing the axum router.
If not available, this is deferred.

---

## 4. Guarantees

| Guarantee | Scope |
|-----------|-------|
| A tool panic does not crash the server | MCP + ACP |
| A transport crash does not affect other transports | All |
| ACP restarts automatically after crash | ACP |
| SSE retries port binding | SSE |
| stdio exit on disconnect is intentional | stdio |
| Max restart limit prevents infinite loops | ACP |
| All crashes are logged with structured context | All |

### What is NOT guaranteed

- **Tool timeout** — a tool handler that blocks indefinitely is not
  interrupted. Future improvement: per-tool timeout.
- **State recovery** — ACP sessions are lost on restart (in-memory only).
  The wiki state (git, index) is unaffected.
- **Graceful shutdown** — `ctrl_c` stops the server, but in-flight
  requests may be dropped. Future improvement: drain period.

---

## 5. Startup Sequence (updated)

```
1. Load ~/.wiki/config.toml
2. Mount all registered wikis
3. Check index staleness (warn or auto-rebuild)
4. Start heartbeat task (debug level, 60s interval)
5. Start stdio MCP server (always)
6. If --sse: start SSE listener (retry on bind failure)
7. If --acp: start ACP thread (supervision loop)
8. Log: "wiki serve — N wikis mounted [stdio] [sse :8080] [acp]"
```
