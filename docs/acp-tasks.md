# ACP Streaming Tasks

Implement streaming tool calls in ACP workflows per acp-transport.md §3.4.

Reference:
- [ACP transport spec](specifications/integrations/acp-transport.md)
- [ACP SDK usage](implementation/acp-sdk.md)

---

## Task A — Streaming helpers

**Goal:** Add `send_tool_call` and `send_tool_result` helpers to `WikiAgent`.

### Analysis

Currently `WikiAgent` has one helper: `send_message` which sends
`AgentMessageChunk`. The SDK supports two more event types needed for
streaming: `ToolCall` (announce) and `ToolCallUpdate` (result).

Both follow the same channel pattern as `send_message` — wrap in
`SessionNotification`, send through `update_tx`, await `oneshot` ack.

### Code changes

- `src/acp.rs` — add to `WikiAgent`:
  - `send_tool_call(session_id, id, title, kind)` — sends
    `SessionUpdate::ToolCall` with `status: InProgress`.
  - `send_tool_result(session_id, id, status, content)` — sends
    `SessionUpdate::ToolCallUpdate` with the given status and content.
  - `make_tool_id(workflow, step)` — returns
    `"{workflow}-{step}-{timestamp_ms}"`.

### Tests

- `tests/acp.rs` — new test: `send_tool_call_and_result_appear_in_channel`
  — call both helpers, drain channel, assert `ToolCall` and
  `ToolCallUpdate` variants appear.

### Exit criteria

- Helpers compile and send correct `SessionUpdate` variants.
- `cargo test` passes.

---

## Task B — Research workflow streaming

**Goal:** Break the research workflow into streaming steps.

### Analysis

Current research workflow in `prompt()`:
1. Call `search::search()` synchronously
2. Format results into a single string
3. Send one `send_message` at the end

Target streaming sequence:
1. `send_message("Searching for: {query}...")`
2. `send_tool_call("wiki_search: {query}", Search)`
3. Execute `search::search()`
4. `send_tool_result(Completed, "{N} results")` or `send_tool_result(Failed, error)`
5. If results: `send_tool_call("wiki_read: {top_slug}", Read)`
6. Execute `markdown::read_page()`
7. `send_tool_result(Completed, "")`
8. `send_message("Based on {N} pages: {summary}")`

### Code changes

- `src/acp.rs` — in `prompt()`, replace the `"research"` match arm with
  the streaming sequence above. Extract into a helper method
  `run_research(&self, session_id, query, wiki_entry)`.

### Tests

- `tests/acp.rs` — update `prompt_research_workflow_streams_answer`:
  - Assert `messages.len() >= 2` (at least a progress message and a final
    message).
  - New test: `prompt_research_workflow_streams_tool_calls` — drain
    channel, assert at least one `ToolCall` variant and one
    `ToolCallUpdate` variant appear.

### Exit criteria

- Research workflow sends multiple streaming events.
- At least one `ToolCall` + `ToolCallUpdate` pair per search step.
- Error in search produces `Failed` tool call update + error message.
- `cargo test` passes.

---

## Task C — Lint workflow streaming

**Goal:** Break the lint workflow into streaming steps.

### Analysis

Current lint workflow:
1. Call `lint::lint()` synchronously
2. Format report into a single string
3. Send one `send_message`

Target streaming sequence:
1. `send_message("Running lint...")`
2. `send_tool_call("wiki_lint: {wiki}", Execute)`
3. Execute `lint::lint()`
4. `send_tool_result(Completed, "{N} orphans, {M} stubs, ...")` or
   `send_tool_result(Failed, error)`
5. `send_message("Lint report for {wiki}: ...")`

### Code changes

- `src/acp.rs` — in `prompt()`, replace the `"lint"` match arm with the
  streaming sequence. Extract into `run_lint(&self, session_id, wiki_entry)`.

### Tests

- `tests/acp.rs` — update `prompt_lint_workflow_dispatches_on_keyword`:
  - Assert `messages.len() >= 2`.
  - New test: `prompt_lint_workflow_streams_tool_calls` — assert `ToolCall`
    + `ToolCallUpdate` pair appears.

### Exit criteria

- Lint workflow sends multiple streaming events.
- `cargo test` passes.

---

## Task D — Test infrastructure update

**Goal:** Extend `drain_messages` to capture all `SessionUpdate` variants.

### Analysis

Current `drain_messages` only captures `AgentMessageChunk` text. To test
tool call streaming, we need to capture `ToolCall` and `ToolCallUpdate`
variants too.

### Code changes

- `tests/acp.rs` — replace `drain_messages` with `drain_updates` that
  returns `Vec<SessionUpdate>` (the full enum). Add helper functions:
  - `extract_messages(updates) -> Vec<String>` — filter `AgentMessageChunk`.
  - `extract_tool_calls(updates) -> Vec<ToolCall>` — filter `ToolCall`.
  - `extract_tool_updates(updates) -> Vec<ToolCallUpdate>` — filter
    `ToolCallUpdate`.
  - Update existing tests to use the new helpers.

### Exit criteria

- All existing ACP tests pass with the new infrastructure.
- New tests can assert on tool call events.
- `cargo test` passes.

---

## Task E — Ingest and crystallize placeholders

**Goal:** Document that ingest and crystallize workflows remain
single-message until they have real multi-step logic.

### Analysis

Both workflows currently return a placeholder string. Adding streaming
would be artificial — there are no real intermediate steps to stream.

### Code changes

None. This task is documentation-only.

### Exit criteria

- acp-transport.md §3.4 documents that ingest and crystallize are
  single-message placeholders.
- No code changes needed.

---

## Execution order

| Order | Task | Dependencies |
|-------|------|-------------|
| 1 | D — Test infrastructure | None |
| 2 | A — Streaming helpers | None |
| 3 | B — Research streaming | A, D |
| 4 | C — Lint streaming | A, D |
| 5 | E — Placeholder docs | None |
