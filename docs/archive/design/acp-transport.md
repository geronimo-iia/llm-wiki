---
title: "ACP Transport"
summary: "How llm-wiki exposes an ACP (Agent Client Protocol) server for IDE integration — session-oriented, streaming, with instructions injected at initialize."
read_when:
  - Implementing wiki serve --acp for Zed or VS Code integration
  - Understanding why ACP adds value over MCP for interactive workflows
  - Designing the ACP session → wiki workflow mapping
status: draft
last_updated: "2025-07-15"
---

# ACP Transport

ACP (Agent Client Protocol) is a session-oriented, streaming protocol over
stdio/NDJSON. It is how Zed's agent panel and VS Code agent extensions drive
an agent process. Adding `wiki serve --acp` makes llm-wiki a first-class IDE
agent with zero MCP configuration required.

---

## 1. Why ACP Adds Value for llm-wiki

MCP is request/response tool-calling. The IDE calls a tool, gets a result. There
is no session, no streaming narrative, no conversation thread visible to the user.

ACP is session-oriented and streaming. Every step of a multi-turn workflow
streams back as a `message` or `tool_call` event. The user sees the workflow
unfold in real time in the editor.

llm-wiki workflows are inherently multi-turn:

```
User: "ingest this folder and enrich it"

→ tool_call: wiki_context("key concepts")
→ tool_call: wiki_read("concepts/mixture-of-experts")
→ tool_call: wiki_read("sources/switch-transformer-2021")
→ message: "Found 2 existing pages. Producing enrichment..."
→ tool_call: wiki_ingest(path, analysis: enrichment.json)
→ done: "+3 enrichments, 1 contradiction detected"
```

With MCP alone, the IDE sees one tool call go in and a result come out. With ACP,
every step is visible. The user can cancel mid-workflow, see which pages were
read, and understand what the enrichment produced.

### Concrete value

| Concern | MCP stdio | ACP stdio |
|---------|-----------|-----------|
| Zed agent panel | requires MCP config | native — zero config |
| VS Code agent extension | requires MCP config | native |
| Streaming workflow steps | not visible | streams as events |
| Session continuity | stateless | named sessions |
| Cancel mid-workflow | not supported | `cancel` message |
| Instructions at connect | via `wiki instruct` tool call | injected at `initialize` |
| Batch pipelines | right tool | wrong tool |

---

## 2. ACP Protocol — Relevant Subset

ACP is NDJSON over stdio. One JSON object per line. The wiki implements the
server (agent) side.

```
initialize      client → wiki   start session, wiki sends capabilities + instructions
newSession      client → wiki   create named session
loadSession     client → wiki   resume existing session
listSessions    client → wiki   list sessions (IDE session picker)
prompt          client → wiki   submit a user message
cancel          client → wiki   cancel active run
message         wiki → client   streaming assistant text
tool_call       wiki → client   streaming tool invocation (visible to user)
done            wiki → client   run complete (stop | cancel | error)
```

No Rust ACP crate exists. The protocol is simple enough to implement directly
with `serde_json` + `tokio::io::BufReader` over stdin/stdout.

---

## 3. Session → Wiki Workflow Mapping

### 3.1 Session model

ACP sessions map to named wiki workflow contexts. A session is not a wiki page —
it is a transient conversation thread. Sessions are stored in memory for the
process lifetime.

```rust
pub struct AcpSession {
    pub id:         String,       // ACP session id
    pub label:      Option<String>,
    pub wiki:       Option<String>, // target wiki name (from meta or default)
    pub created_at: u64,
    pub active_run: Option<String>,
}
```

Optionally, a completed research session can be saved as a `query-result` page
via `wiki_ingest` — the conversation becomes a wiki artifact.

### 3.2 `initialize` → inject instructions

On `initialize`, the wiki sends `src/instructions.md` as the system context.
The LLM starts every session already knowing the enrichment contract and doc
authoring rules. No separate `wiki instruct` call needed.

```json
{
  "type": "initialize_response",
  "agent": { "name": "llm-wiki", "version": "0.x.0" },
  "system": "<contents of src/instructions.md>",
  "capabilities": { "streaming": true, "sessions": true }
}
```

### 3.3 `prompt` → workflow dispatch

A `prompt` message triggers a wiki workflow. The wiki determines the workflow
from the prompt text (or from session metadata set at `newSession`):

| Prompt contains | Workflow dispatched |
|-----------------|---------------------|
| "ingest", "add", path-like | ingest workflow |
| "search", "find", "what do you know" | research workflow |
| "lint", "orphans", "contradictions" | lint workflow |
| "enrich", "analyze" | enrichment workflow |
| anything else | research workflow (default) |

Each workflow step emits `tool_call` events (visible in the IDE) and `message`
events (narrative text). The run ends with `done`.

### 3.4 Workflow streaming example

```
prompt: "ingest agent-skills/semantic-commit/ into the wiki"

wiki → tool_call: { name: "wiki_context", input: { question: "semantic commit" } }
wiki → tool_call: { name: "wiki_read", input: { slug: "skills/semantic-commit" } }
wiki → message: "Page not found. Will create from folder."
wiki → tool_call: { name: "wiki_ingest", input: { path: "agent-skills/semantic-commit/", prefix: "skills" } }
wiki → message: "Ingested: skills/semantic-commit/index.md + lifecycle.yaml co-located."
wiki → done: { stop_reason: "stop" }
```

---

## 4. Implementation

### 4.1 Rust SDK

The official Rust ACP SDK exists, maintained by Zed:
`https://github.com/agentclientprotocol/rust-sdk`

Two crates needed:

```toml
agent-client-protocol       = "0.10"  # Agent trait + typed schema
agent-client-protocol-tokio = "0.1"   # tokio stdio transport
```

The SDK handles all NDJSON framing, request routing, and protocol versioning.
No hand-rolled transport needed.

### 4.2 `Agent` trait implementation

The wiki implements the `Agent` trait for a `WikiAgent` struct:

```rust
use agent_client_protocol::{Agent, Result};
use agent_client_protocol_schema::{
    InitializeRequest, InitializeResponse,
    NewSessionRequest, NewSessionResponse,
    LoadSessionRequest, LoadSessionResponse,
    ListSessionsRequest, ListSessionsResponse,
    PromptRequest, PromptResponse,
    CancelNotification,
};

pub struct WikiAgent {
    wiki_root: PathBuf,
    wiki_name: String,
    sessions:  Mutex<HashMap<String, AcpSession>>,
}

#[async_trait::async_trait(?Send)]
impl Agent for WikiAgent {
    async fn initialize(&self, _req: InitializeRequest) -> Result<InitializeResponse> {
        // inject src/instructions.md as system context
    }
    async fn new_session(&self, req: NewSessionRequest) -> Result<NewSessionResponse> { ... }
    async fn load_session(&self, req: LoadSessionRequest) -> Result<LoadSessionResponse> { ... }
    async fn list_sessions(&self, req: ListSessionsRequest) -> Result<ListSessionsResponse> { ... }
    async fn prompt(&self, req: PromptRequest) -> Result<PromptResponse> { ... }
    async fn cancel(&self, notif: CancelNotification) { ... }
}
```

### 4.3 Tokio stdio transport

`agent-client-protocol-tokio` provides the stdio wiring:

```rust
use agent_client_protocol_tokio::serve_agent_stdio;

pub async fn serve_acp(wiki_root: &Path, wiki_name: &str) -> Result<()> {
    let agent = WikiAgent::new(wiki_root, wiki_name);
    serve_agent_stdio(agent).await
}
```

### 4.4 New module `acp.rs`

```
src/
├── acp.rs    ← new: WikiAgent, AcpSession, workflow dispatch
├── server.rs ← existing MCP server (unchanged)
```

`acp.rs` calls the same functions as `server.rs` — `context::context`,
`ingest::ingest`, `search::search`, `lint::lint`. No duplication of logic.

### 4.5 CLI

```
wiki serve              → MCP stdio (existing)
wiki serve --sse :8080  → MCP SSE (existing)
wiki serve --acp        → ACP stdio (new)
```

---

## 5. Zed Configuration

```json
{
  "agent_servers": {
    "llm-wiki": {
      "type": "custom",
      "command": "wiki",
      "args": ["serve", "--acp"],
      "env": {}
    }
  }
}
```

For a specific wiki:

```json
{
  "agent_servers": {
    "llm-wiki (research)": {
      "type": "custom",
      "command": "wiki",
      "args": ["serve", "--acp", "--wiki", "research"],
      "env": {}
    }
  }
}
```

---

## 6. What ACP Does Not Replace

- **MCP stdio** — agent pipelines, Claude Code tool calls, batch ingest. MCP is
  the right protocol when there is no human in the loop.
- **MCP SSE** — remote multi-client access. ACP is stdio-only (one client per
  process).
- **`wiki instruct` CLI** — still useful for printing instructions outside of
  an ACP session.

---

## 7. Open Questions

1. **Workflow dispatch heuristic** — keyword matching on prompt text is fragile.
   Should the session `meta` carry an explicit `workflow` field instead?

2. **Session persistence** — sessions are in-memory only. If the wiki process
   restarts, sessions are lost. Should active sessions be checkpointed to
   `.wiki/acp-sessions.json`?

3. **`wiki` parameter in ACP** — the `--wiki` flag at startup sets the target
   wiki for the whole ACP process. Should per-session wiki targeting be supported
   via `newSession` meta instead?
