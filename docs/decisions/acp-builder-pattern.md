# Decision: ACP Builder Pattern

## Problem

`agent-client-protocol` 0.11 removed the `Agent` trait. The old
`WikiAgent` struct with `#[async_trait(?Send)] impl Agent` no longer
compiles. The `!Send` constraint forced a dedicated OS thread with
`LocalSet` + `spawn_local` + `mpsc` channel bridge for notifications.

## Decision

Adopt the 0.11 builder pattern. No `WikiAgent` struct — shared state
(`Arc<WikiEngine>`, `Arc<Mutex<Sessions>>`) captured directly into
handler closures.

```
Agent.builder()
    .on_receive_request(async |req, responder, cx| { ... })
    .connect_to(ByteStreams::new(stdout, stdin))
```

Split into `src/acp/` module: `mod.rs`, `helpers.rs`, `research.rs`,
`server.rs`.

## Why not the Proxy pattern

`agent-client-protocol-rmcp` wraps our rmcp `ServerHandler` as an
ACP MCP server — but that's a Proxy pattern requiring a conductor.
We're an Agent (handles prompts directly). The Proxy pattern would
change the architecture for no gain at this stage.

## What was removed

- `WikiAgent` struct
- `Agent` trait impl
- `mpsc::unbounded_channel` + `oneshot` notification bridge
- `LocalSet` + `spawn_local`
- Dedicated OS thread in `server.rs`
- `async-trait` crate dependency
- `AtomicBool` shutdown flag (only `watch` channel remains)
