---
title: "MCP Tool Implementation"
summary: "Patterns for adding new MCP tools — tool definition, handler, dispatch, and testing."
status: ready
last_updated: "2026-04-28"
---

# MCP Tool Implementation

How to add a new MCP tool to the engine. Every tool follows the same
pattern across 4 files.

## Files to touch

| File | What to add |
|------|-------------|
| `src/mcp/tools.rs` | Tool definition (name, description, parameter schema) |
| `src/mcp/handlers.rs` | Handler function (parse args, call ops, format result) |
| `src/mcp/tools.rs` (dispatch) | Match arm in `call()` function |
| `src/ops/<module>.rs` | Business logic (shared with CLI) |
| `src/cli.rs` | CLI subcommand (if the tool has a CLI equivalent) |
| `tests/mcp.rs` | Update tool count assertion |

## 1. Tool definition (tools.rs)

Add a `Tool::new()` entry to `tool_list()`:

```rust
Tool::new(
    "wiki_my_tool",
    "Short description of what it does",
    schema(
        json!({
            "required_param": str_prop("Description of required param"),
            "optional_param": opt_str("Description of optional param"),
            "flag": opt_bool("Description of boolean flag"),
            "count": opt_int("Description of integer param"),
            "wiki": opt_str("Target wiki name"),
        }),
        &["required_param"],  // required parameters
    ),
),
```

Helper functions for parameter types:
- `str_prop(desc)` — required string
- `opt_str(desc)` — optional string
- `opt_bool(desc)` — optional boolean
- `opt_int(desc)` — optional integer

## 2. Handler function (handlers.rs)

```rust
pub fn handle_my_tool(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    // 1. Parse arguments
    let required = arg_str_req(args, "required_param")?;
    let optional = arg_str(args, "optional_param");
    let flag = arg_bool(args, "flag");

    // 2. Get engine and resolve wiki
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;

    // 3. Call ops function
    let result = ops::my_operation(&engine, &wiki_name, &required)
        .map_err(|e| format!("{e}"))?;

    // 4. Format and return
    let s = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("{e}"))?;
    ok_text(s)
}
```

### Argument helpers (from `helpers.rs`)

| Helper | Returns | Use for |
|--------|---------|---------|
| `arg_str(args, key)` | `Option<String>` | Optional string params |
| `arg_str_req(args, key)` | `Result<String, String>` | Required string params |
| `arg_bool(args, key)` | `bool` | Boolean flags (default false) |
| `arg_usize(args, key)` | `Option<usize>` | Optional integer params |
| `resolve_wiki_name(&engine, args)` | `Result<String, String>` | Wiki name from `wiki` param or default |

### Return helpers

| Helper | Use for |
|--------|---------|
| `ok_text(string)` | Success with text content |
| `err_text(string)` | Error content (used by dispatch, not handlers) |

### When the handler needs to drop the engine lock

If the operation needs `&WikiEngine` (e.g. for write operations
that call `refresh_index`), drop the engine read lock first:

```rust
let engine = server.engine();
let wiki_name = resolve_wiki_name(&engine, args)?;
drop(engine);  // release read lock

let report = ops::my_write_op(&server.manager, &wiki_name, ...)
    .map_err(|e| format!("{e}"))?;
```

## 3. Dispatch (tools.rs)

Add a match arm in the `call()` function:

```rust
"wiki_my_tool" => handlers::handle_my_tool(server, args),
```

## 4. Business logic (ops/<module>.rs)

The handler calls an `ops::` function that contains the actual logic.
This function is shared with the CLI — same code, different entry point.
Add the function to the most relevant existing module under `src/ops/`,
or create a new one and declare it in `src/ops/mod.rs`.

```rust
pub fn my_operation(engine: &EngineState, wiki_name: &str, param: &str) -> Result<MyResult> {
    let space = engine.space(wiki_name)?;
    // ... business logic using space.type_registry, space.index_schema, etc.
    Ok(result)
}
```

### Accessing per-wiki state

```rust
let space = engine.space(wiki_name)?;
space.type_registry   // SpaceTypeRegistry — validators, aliases
space.index_schema    // IndexSchema — tantivy field handles
space.wiki_root       // PathBuf — wiki/ directory
space.repo_root       // PathBuf — repository root
space.index_path      // PathBuf — ~/.llm-wiki/indexes/<name>/
```

## 5. Update tool count test

In `tests/mcp.rs`, update the assertion:

```rust
assert_eq!(tools.len(), N);  // increment by 1
```

## Checklist for a new tool

- [ ] Tool definition in `tools.rs` with correct parameter schema
- [ ] Handler in `handlers.rs` following the pattern
- [ ] Dispatch match arm in `tools.rs` `call()`
- [ ] Business logic in `src/ops/<module>.rs`
- [ ] CLI subcommand in `cli.rs` (if applicable)
- [ ] CLI dispatch in `main.rs` (if applicable)
- [ ] Tool count test updated in `tests/mcp.rs`
- [ ] Integration tests for the ops function
- [ ] Spec document in `docs/specifications/tools/`
