# Phase 6 — MCP Server + Session Bootstrap

## Context

Phases 1–5 are complete. All engine functions exist and are tested.
You are now wiring everything into a working MCP server and completing
the session bootstrap.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every tool name, parameter name, and return type must match the spec exactly.
- Do not add tools, prompts, or resources not described in the specs.
- Do not modify any file under `docs/`.
- Do not modify Phase 1–5 modules unless fixing a compilation error.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/features.md` — MCP Tools table
- `docs/specifications/commands/serve.md`
- `docs/specifications/commands/instruct.md`
- `docs/specifications/llm/session-bootstrap.md`
- `docs/specifications/integrations/acp-transport.md` — stub only in this phase

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/server.rs`

Implement `WikiServer`:
- Load all registered wikis at startup
- Startup sequence from `docs/specifications/commands/serve.md` §4
- stdio MCP transport always active
- SSE transport when `--sse` flag provided
- Inject `instructions.md` + `schema.md` at session start
  (runtime concatenation — see
  `docs/specifications/llm/session-bootstrap.md` §6)

### 2. `src/mcp.rs` — complete

Wire all tools from phases 1–5 into `WikiServer`. Add:
- `wiki` parameter to all tools (targets specific wiki, defaults to
  `global.default_wiki`)
- MCP resources namespaced as `wiki://<name>/<slug>`
- Resource update notifications on every ingest
- Prompts: `ingest_source`, `research_question`, `lint_and_fix`

Remove `wiki_context` if it exists from the beta implementation.

### 3. `src/instructions.md` — session bootstrap additions

Add to the existing `src/instructions.md`:
- `## session-orientation` preamble
- `## linking-policy` preamble
- Orientation step at the start of every workflow section

Content is specified in:
- `docs/specifications/llm/session-bootstrap.md` §5
- `docs/specifications/llm/backlink-quality.md` §5

### 4. `src/cli.rs`

Add Phase 6 commands:
- `wiki serve [--sse [:<port>]] [--acp] [--dry-run]`
- `wiki instruct [<workflow>]`

`--acp` flag accepted but prints "not yet implemented" — ACP is Phase 7.

### 5. `src/acp.rs`

Stub only — empty `WikiAgent` struct that satisfies compilation.
Full implementation is Phase 7.

## Exit criteria

Before marking Phase 6 complete:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `wiki serve` starts and accepts MCP connections on stdio
- [ ] `wiki serve --sse :8080` starts SSE listener
- [ ] All MCP tools from `docs/specifications/features.md` are callable
- [ ] `wiki instruct crystallize` prints the crystallize workflow
- [ ] `schema.md` content is injected alongside instructions at session start
