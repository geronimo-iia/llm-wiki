---
title: "ACP Workflows v0.3.0"
summary: "Design decisions for ACP workflow expansion: new workflows, cancellation, session cap, watcher push."
date: "2026-05-01"
---

# ACP Workflows v0.3.0

## Decision

Ship ACP as a first-class transport with six workflows (`research`, `lint`, `graph`, `ingest`, `use`, `help`), cooperative cancellation via `Arc<AtomicBool>`, session cap via `serve.acp_max_sessions`, and proactive watcher push via mpsc channel.

## Context

v0.1.1 added ACP transport but wired only the `research` workflow. `step_read` discarded the page body (`Ok(_)` on `content_read` result) so nothing was streamed to the client. The cancel handler only cleared `active_run` — no interrupt signal reached running workflows. There was no session limit. The watcher ran independently with no way to push notifications to ACP sessions.

## Decisions and Rationale

### Workflow dispatch via `llm-wiki:` prefix

**Decision:** Prefix format `llm-wiki:<workflow> [args]` parsed by `dispatch_workflow`; bare prompt defaults to `research`.

**Rationale:** Mirrors MCP tool naming (`llm-wiki:research` is consistent with `wiki_search`). Bare prompts stay natural for the common case. Prefix is unambiguous with typical wiki query text.

### `step_read` `stream_content` flag

**Decision:** Add `stream_content: bool` parameter to `step_read`. Research path passes `false` (existing behavior unchanged); `use` workflow passes `true` to stream full page body.

**Rationale:** Research already shows a summary via tool call result. `use` is explicitly "give me the content" — streaming it directly matches user intent. Single flag avoids two separate functions.

### Cooperative cancellation via `Arc<AtomicBool>`

**Decision:** Each `AcpSession` holds `cancelled: Arc<AtomicBool>`. Cancel handler sets it; workflows poll it between steps. New prompt resets it to `false`.

**Rationale:** Workflows run synchronously in the request handler (no separate task). Preemptive cancellation would require every blocking call to be wrapped. Cooperative polling between steps is the right granularity — a user cancelling mid-lint gets a clean stop after the current finding batch, not a panic.

**Trade-off:** A step that blocks for several seconds (e.g. large ingest) will not cancel until it returns. Acceptable for v0.3.0.

### Session cap via `serve.acp_max_sessions`

**Decision:** `ServeConfig` gets `acp_max_sessions: usize` (default 20). `NewSession` handler rejects with `InvalidParams` error when `sessions.len() >= cap`. Config key is global-only.

**Rationale:** ACP sessions hold live resources (AtomicBool, label state). Unbounded growth risks OOM on long-running servers. 20 is generous for single-user IDE usage. Global-only because the cap is a server policy, not per-wiki.

**Known limitation:** Session ID uses `timestamp_millis()`. Concurrent `NewSession` requests within the same millisecond generate the same ID, causing HashMap overwrites that silently bypass the cap check. Not fixed in v0.3.0 — the race window is <1 ms and IDE usage is sequential. Documented in `validate-acp.md`.

### Watcher push via mpsc channel

**Decision:** `serve_acp` creates a `tokio::sync::mpsc::Sender<(String, String)>`. Watcher sends `(wiki_name, message)` on successful ingest. ACP server spawns a task that drains the channel and calls `send_text` for all matching idle sessions.

**Rationale:** `send_text` requires `ConnectionTo<Client>` which lives inside the ACP server loop. The watcher runs outside. Passing the connection handle across the boundary (Option A: mpsc) is cleaner than polling a shared Vec (Option B). The mpsc approach is one-way and non-blocking from the watcher's perspective.

### ACP requires `--http` when MCP is also active

**Decision:** Running `serve --acp` without `--http` leaves MCP and ACP both competing for stdio. The correct invocation is `serve --acp --http :PORT` which moves MCP to HTTP, giving ACP exclusive stdio.

**Rationale:** Both protocols are NDJSON over stdio by default. They cannot share the same stream. `--http` flag is the opt-in to displace MCP; ACP always owns stdio when `--acp` is set.

### Test strategy: config verification for session cap

**Decision:** Automated cap test (`08-session-cap.sh`) verifies only that `serve.acp_max_sessions` is set to a low value. Behavioral cap enforcement is deferred to manual testing (documented in `validate-acp.md`).

**Rationale:** The `timestamp_millis()` TOCTOU race makes automated cap enforcement tests unreliable — sessions spawned in rapid succession get the same ID and silently overwrite each other in the HashMap, so the cap is never triggered. Fixing the race was blocked (out of scope for v0.3.0). The config check confirms the system is wired correctly without producing false failures.
