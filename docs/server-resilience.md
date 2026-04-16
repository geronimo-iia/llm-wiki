# Server Resilience — Thread Restart & Crash Handling

## Current State

### Transport architecture

`wiki serve` runs up to three transports concurrently:

| Transport | Runtime | Lifecycle |
|-----------|---------|-----------|
| stdio MCP | Main tokio runtime | `service.waiting().await` — blocks until disconnect |
| SSE MCP | Main tokio runtime | `SseServer::serve` + `ctrl_c().await` |
| ACP | Dedicated OS thread + `new_current_thread` tokio runtime | `std::thread::spawn` → `rt.block_on(serve_acp)` |

All transports share the same `WikiServer` state via `Arc`.

### What happens on crash

| Failure | Behavior | Recovery |
|---------|----------|----------|
| ACP thread panic | `join()` returns `Err` → process exits | None |
| MCP stdio/SSE error | `?` propagates → process exits | None |
| Tool handler panic | Undefined — may abort the tokio task or the runtime | None |
| SSE bind failure | Process exits immediately | None |

There is no supervision loop, no restart logic, no `catch_unwind` boundary.
A single transport failure is terminal for the entire process.

Reference: `src/server.rs` — the `serve` function, `serve_stdio`, `serve_sse`.

---

## Improvements

### 1. ACP thread supervision loop

Wrap the ACP thread in a respawn loop with exponential backoff:

```rust
let acp_handle = std::thread::spawn(move || {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);
    loop {
        let global = global_arc.clone();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build ACP runtime");
        match rt.block_on(crate::acp::serve_acp(global)) {
            Ok(()) => break,  // clean shutdown
            Err(e) => {
                eprintln!("ACP crashed: {e} — restarting in {backoff:?}");
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
});
```

Add a max-restart counter to prevent infinite crash loops.

### 2. catch_unwind boundary for tool handlers

Wrap MCP tool dispatch in `std::panic::catch_unwind` so a panicking tool
does not take down the server:

```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    // tool execution
}));
match result {
    Ok(r) => r,
    Err(_) => CallToolResult::error("internal error: tool panicked"),
}
```

This isolates tool-level failures from the transport layer.

### 3. SSE transport retry

The SSE server exits on bind failure. Add a retry loop:

```rust
pub async fn serve_sse(server: WikiServer, port: u16) -> Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let mut backoff = Duration::from_secs(1);
    loop {
        match SseServer::serve(addr).await {
            Ok(sse_server) => {
                backoff = Duration::from_secs(1);
                let _ct = sse_server.with_service(move || server.clone());
                tokio::signal::ctrl_c().await?;
                return Ok(());
            }
            Err(e) => {
                eprintln!("SSE bind failed: {e} — retrying in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
            }
        }
    }
}
```

### 4. Health check / liveness signal

External process managers (systemd, launchd) need a way to detect a hung server.

- **SSE**: add a `/health` HTTP route returning `200 OK`
- **stdio**: MCP already supports `ping` via the protocol
- **Heartbeat**: log a periodic heartbeat to stderr so silence-based monitors can detect hangs

### 5. Structured logging for crash forensics

Replace `eprintln!` with `tracing` for structured, filterable output:

```rust
tracing::error!(transport = "acp", error = %e, "transport crashed, restarting");
```

This enables log aggregation and post-mortem analysis.

---

## Summary

| Area | Current | Target |
|------|---------|--------|
| ACP thread panic | Process exits | Supervision loop with exponential backoff |
| Tool handler panic | Undefined | `catch_unwind` boundary per tool call |
| SSE bind failure | Process exits | Retry loop with backoff |
| Health monitoring | None | `/health` endpoint + heartbeat logging |
| Crash diagnostics | `eprintln!` | Structured tracing |
| Max restart limit | N/A | Cap restarts to prevent infinite loops |
