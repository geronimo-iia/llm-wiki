---
title: "Server"
summary: "llm-wiki serve — transports, multi-wiki, startup, resilience, logging."
read_when:
  - Understanding how llm-wiki serve works
  - Understanding failure isolation and crash recovery
status: ready
last_updated: "2025-07-17"
---

# Server

`llm-wiki serve` starts the engine server. It mounts all registered
wikis at startup and exposes them via MCP tools. stdio is always active.
SSE and ACP are opt-in and can run simultaneously.


## Transports

| Transport | Protocol | Use case                                                |
| --------- | -------- | ------------------------------------------------------- |
| stdio     | MCP      | Claude Code, local agents, batch pipelines — always on  |
| SSE       | MCP      | Remote agents, multi-client access                      |
| ACP       | ACP      | Zed / VS Code agent panel — streaming, session-oriented |

All active transports share the same wiki engine and spaces. A request
on any transport sees the same pages and state.


## CLI

```
llm-wiki serve
          [--sse [:<port>]]         # enable SSE (default port: from config)
          [--acp]                   # enable ACP
```

### Examples

```bash
llm-wiki serve                     # stdio only
llm-wiki serve --sse               # stdio + SSE on default port
llm-wiki serve --sse :9090         # stdio + SSE on port 9090
llm-wiki serve --acp               # stdio + ACP
llm-wiki serve --sse --acp         # all three
```


## Multi-Wiki

All wikis registered in `~/.llm-wiki/config.toml` are mounted at
startup. See [engine-state.md](engine-state.md) for the engine state
layout and [global-config.md](../model/global-config.md) for the space
registry. MCP resources are namespaced by wiki name:

```
wiki://research/concepts/mixture-of-experts
wiki://work/concepts/transformer-scaling
```

Tools accept an optional `wiki` parameter. When omitted, the default
wiki is used.


## Startup Sequence

```
1. Load ~/.llm-wiki/config.toml — spaces + global config
2. Mount all registered wikis
3. Check index staleness for each wiki (warn or auto-rebuild per config)
4. Create shutdown channel (watch + AtomicBool)
5. Start ctrl_c handler (sends shutdown signal)
6. Start heartbeat task (debug level, configurable interval)
7. Start stdio MCP server (always)
8. If --sse: start SSE listener (retry on bind failure)
9. If --acp: start ACP thread (supervision loop)
10. Log: "llm-wiki serve — N wikis mounted [stdio] [sse :8080] [acp]"
```


## Failure Isolation

### Tool handler isolation

A panic or error in a single tool handler does not crash the server.
`std::panic::catch_unwind` around the tool dispatch function.

| Failure            | Behavior                                       |
| ------------------ | ---------------------------------------------- |
| Tool returns error | Error response returned, server continues      |
| Tool panics        | Panic caught, error response, server continues |

### Transport isolation

Each transport runs independently. A failure in one does not affect
others.

| Transport | Isolation                                                              |
| --------- | ---------------------------------------------------------------------- |
| stdio     | Process-level — broken pipe means client disconnected, exit is correct |
| SSE       | Retry loop on bind failure                                             |
| ACP       | Supervision loop with restart                                          |


## Transport Supervision

### ACP supervision

On crash, the ACP transport restarts with exponential backoff:

```
attempt 1: immediate
attempt 2: 1s delay
attempt 3: 2s delay
...
attempt N: min(2^(N-2), 30) seconds delay
```

Exits on: clean shutdown, or max restarts reached.

### SSE retry on bind failure

Retries port binding with exponential backoff (same formula). Once bound
successfully, runs until shutdown.


## Configuration

All `serve.*` and `logging.*` keys are global-only. CLI flags override
config per-invocation. See [global-config.md](../model/global-config.md)
for the full key reference.


## Logging

Structured logging via `tracing` and `tracing-subscriber`. All
operational logs go to stderr. Long-running `llm-wiki serve` also
writes to rotating log files under `~/.llm-wiki/logs/`.

### Two channels

| Concern             | Channel                       | Who reads it        |
| ------------------- | ----------------------------- | ------------------- |
| CLI user output     | `println!` to stdout          | Human at terminal   |
| Operational logging | `tracing::*` to stderr + file | Developer debugging |

stdout is the MCP stdio transport — logging must never write to stdout.

### Log levels

| Level   | Usage                                                      |
| ------- | ---------------------------------------------------------- |
| `error` | Unrecoverable: transport crash, ACP connection lost        |
| `warn`  | Recoverable: git commit failed, index rebuild failed       |
| `info`  | Milestones: server started, SSE listening, session created |
| `debug` | Per-request: tool call ok, search results count, heartbeat |

Default filter: `llm_wiki=info,warn`. Override with `RUST_LOG`:

```bash
RUST_LOG=llm_wiki=debug llm-wiki serve
```

### File rotation

`llm-wiki serve` writes rotating log files. CLI commands log to stderr
only.

| Setting                 | Default            | Description                        |
| ----------------------- | ------------------ | ---------------------------------- |
| `logging.log_path`      | `~/.llm-wiki/logs` | Directory; empty = stderr only     |
| `logging.log_rotation`  | `daily`            | `daily`, `hourly`, `never`         |
| `logging.log_max_files` | `7`                | Max rotated files; `0` = unlimited |
| `logging.log_format`    | `text`             | `text` (human) or `json` (machine) |

When file logging is active, logs go to both stderr and the log file.

All `logging.*` keys are global-only. See
[global-config.md](../model/global-config.md) for the full key
reference.

### What is not logged

- Page content (could be large or sensitive)
- Frontmatter field values
- Config file contents (may contain paths)
- Search queries (debug level only)


## Guarantees

| Guarantee                                 | Scope     |
| ----------------------------------------- | --------- |
| Tool panic does not crash server          | MCP + ACP |
| Transport crash does not affect others    | All       |
| ACP restarts automatically                | ACP       |
| SSE retries port binding                  | SSE       |
| stdio exit on disconnect is intentional   | stdio     |
| Max restart limit prevents infinite loops | ACP       |
| Coordinated shutdown on ctrl_c            | All       |

### Shutdown

On ctrl_c, the engine sends a shutdown signal to all transports:

| Transport | Behavior                                              |
| --------- | ----------------------------------------------------- |
| stdio     | Stops waiting, exits cleanly                          |
| SSE       | Stops accepting connections, exits                    |
| ACP       | Supervision loop checks flag, exits on next iteration |
| Heartbeat | Stops ticking, task exits                             |

"server stopped" is logged before process exit. In-flight requests
are dropped (best-effort, no grace period).

### Not guaranteed

- **Tool timeout** — blocking tool handler is not interrupted
- **State recovery** — ACP sessions are in-memory only, lost on restart
- **In-flight completion** — active requests are dropped on shutdown
