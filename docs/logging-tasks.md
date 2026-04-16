# Logging Tasks

Implement structured logging per logging spec and logging.md analysis.
Phased approach — essentials first, polish later.

Reference:
- [Logging spec](specifications/core/logging.md)
- [Logging analysis](logging.md)

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

**Goal:** Add rotating file logging to `wiki serve` under `~/.wiki/logs/`.

Reference: [Logging spec §4-5](specifications/core/logging.md)

#### Config changes

- `src/config.rs` — add `LoggingConfig` struct:
  ```rust
  pub struct LoggingConfig {
      pub log_path: String,       // default: ~/.wiki/logs
      pub log_rotation: String,   // daily | hourly | never
      pub log_max_files: u32,     // default: 7
      pub log_format: String,     // text | json
  }
  ```
  Add `logging: LoggingConfig` to `GlobalConfig`. Global-only — not in
  `WikiConfig` or `ResolvedConfig`.
- `src/config.rs` — add to `set_global_config_value` match arms:
  `logging.log_path`, `logging.log_rotation`, `logging.log_max_files`,
  `logging.log_format`.
- `src/config.rs` — add `logging.log_path`, `logging.log_rotation`,
  `logging.log_max_files`, `logging.log_format` to the global-only
  rejection list in `set_wiki_config_value` (alongside `serve.*`).
- `src/main.rs` — in `get_config_value`: add the 3 logging keys.
- `src/mcp/tools.rs` — in `get_value`: add the 3 logging keys.

#### Init changes

- `src/init.rs` — in `init()`, after `spaces::register`: create
  `~/.wiki/logs/` directory if it doesn't exist. This is global engine
  infrastructure, created once alongside `~/.wiki/config.toml`.
- Only create on the first `wiki init` (when `~/.wiki/` is new). If
  `~/.wiki/logs/` already exists, skip.

#### Logging changes

- `Cargo.toml` — add `tracing-appender = "0.2"`.
- `src/main.rs` — in `Commands::Serve` branch, before starting the server:
  - Read `logging` config from global config.
  - If `log_path` is non-empty:
    - Create `log_path` directory if it doesn't exist.
    - Build `RollingFileAppender` with the configured rotation.
    - Set `max_log_files` if > 0.
    - Wrap in `non_blocking`.
    - Build a layered subscriber: stderr + file.
    - If `log_format` is `json`: use `.json()` formatter.
    - If `log_format` is `text` (default): use `.compact()` formatter.
    - Hold the `_guard` in `main()` scope.
  - If `log_path` is empty: stderr-only (current behavior).
- CLI commands (non-serve): always stderr-only, ignore `log_path`.

#### Default behavior

- `log_path` defaults to `~/.wiki/logs` — file logging works out of the
  box for `wiki serve`.
- `log_path = ""` explicitly disables file logging.
- `~/.wiki/logs/` is created automatically on first `wiki serve`.

#### Tests

- `tests/config.rs` — new tests:
  - `logging_config_defaults` — assert default values.
  - `set_global_config_value_sets_logging_keys` — set each key, assert.
  - `set_wiki_config_value_rejects_logging_keys` — set `logging.log_path`
    on a WikiConfig, assert error with "global-only" message.
- No file rotation tests (would require time manipulation or waiting).

#### Exit criteria

- `wiki init` creates `~/.wiki/logs/` alongside `~/.wiki/config.toml`.
- `wiki serve` creates `~/.wiki/logs/wiki.log.YYYY-MM-DD`.
- Logs appear in both stderr and the file.
- `wiki config get logging.log_path` returns the path.
- `wiki config set logging.log_path "" --global` disables file logging.
- `wiki config set logging.log_path "/tmp" --wiki research` → error
  (global-only key).
- CLI commands do not create log files.
- `cargo test` passes.

---

### Task L7 — (merged into L6)

JSON log output is now part of Task L6 via the `log_format` config key.

---

## Execution order

| Order | Task | Effort | Dependencies |
|-------|------|--------|-------------|
| 1 | L1 — Init subscriber | Tiny | None |
| 2 | L2 — Replace eprintln! | Small | L1 |
| 3 | L3 — Stop silent discard | Small | L1 |
| 4 | L4 — Tool observability | Small | L1 |
| 5 | L5 — ACP spans | Small | L1 |
| 6 | L6 — File rotation + JSON format | Medium | L1 |

Phase 1 (L1–L5) is done. L6 adds a dependency (`tracing-appender`),
config surface (`[logging]`), and init changes (`~/.wiki/logs/`).

---

## Known issue — init path canonicalization (FIXED)

Discovered during L6 testing. `init()` used `path.canonicalize()` before
creating the directory. On macOS, `tempdir()` returns `/var/folders/...`
which canonicalizes to `/private/var/folders/...` — but only after the
path exists.

The first call stored the non-canonical path (canonicalize failed, fell
back to raw path). The second call canonicalized successfully to a
different string. The re-run comparison `w.path == path.to_string_lossy()`
failed.

**Fix:** create the directory *before* canonicalizing. Both the stored
path and future comparisons now use the same canonical form.
