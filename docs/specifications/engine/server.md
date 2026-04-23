---
title: "Server"
summary: "llm-wiki serve — transports, multi-wiki, startup, resilience, logging."
read_when:
  - Understanding how llm-wiki serve works
  - Understanding failure isolation and crash recovery
status: ready
last_updated: "2025-07-21"
---

# Server

`llm-wiki serve` starts the engine server. It mounts all registered
wikis at startup and exposes them via MCP tools. Wikis can be added
or removed at runtime via space management tools. stdio is always
active. SSE and ACP are opt-in and can run simultaneously.


## Transports

| Transport | Protocol | Use case                                                |
| --------- | -------- | ------------------------------------------------------- |
| stdio     | MCP      | Claude Code, local agents, batch pipelines — always on  |
| HTTP      | MCP      | Remote agents, multi-client access (Streamable HTTP)    |
| ACP       | ACP      | Zed / VS Code agent panel — streaming, session-oriented |

All active transports share the same wiki engine and spaces. A request
on any transport sees the same pages and state.


## CLI

```
llm-wiki serve
          [--http [:<port>]]         # enable HTTP (default port: from config)
          [--acp]                   # enable ACP
          [--watch]                 # enable filesystem watcher
```

### Examples

```bash
llm-wiki serve                     # stdio only
llm-wiki serve --http              # stdio + HTTP on default port
llm-wiki serve --http :9090        # stdio + HTTP on port 9090
llm-wiki serve --acp               # stdio + ACP
llm-wiki serve --http --acp        # all three
llm-wiki serve --watch             # stdio + filesystem watcher
llm-wiki serve --http --watch      # stdio + HTTP + watcher
```


## Multi-Wiki

All wikis registered in `~/.llm-wiki/config.toml` are mounted at
startup. Wikis can be added or removed at runtime — see
[Hot Reload](#hot-reload). See [engine-state.md](engine-state.md)
for the engine state layout and
[global-config.md](../model/global-config.md) for the space registry.
MCP resources are namespaced by wiki name:

```
wiki://research/concepts/mixture-of-experts
wiki://work/concepts/transformer-scaling
```

Tools accept an optional `wiki` parameter. When omitted, the default
wiki is used.


## Startup Sequence

```
1. Load ~/.llm-wiki/config.toml — spaces + global config
2. Create wiki map: RwLock<HashMap<String, Arc<WikiHandle>>>
3. Mount all registered wikis into the map
4. Check index staleness for each wiki (warn or auto-rebuild per config)
5. Create shutdown channel (watch + AtomicBool)
6. Start ctrl_c handler (sends shutdown signal)
7. Start heartbeat task (debug level, configurable interval)
8. Start stdio MCP server (always)
9. If --http: start HTTP listener (retry on bind failure)
10. If --acp: start ACP task
11. If --watch: start filesystem watcher task
12. Log: "llm-wiki serve — N wikis mounted [stdio] [http :8080] [acp] [watch]"
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
| HTTP      | Retry loop on bind failure                                             |
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

### HTTP retry on bind failure

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
| `info`  | Milestones: server started, HTTP listening, session created |
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
| HTTP retries port binding                  | HTTP      |
| stdio exit on disconnect is intentional   | stdio     |
| Max restart limit prevents infinite loops | ACP       |
| Coordinated shutdown on ctrl_c            | All       |
| Hot reload does not interrupt transports  | All       |

### Shutdown

On ctrl_c, the engine sends a shutdown signal to all transports:

| Transport | Behavior                                              |
| --------- | ----------------------------------------------------- |
| stdio     | Stops waiting, exits cleanly                          |
| HTTP      | Stops accepting connections, exits                    |
| ACP       | Supervision loop checks flag, exits on next iteration |
| Heartbeat | Stops ticking, task exits                             |

"server stopped" is logged before process exit. In-flight requests
are dropped (best-effort, no grace period).

### Not guaranteed

- **Tool timeout** — blocking tool handler is not interrupted
- **State recovery** — ACP sessions are in-memory only, lost on restart
- **In-flight completion** — active requests are dropped on shutdown


## Hot Reload

Space management tools update the wiki map at runtime. No server
restart needed.

| Tool | Runtime effect |
|------|----------------|
| `wiki_spaces_create` | Mounts the new wiki immediately |
| `wiki_spaces_remove` | Unmounts the wiki immediately |
| `wiki_spaces_set_default` | Updates the default immediately |

### Shared state

The engine holds mounted wikis in a
`RwLock<HashMap<String, Arc<WikiHandle>>>`. Read paths (search, list,
read, graph) take a read lock, clone the `Arc<WikiHandle>`, release
the lock, then operate on the handle. Mount/unmount takes a write
lock.

### Mount

On `wiki_spaces_create`:

1. Write `config.toml` (register the space)
2. Open or create the tantivy index at `~/.llm-wiki/indexes/<name>/`
3. Run staleness check (same rules as startup)
4. Insert into wiki map under write lock
5. Emit `notifications/resources/list_changed` (MCP notification)
6. Log: `reload: mounted <name>`

### Unmount

On `wiki_spaces_remove`:

1. Refuse if the wiki is the current default (same rule as the CLI)
2. Remove from wiki map under write lock
3. Close index reader/writer handles (do not delete index files)
4. Write `config.toml` (unregister the space)
5. If `--delete`: also delete index files at
   `~/.llm-wiki/indexes/<name>/`
6. Emit `notifications/resources/list_changed`
7. Log: `reload: unmounted <name>`

In-flight requests that already hold an `Arc<WikiHandle>` complete
normally — the handle stays alive until the last reference is dropped.

### MCP notification

After mount, unmount, or set-default, the engine emits
`notifications/resources/list_changed` on transports that support
notifications. Agents can re-bootstrap if they care. Transports that
don't support notifications (stdio batch) skip silently.
