# Phase 7 — ACP Transport

## Context

Phases 1–6 are complete. The MCP server is fully working. You are now
implementing the ACP transport so llm-wiki works as a native Zed / VS Code
streaming agent.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every struct, method name, and signature must match the spec exactly.
- Do not add ACP methods or session fields not described in the spec.
- Do not modify any file under `docs/`.
- Do not modify Phase 1–6 modules unless fixing a compilation error.

## Specs to read before starting

Read this file in full before writing any code:

- `docs/specifications/integrations/acp-transport.md`

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `Cargo.toml`

Add:
```toml
agent-client-protocol       = "0.10"
agent-client-protocol-tokio = "0.1"
```

### 2. `src/acp.rs`

Replace the Phase 6 stub with the full implementation:

- `AcpSession { id, label, wiki, created_at, active_run }`
- `WikiAgent { spaces, sessions }`
- Implement `Agent` trait:
  - `initialize` — inject `src/instructions.md` as system context
  - `new_session`
  - `load_session`
  - `list_sessions`
  - `prompt` — workflow dispatch by keyword matching:
    - ingest/add/path-like → ingest workflow
    - search/find/what do you know → research workflow
    - lint/orphans/stubs → lint workflow
    - crystallize/distil/capture → crystallize workflow
    - default → research workflow
  - `cancel`
- `serve_acp(spaces) -> Result<()>` using `serve_agent_stdio`

Each workflow streams `tool_call` and `message` events as it runs.
`acp.rs` calls the same engine functions as `mcp.rs` — no logic
duplication.

### 3. `src/server.rs`

Start `serve_acp` alongside MCP when `--acp` flag is set.

## Exit criteria

Before marking Phase 7 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki serve --acp` starts without error
- [ ] Zed agent panel connects and lists sessions
- [ ] Ingest workflow streams `tool_call` events visibly
- [ ] Research workflow streams answer as `message` events
- [ ] `cancel` stops an active run
