---
title: "MCP Server Implementation"
summary: "rmcp setup, tool registration, resource namespacing, stdio + HTTP transport wiring."
status: ready
last_updated: "2025-07-21"
---

# MCP Server Implementation

Implementation reference for the MCP server. Not a specification —
see [server.md](../specifications/engine/server.md) for the design and
[mcp-clients.md](../specifications/integrations/mcp-clients.md) for
client configuration.

## Architecture

The MCP server implements rmcp's `ServerHandler` trait on a struct that
holds a reference to the shared `EngineState`:

```rust
struct McpServer {
    engine: Arc<RwLock<EngineState>>,
}
```

`ServerHandler` methods:
- `get_info` — return capabilities, version, server name
- `list_tools` — return the 15 tool definitions
- `call_tool` — dispatch to tool handler, return result
- `list_resources` — list wiki pages as `wiki://` resources
- `read_resource` — read a page by `wiki://` URI

## Tool Registration

Tools are defined as a static list of `Tool` structs with JSON Schema
input schemas. Each tool has a name, description, and parameter schema.

The current code builds tool definitions inline with helper functions
(`str_prop`, `opt_bool`, etc.). This pattern is reusable.

### Tool name mapping

| Old name                             | New name                                 |
| ------------------------------------ | ---------------------------------------- |
| `wiki_init`                          | `wiki_spaces_create`                     |
| `wiki_read`                          | `wiki_content_read`                      |
| `wiki_write`                         | `wiki_content_write`                     |
| `wiki_new_page` + `wiki_new_section` | `wiki_content_new`                       |
| `wiki_commit`                        | `wiki_content_commit`                    |
| `wiki_lint`                          | remove                                   |
| `wiki_index_check`                   | remove (folded into `wiki_index_status`) |

## Tool Dispatch

A single `call` function matches on tool name and dispatches to
handler functions. Wrapped in `catch_unwind` for panic isolation.

```rust
fn call(engine: &EngineState, name: &str, args: &Map<String, Value>) -> ToolResult
```

Each handler:
1. Extracts arguments from the JSON map
2. Resolves the target wiki (via `WikiUri::resolve` or `--wiki` arg)
3. Calls engine functions
4. Returns `ToolResult` with content and optional resource URIs to notify

### ToolResult

```rust
struct ToolResult {
    content: Vec<Content>,
    is_error: bool,
    notify_uris: Vec<String>,
}
```

`notify_uris` triggers MCP resource update notifications after ingest
or commit — clients that subscribed to those URIs get notified.

## Resource Namespacing

Wiki pages are exposed as MCP resources with `wiki://` URIs:

```
wiki://research/concepts/moe
wiki://work/skills/ingest
```

`list_resources` walks all registered wikis and returns a resource per
page. `read_resource` resolves the URI and returns page content.

## Transports

### stdio (always on)

```rust
let transport = rmcp::transport::io::stdio();
let server = McpServer::new(engine);
server.serve(transport).await?;
```

### HTTP (opt-in)

```rust
let config = StreamableHttpServerConfig::default()
    .with_cancellation_token(cancel_token)
    .with_allowed_hosts(allowed_hosts);
let service = StreamableHttpService::new(move || Ok(server.clone()), Default::default(), config);
let router = axum::Router::new().nest_service("/mcp", service);
axum::serve(listener, router).with_graceful_shutdown(cancel.cancelled_owned()).await?;
```

HTTP retries port binding with exponential backoff. Once bound, runs
until shutdown via `CancellationToken`.

### Both simultaneously

When `--http` is passed, the server clones and runs both transports.
Both share the same `Arc<WikiEngine>`.

## Prompts

The current code defines MCP prompts (`ingest_source`,
`research_question`, `lint_and_fix`) that inject workflow instructions.
These are removed — skills handle workflow instructions now.

MCP prompts may be reintroduced later if useful, but they won't embed
instructions from the engine binary.

## Existing Code

| Component                     | Reusable | Notes                                                           |
| ----------------------------- | -------- | --------------------------------------------------------------- |
| `WikiServer` struct           | rewrite  | Replace with `McpServer` holding `Arc<RwLock<EngineState>>`          |
| `ServerHandler` impl          | mostly   | Update tool names, remove prompts                               |
| `tool_list()`                 | rewrite  | New tool names, new parameters (`--format`, `--type` on search) |
| `call()` dispatch             | mostly   | Update handler names, remove lint/index_check                   |
| Argument helpers              | yes      | `arg_str`, `arg_bool`, `arg_usize`, `arg_str_req`               |
| `resolve_wiki` helper         | rewrite  | Use `WikiUri::resolve` from slug module                         |
| `ToolResult` struct           | yes      | As-is                                                           |
| Resource notification         | yes      | `collect_page_uris` + peer notification                         |
| `serve_stdio`                 | yes      | Transport wiring                                                |
| `serve_http`                  | yes      | Transport wiring with retry                                     |
| Prompt definitions            | remove   | Skills handle this                                              |
| `get_prompt` / `list_prompts` | remove   | Skills handle this                                              |

### Changes needed

- Replace `WikiServer` with `McpServer` backed by `EngineState`
- Rename all tools to new names
- Add `wiki_content_new` (merged page + section with `--section` flag)
- Add `wiki_content_write` with `--file` support
- Add `--type` parameter to search
- Add `--relation` parameter to graph
- Remove `wiki_lint`, `wiki_index_check`, prompt definitions
- Tool handlers call `WikiEngine` for mutations (ingest, commit)

## Crate

```toml
rmcp = { version = "1", features = ["server", "transport-io", "transport-streamable-http-server"] }
axum = "0.8"
```

Reference: https://docs.rs/rmcp/latest/rmcp/
