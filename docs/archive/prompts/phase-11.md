# Phase 11 — ACP Transport

You are implementing Phase 11 of llm-wiki. Phase 10 (context retrieval +
instruct update) is complete. `src/instructions.md` is stable.

## Context

The full task list is in `docs/tasks/phase-11.md`.
Design ref: `docs/design/acp-transport.md`.

## The Goal

`wiki serve --acp` starts llm-wiki as a native ACP agent — the protocol used
by Zed's agent panel and VS Code agent extensions. Sessions are streaming and
multi-turn. `src/instructions.md` is injected as system context at session start.

## Dependencies to add

```toml
agent-client-protocol       = "0.10"
agent-client-protocol-tokio = "0.1"
```

The official Rust SDK from https://github.com/agentclientprotocol/rust-sdk.
No hand-rolled NDJSON transport needed.

## What to implement

### `src/acp.rs` — new module

Implement the `Agent` trait from `agent-client-protocol` on a `WikiAgent` struct.

```rust
pub struct WikiAgent {
    wiki_root: PathBuf,
    wiki_name: String,
    sessions:  Mutex<HashMap<String, AcpSession>>,
}

pub struct AcpSession {
    id:         String,
    label:      Option<String>,
    created_at: u64,
    active_run: Option<AbortHandle>,
}

pub enum WorkflowKind { Ingest, Research, Lint, Enrichment }
```

**`Agent` trait methods:**

- `initialize` → return `InitializeResponse` with
  `system: include_str!("instructions.md")`
- `new_session` → create `AcpSession`, store, return `NewSessionResponse`
- `load_session` → resolve by id or label, ACP error if not found
- `list_sessions` → return all sessions
- `prompt` → dispatch to workflow, stream events, return when done
- `cancel` → abort active run via `AbortHandle`
- `authenticate` → return success (no auth for local use)

**Workflow dispatch:**

```rust
fn dispatch_workflow(prompt_text: &str, meta: Option<&serde_json::Value>) -> WorkflowKind
```

Check `meta["workflow"]` string first (explicit override). Keyword heuristic
fallback: "ingest"/"add"/"folder" → Ingest, "lint"/"orphan" → Lint,
"enrich"/"analyze" → Enrichment, default → Research.

Each workflow calls the existing engine functions:
- `run_research_workflow` → `context::context` + `search::search`
- `run_ingest_workflow` → `ingest::ingest`
- `run_lint_workflow` → `lint::lint`
- `run_enrichment_workflow` → `context::context` + `ingest::ingest`

**Entry point:**

```rust
pub async fn serve_acp(wiki_root: &Path, wiki_name: &str) -> Result<()> {
    let agent = WikiAgent::new(wiki_root, wiki_name);
    agent_client_protocol_tokio::Stdio::new()
        .connect_to(agent)
        .await
}
```

### `src/cli.rs`

Add `--acp` flag to `wiki serve`. Mutually exclusive with `--sse`.

```
wiki serve --acp              # ACP stdio
wiki serve --acp --wiki <name>
```

### `src/server.rs`

No changes. MCP and ACP are independent transports.

### `src/lib.rs`

Add `pub mod acp;`

## Tests — `tests/acp.rs`

Unit tests:
- `dispatch_workflow` — explicit meta override → correct kind
- `dispatch_workflow` — keyword heuristics for each kind
- `dispatch_workflow` — unknown prompt → Research (default)
- `WikiAgent::initialize` — `system` field contains instructions.md content
- `WikiAgent::new_session` — session stored, id returned
- `WikiAgent::load_session` — known id → success, unknown → ACP error
- `WikiAgent::list_sessions` — returns all sessions
- `WikiAgent::cancel` — active run aborted

Integration tests (in-process pipe, no real stdio):
- Full `initialize` → `new_session` → `prompt` → response sequence
- `cancel` during active run → cancelled response
- `wiki serve --acp` starts, reads stdin, writes stdout, exits cleanly on EOF

Manual tests (document results in task doc):
- Configure Zed: `{ "agent_servers": { "llm-wiki": { "command": "wiki", "args": ["serve", "--acp"] } } }`
- Open Zed agent panel → research workflow streams
- "ingest agent-skills/semantic-commit/" → ingest workflow streams steps

## Zed config snippet for README

```json
{
  "agent_servers": {
    "llm-wiki": {
      "type": "custom",
      "command": "wiki",
      "args": ["serve", "--acp"]
    }
  }
}
```

## Acceptance

```bash
cargo test
# In Zed with config above:
# "what do you know about MoE?" → research workflow streams results
# "ingest agent-skills/semantic-commit/" → ingest workflow streams steps
```

## Constraints

- No LLM dependency
- Use the official SDK — do not hand-roll NDJSON framing
- `src/server.rs` must not be modified
- Update `CHANGELOG.md` with a Phase 11 entry
- Create `docs/dev/acp.md` covering WikiAgent, workflow dispatch, session lifecycle
