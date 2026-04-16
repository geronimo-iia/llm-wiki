# Logging — Production Readiness

## Current State

`tracing` and `tracing-subscriber` (with `env-filter`) are declared in
`Cargo.toml` but **never used**. Zero `tracing::` calls in the codebase.

All logging is ad-hoc `eprintln!`:

| File | Count | Purpose |
|------|-------|---------|
| `src/server.rs` | 4 | SSE listening, index rebuild warnings, startup banner |
| `src/acp.rs` | 1 | ACP notification error |
| `src/main.rs` | many | CLI user-facing output (`println!` — correct, not logging) |
| `src/mcp/tools.rs` | 0 | Errors returned as `Result` or silently discarded |

### Silent error discard

Several tool handlers swallow failures with no trace:

```rust
let _ = git::commit(...);           // commit failure invisible
let _ = search::rebuild_index(...); // index rebuild failure invisible
let _ = lint::write_lint_md(...);   // lint write failure invisible
```

---

## Improvements

### 1. Initialize tracing-subscriber

Wire up the existing dependency in `main.rs`:

```rust
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_wiki=info,warn".into()),
        )
        .with_writer(std::io::stderr)  // stdout is MCP stdio transport
        .init();
    // ...
}
```

Gives `RUST_LOG=llm_wiki=debug wiki serve` for free.

### 2. Replace eprintln! with structured tracing

| Current | Replacement |
|---------|-------------|
| `eprintln!("SSE server listening on {addr}")` | `tracing::info!(addr = %addr, "SSE server listening")` |
| `eprintln!("warning: failed to rebuild index for {}: {e}", ...)` | `tracing::warn!(wiki = %name, error = %e, "index rebuild failed")` |
| `eprintln!("wiki serve — {wiki_count} wikis mounted [{}]", ...)` | `tracing::info!(wikis = wiki_count, transports = %t, "server started")` |
| `eprintln!("ACP notification error: {e}")` | `tracing::error!(error = %e, "ACP notification failed")` |

### 3. Add tracing to tool handlers

MCP tool dispatch in `mcp/tools.rs` has zero observability. Add spans and
events:

```rust
pub fn call(server: &WikiServer, name: &str, args: &Map<String, Value>) -> ToolResult {
    let _span = tracing::info_span!("tool_call", tool = name).entered();
    let result = match name { ... };
    match &result {
        Err(msg) => tracing::warn!(tool = name, error = %msg, "tool call failed"),
        Ok(_) => tracing::debug!(tool = name, "tool call succeeded"),
    }
    // ...
}
```

Stop silently discarding errors:

```rust
// Before:
let _ = git::commit(&repo_root, &format!("new: {uri}"));

// After:
if let Err(e) = git::commit(&repo_root, &format!("new: {uri}")) {
    tracing::warn!(error = %e, "git commit failed");
}
```

### 4. Separate concerns: CLI output vs operational logging

Formalize the rule:

- `println!` — only in `main.rs` for CLI user-facing output
- `tracing::*` — everywhere else for operational logging
- Never `eprintln!` in library code

### 5. Request-level context for MCP/ACP

Add spans with request identity so logs can be correlated across a single
tool call chain:

```rust
// MCP
fn call_tool(&self, request: CallToolRequestParam, ...) -> ... {
    let _span = tracing::info_span!("mcp_call_tool", tool = %request.name).entered();
    // ...
}

// ACP — session ID is already available
async fn prompt(&self, req: acp::PromptRequest) -> ... {
    let _span = tracing::info_span!(
        "acp_prompt",
        session = %req.session_id,
        workflow = %workflow,
    ).entered();
    // ...
}
```

### 6. File rotation for long-running serve

For `wiki serve` running as a daemon, add file-based logging with rotation
via `tracing-appender`:

```toml
# Cargo.toml
tracing-appender = "0.2"
```

```rust
let file_appender = tracing_appender::rolling::daily("/var/log/llm-wiki", "wiki.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
tracing_subscriber::fmt()
    .with_writer(non_blocking)
    .init();
```

For CLI commands (init, search, lint, etc.), stderr-only is sufficient.

### 7. Structured JSON output

Add a `--log-format json` flag or config option for machine-parseable logs:

```rust
if json_logs {
    tracing_subscriber::fmt().json().with_writer(std::io::stderr).init();
} else {
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
}
```

---

## Summary

| Area | Current | Target |
|------|---------|--------|
| Subscriber init | Not initialized | `tracing_subscriber::fmt()` with `env-filter` in main |
| Library logging | `eprintln!` (5 calls) | `tracing::info/warn/error` with structured fields |
| Tool observability | Zero logging | Spans per tool call, warn on failures |
| Silent error discard | `let _ =` on git/index/lint | `if let Err` + `tracing::warn` |
| Request correlation | None | Span with tool name / session ID |
| Log levels | N/A | `RUST_LOG` env filter (already a dep) |
| File rotation | None | `tracing-appender` for serve mode |
| JSON output | None | `--log-format json` option |
| stdout safety | Mostly clean | Formalize: `println!` only in main.rs |
