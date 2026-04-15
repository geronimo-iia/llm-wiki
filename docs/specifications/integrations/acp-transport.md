---
title: "ACP Transport"
summary: "How llm-wiki exposes an ACP server for IDE integration — session-oriented, streaming, with instructions injected at initialize."
read_when:
  - Implementing wiki serve --acp for Zed or VS Code integration
  - Understanding why ACP adds value over MCP for interactive workflows
  - Designing the ACP session → wiki workflow mapping
status: draft
last_updated: "2025-07-15"
---

# ACP Transport

ACP (Agent Client Protocol) is a session-oriented, streaming protocol over
stdio/NDJSON. Adding `wiki serve --acp` makes llm-wiki a first-class IDE
agent with zero MCP configuration required.

---

## 1. Why ACP Adds Value

MCP is request/response tool-calling — the IDE calls a tool, gets a result,
no streaming, no session. ACP is session-oriented and streaming — every step
of a multi-turn workflow streams back as a `message` or `tool_call` event
visible to the user in real time.

llm-wiki workflows are inherently multi-turn:

```
User: "ingest this folder"

→ tool_call: wiki_search({ query: "semantic commit", wiki: "research" })
→ message: "Found existing context. Writing pages..."
→ tool_call: wiki_write({ path: "skills/semantic-commit/index.md", content: "..." })
→ tool_call: wiki_ingest({ path: "skills/semantic-commit/" })
→ done: "+1 page, 2 assets committed"
```

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

ACP is NDJSON over stdio. One JSON object per line.

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

---

## 3. Session → Wiki Workflow Mapping

### 3.1 Session Model

Sessions are transient conversation threads stored in memory for the process
lifetime. A session targets a specific wiki from the spaces config.

```rust
pub struct AcpSession {
    pub id:         String,
    pub label:      Option<String>,
    pub wiki:       Option<String>,   // target wiki name — None = default wiki
    pub created_at: u64,
    pub active_run: Option<String>,
}
```

A completed research session can optionally be saved as a `query-result` page
via `wiki_ingest`.

### 3.2 `initialize` → Inject Instructions

On `initialize`, the wiki injects `src/instructions.md` as system context.
The LLM starts every session already knowing the wiki workflows and
conventions. No separate `wiki instruct` call needed.

```json
{
  "type": "initialize_response",
  "agent": { "name": "llm-wiki", "version": "0.x.0" },
  "system": "<contents of src/instructions.md>",
  "capabilities": { "streaming": true, "sessions": true }
}
```

### 3.3 `prompt` → Workflow Dispatch

A `prompt` triggers a wiki workflow. Dispatch is determined by session
`meta.workflow` if set, otherwise by keyword matching on the prompt text:

| Prompt contains | Workflow dispatched |
|-----------------|---------------------|
| "ingest", "add", path-like | ingest workflow |
| "search", "find", "what do you know" | research workflow |
| "lint", "orphans", "stubs" | lint workflow |
| "crystallize", "distil", "capture" | crystallize workflow |
| anything else | research workflow (default) |

### 3.4 Workflow Streaming Examples

**Ingest workflow:**

```
prompt: "ingest the semantic-commit skill into research/skills"

wiki → tool_call: wiki_search({ query: "semantic commit", wiki: "research" })
wiki → tool_call: wiki_write({ path: "skills/semantic-commit/index.md", content: "..." })
wiki → tool_call: wiki_ingest({ path: "skills/semantic-commit/" })
wiki → message: "Committed: skills/semantic-commit + lifecycle.yaml co-located."
wiki → done: { stop_reason: "stop" }
```

**Research workflow:**

```
prompt: "what do we know about MoE scaling?"

wiki → tool_call: wiki_search({ query: "MoE scaling", wiki: "research" })
wiki → tool_call: wiki_read({ uri: "wiki://research/concepts/mixture-of-experts" })
wiki → tool_call: wiki_read({ uri: "wiki://research/sources/switch-transformer-2021" })
wiki → message: "Based on 2 pages: MoE reduces compute 8x at pre-training scale..."
wiki → done: { stop_reason: "stop" }
```

**Lint workflow:**

```
prompt: "run lint on research wiki"

wiki → tool_call: wiki_lint({ wiki: "research" })
wiki → message: "Found 2 orphans, 1 missing stub, 0 empty sections."
wiki → tool_call: wiki_new_page({ uri: "wiki://research/concepts/flash-attention" })
wiki → message: "Created stub for missing page."
wiki → done: { stop_reason: "stop" }
```

---

## 4. Implementation

### 4.1 Rust SDK

```toml
agent-client-protocol       = "0.10"
agent-client-protocol-tokio = "0.1"
```

### 4.2 `Agent` Trait Implementation

`WikiAgent` holds the full spaces config — all registered wikis are accessible
per session, consistent with `wiki serve` mounting all wikis at startup.

```rust
pub struct WikiAgent {
    spaces: Arc<Spaces>,
    sessions: Mutex<HashMap<String, AcpSession>>,
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

```rust
use agent_client_protocol_tokio::serve_agent_stdio;

pub async fn serve_acp(spaces: Arc<Spaces>) -> Result<()> {
    let agent = WikiAgent::new(spaces);
    serve_agent_stdio(agent).await
}
```

### 4.4 New module `acp.rs`

```
src/
├── acp.rs    ← new: WikiAgent, AcpSession, workflow dispatch
├── server.rs ← existing: WikiServer, startup, stdio + SSE transport wiring
└── mcp.rs    ← existing: all MCP tools, resources, prompts
```

`acp.rs` calls the same functions as `mcp.rs` — `ingest::ingest`,
`search::search`, `lint::lint`, `markdown::read_page`. No duplication of logic.

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

All registered wikis are available in every session. The session targets the
default wiki unless `meta.wiki` is set at `newSession`.

---

## 6. What ACP Does Not Replace

- **MCP stdio** — agent pipelines, Claude Code tool calls, batch ingest
- **MCP SSE** — remote multi-client access (ACP is stdio-only)
- **`wiki instruct` CLI** — printing instructions outside of an ACP session

---

## 7. Open Questions

1. **Workflow dispatch heuristic** — keyword matching is fragile. Should
   `newSession` meta carry an explicit `workflow` field instead?

2. **Session persistence** — sessions are in-memory only. Should active
   sessions be checkpointed to `.wiki/acp-sessions.json` on process restart?

3. **Per-session wiki targeting** — currently set at `newSession` via
   `meta.wiki`. Should the user be able to switch target wiki mid-session?
