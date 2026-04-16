# Logging Tasks

Implement structured logging per logging.md. Phased approach — essentials
first, polish later.

Reference: [logging.md](logging.md)

Dependencies already in Cargo.toml: `tracing = "0.1"`, `tracing-subscriber`
with `env-filter`. No new crates needed for Phase 1.

---

## Phase 1 — Essentials

### Task L1 — Initialize tracing subscriber

**Goal:** Wire up the existing `tracing-subscriber` dependency so
`RUST_LOG` works.

#### Code changes

- `src/main.rs` — at the top of `main()`, before CLI parsing:
  ```rust
  tracing_subscriber::fmt()
      .with_env_filter(
          tracing_subscriber::EnvFilter::try_from_default_env()
              .unwrap_or_else(|_| "llm_wiki=info,warn".into()),
      )
      .with_writer(std::io::stderr)
      .init();
  ```
  stdout is the MCP stdio transport — all logs must go to stderr.

#### Tests

- No new tests. Verify manually: `RUST_LOG=llm_wiki=debug cargo run -- serve --dry-run`
  produces structured output on stderr.

#### Exit criteria

- `RUST_LOG=llm_wiki=debug` produces tracing output on stderr.
- `RUST_LOG` unset defaults to `info` + `warn`.
- stdout remains clean (MCP transport unaffected).
- `cargo test` passes.

---

### Task L2 — Replace eprintln! in library code

**Goal:** Replace all `eprintln!` in library code with structured tracing
calls. Keep `println!` in `main.rs` for CLI user-facing output.

#### Inventory

| File | Line | Current | Replacement |
|------|------|---------|-------------|
| `src/server.rs:118` | `eprintln!("SSE server listening on {addr}")` | `tracing::info!(%addr, "SSE server listening")` |
| `src/server.rs:154` | `eprintln!("warning: failed to rebuild index for {}: {e}", ...)` | `tracing::warn!(wiki = %entry.name, error = %e, "index rebuild failed")` |
| `src/server.rs:157` | `eprintln!("warning: index for \"{}\" is stale ...")` | `tracing::warn!(wiki = %entry.name, "index stale")` |
| `src/server.rs:173` | `eprintln!("wiki serve — {wiki_count} wikis mounted [{}]", ...)` | `tracing::info!(wikis = wiki_count, transports = %t, "server started")` |
| `src/acp.rs:333` | `eprintln!("ACP notification error: {e}")` | `tracing::error!(error = %e, "ACP notification failed")` |

`main.rs` `eprintln!` calls are CLI user-facing warnings (stale index,
rebuild failure, unknown workflow). These stay as `eprintln!` — they are
user output, not operational logs.

#### Code changes

- `src/server.rs` — replace 4 `eprintln!` with `tracing::info!` / `tracing::warn!`
- `src/acp.rs` — replace 1 `eprintln!` with `tracing::error!`

#### Tests

- No new tests. Existing tests pass (logging is additive).

#### Exit criteria

- Zero `eprintln!` in `src/server.rs` and `src/acp.rs`.
- `eprintln!` remains only in `src/main.rs` (CLI user output).
- `cargo test` passes.

---

### Task L3 — Stop silent error discard

**Goal:** Replace all `let _ =` on fallible operations with
`if let Err` + `tracing::warn!`.

#### Inventory

| File | Line | Operation |
|------|------|-----------|
| `src/mcp/tools.rs:478` | `let _ = git::commit(...)` | new_page commit |
| `src/mcp/tools.rs:489` | `let _ = git::commit(...)` | new_section commit |
| `src/mcp/tools.rs:525` | `let _ = search::rebuild_index(...)` | search staleness rebuild |
| `src/mcp/tools.rs:582` | `let _ = search::rebuild_index(...)` | list staleness rebuild |
| `src/mcp/tools.rs:630` | `let _ = lint::write_lint_md(...)` | lint report write |
| `src/mcp/tools.rs:638` | `let _ = git::commit(...)` | lint commit |
| `src/mcp/tools.rs:675` | `let _ = std::fs::write(...)` | graph output write |
| `src/mcp/mod.rs:154` | `let _ = peer.notify_resource_updated(...)` | resource notification |

#### Code changes

- `src/mcp/tools.rs` — replace each `let _ =` with:
  ```rust
  if let Err(e) = git::commit(...) {
      tracing::warn!(error = %e, "git commit failed");
  }
  ```
  Same pattern for all 7 sites.
- `src/mcp/mod.rs` — replace the resource notification discard:
  ```rust
  if let Err(e) = peer.notify_resource_updated(...).await {
      tracing::warn!(error = %e, uri = %uri, "resource notification failed");
  }
  ```

#### Tests

- No new tests. Existing tests pass.

#### Exit criteria

- Zero `let _ =` on fallible operations in `src/mcp/`.
- Every discarded error now produces a `tracing::warn!`.
- `cargo test` passes.

---

### Task L4 — Tool call observability

**Goal:** Add tracing spans and events to MCP tool dispatch.

#### Code changes

- `src/mcp/tools.rs` — in `call()`:
  ```rust
  pub fn call(server: &WikiServer, name: &str, args: &Map<String, Value>) -> ToolResult {
      let _span = tracing::info_span!("tool_call", tool = name).entered();
      let result = match name { ... };
      match &result {
          Err(msg) => tracing::warn!(tool = name, error = %msg, "tool call failed"),
          Ok(_) => tracing::debug!(tool = name, "tool call ok"),
      }
      // ...
  }
  ```

- `src/mcp/mod.rs` — in `call_tool()`:
  ```rust
  fn call_tool(&self, request: CallToolRequestParam, ...) -> ... {
      let _span = tracing::info_span!("mcp_call_tool", tool = %request.name).entered();
      // ...
  }
  ```

#### Tests

- No new tests. Verify with `RUST_LOG=llm_wiki=debug`.

#### Exit criteria

- Every MCP tool call produces a tracing span.
- Failed tool calls produce a `warn` event.
- Successful tool calls produce a `debug` event.
- `cargo test` passes.

---

## Phase 2 — Polish (defer)

### Task L5 — Request-level spans for ACP

Add session-scoped spans to ACP `prompt()`:

```rust
let _span = tracing::info_span!(
    "acp_prompt",
    session = %req.session_id,
    workflow = %workflow,
).entered();
```

Depends on: Task L1.

---

### Task L6 — File rotation for serve mode

Add `tracing-appender` for long-running `wiki serve` processes.

New dependency: `tracing-appender = "0.2"`.

Config: `serve.log_path` (optional, default: stderr only).

Depends on: Task L1.

---

### Task L7 — JSON log output

Add `--log-format json` flag or `serve.log_format` config for
machine-parseable logs.

Depends on: Task L1.

---

## Execution order

| Order | Task | Effort | Dependencies |
|-------|------|--------|-------------|
| 1 | L1 — Init subscriber | Tiny | None |
| 2 | L2 — Replace eprintln! | Small | L1 |
| 3 | L3 — Stop silent discard | Small | L1 |
| 4 | L4 — Tool observability | Small | L1 |
| — | L5 — ACP spans | Small | L1 (defer) |
| — | L6 — File rotation | Medium | L1 (defer) |
| — | L7 — JSON output | Small | L1 (defer) |

Phase 1 (L1–L4) can be done in a single commit. No new dependencies,
no behavior changes, no new config surface. Pure additive.
