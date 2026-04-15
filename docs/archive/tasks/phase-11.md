# Phase 11 — ACP Transport

Goal: `wiki serve --acp` works as a native Zed / VS Code agent. Sessions are
streaming, multi-turn, with `src/instructions.md` injected at `initialize`.
Uses the official `agent-client-protocol` Rust SDK — no hand-rolled transport needed.

Depends on: Phase 10 complete (`src/instructions.md` is rewritten in Phase 10
and must be stable before it is injected at ACP session start).
Design ref: [../design/acp-transport.md](../design/acp-transport.md).

---

## `acp.rs` — new module

### Dependencies

- [ ] Add to `Cargo.toml`:
  ```toml
  agent-client-protocol       = "0.10"
  agent-client-protocol-tokio = "0.1"
  ```

### `WikiAgent` struct

- [ ] `WikiAgent { wiki_root: PathBuf, wiki_name: String, sessions: Mutex<HashMap<String, AcpSession>> }`
- [ ] `AcpSession { id, label, wiki, created_at, active_run: Option<AbortHandle> }`
- [ ] `WorkflowKind` enum: `Ingest`, `Research`, `Lint`, `Enrichment`

### `Agent` trait implementation

- [ ] `initialize` → return `InitializeResponse` with `system: include_str!("instructions.md")`
- [ ] `new_session` → create `AcpSession`, store in map, return `NewSessionResponse`
- [ ] `load_session` → resolve by id or label, error if not found
- [ ] `list_sessions` → return all sessions as `ListSessionsResponse`
- [ ] `prompt` → dispatch to workflow, stream events via `PromptResponse` sender,
  return when workflow completes
- [ ] `cancel` → abort active run via `AbortHandle`, no response needed
- [ ] `authenticate` → return `AuthenticateResponse` (no auth required for local use)

### Workflow dispatch

- [ ] `dispatch_workflow(prompt_text: &str, meta: Option<&serde_json::Value>) -> WorkflowKind`
  — check `meta["workflow"]` string first (explicit override)
  — keyword heuristic fallback: ingest | research | lint | enrichment
  — default: `Research`
- [ ] `run_ingest_workflow` — calls `ingest::ingest`, streams tool events
- [ ] `run_research_workflow` — calls `context::context` + `search::search`
- [ ] `run_lint_workflow` — calls `lint::lint`
- [ ] `run_enrichment_workflow` — calls `context::context` + `ingest::ingest`

### Entry point

- [ ] `serve_acp(wiki_root: &Path, wiki_name: &str) -> Result<()>`
  — construct `WikiAgent`, call `agent_client_protocol_tokio::serve_agent_stdio(agent)`

---

## `cli.rs`

- [ ] Add `--acp` flag to `wiki serve` subcommand
- [ ] `wiki serve --acp` → call `acp::serve_acp(wiki_root, wiki_name)`
- [ ] `wiki serve --acp --wiki <name>` → resolve wiki from registry, pass to `serve_acp`
- [ ] Mutually exclusive: `--acp` and `--sse` cannot be combined

---

## `server.rs`

No changes. MCP and ACP are independent transports.

---

## Tests

**Test file:** `tests/acp.rs`

### Unit tests

- [ ] `dispatch_workflow` — `meta["workflow"] = "ingest"` → `WorkflowKind::Ingest`
- [ ] `dispatch_workflow` — prompt "ingest this folder" → `WorkflowKind::Ingest`
- [ ] `dispatch_workflow` — prompt "what do you know about MoE" → `WorkflowKind::Research`
- [ ] `dispatch_workflow` — unknown prompt → `WorkflowKind::Research` (default)
- [ ] `WikiAgent::initialize` — response `system` field contains `instructions.md` content
- [ ] `WikiAgent::new_session` — session stored, response contains session id
- [ ] `WikiAgent::load_session` — known id → success
- [ ] `WikiAgent::load_session` — unknown id → ACP error response
- [ ] `WikiAgent::list_sessions` — returns all created sessions
- [ ] `WikiAgent::cancel` — active run aborted

### Integration tests

- [ ] Full `initialize` → `newSession` → `prompt` → `done` sequence over
  in-process pipe (no real stdio)
- [ ] `prompt` triggers `ToolCall` events before `Done`
- [ ] `cancel` during active run → `Done { stop_reason: "cancel" }` before
  workflow completes
- [ ] `wiki serve --acp` starts without error, reads from stdin, writes to stdout
- [ ] EOF on stdin → clean exit (no panic, exit code 0)

### Manual tests (document results)

- [ ] Configure Zed with `wiki serve --acp` as agent server
- [ ] Open Zed agent panel, select llm-wiki agent
- [ ] Type "what do you know about MoE?" → research workflow streams results
- [ ] Type "ingest agent-skills/semantic-commit/" → ingest workflow streams steps
- [ ] Cancel mid-workflow → `done { stop_reason: "cancel" }` received

---

## Changelog

- [ ] `CHANGELOG.md` — Phase 11: `wiki serve --acp`, ACP transport, Zed integration,
  session model, workflow dispatch, streaming tool_call events

## README

- [ ] CLI reference — add `wiki serve --acp` entry
- [ ] **IDE integration** section:
  - Zed: `~/.config/zed/settings.json` snippet
  - VS Code: agent extension config snippet
  - Note: ACP is for interactive use; MCP stdio is for agent pipelines
- [ ] Note: no external dependency — ACP implemented directly in `acp.rs`

## Dev documentation

- [ ] `docs/dev/acp.md` — `WikiAgent` struct, `Agent` trait methods, workflow
  dispatch, session lifecycle, how to add a new workflow kind
- [ ] `docs/dev/acp.md` — SDK crates used: `agent-client-protocol` +
  `agent-client-protocol-tokio`; why the official SDK over hand-rolled
- [ ] Update `docs/dev/architecture.md` — add `acp.rs` to module map,
  mark Phase 10 complete
