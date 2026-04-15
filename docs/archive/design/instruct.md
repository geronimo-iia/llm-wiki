---
title: "Instruct"
summary: "Print embedded workflow instructions for LLMs — the binary is the single source of truth for how to use the wiki."
read_when:
  - Implementing or extending the instruct command
  - Understanding how instructions are embedded and exposed
  - Writing or updating src/instructions.md
status: draft
last_updated: "2025-07-15"
---

# Instruct

`wiki instruct` prints workflow instructions embedded in the binary at compile
time from `src/instructions.md`. It is the single source of truth for how an
LLM should use the wiki — plugin files, slash commands, and ACP sessions all
delegate here.

---

## 1. Purpose

Instructions live in the binary, not in plugin files or external docs. This means:

- Updating instructions = releasing a new binary
- Plugin files stay as thin one-liners that call `wiki instruct`
- ACP sessions inject instructions at `initialize` without a separate call
- MCP server exposes instructions via the `instructions` field on the server handler

---

## 2. `src/instructions.md`

Embedded at compile time:

```rust
const INSTRUCTIONS: &str = include_str!("instructions.md");
```

Structured as named workflow sections:

```markdown
# llm-wiki Instructions

## help
...

## ingest
...

## research
...

## lint
...

## enrich
...
```

Each section is self-contained — `wiki instruct ingest` prints only the
`## ingest` section.

---

## 3. CLI Interface

```
wiki instruct                  # print all instructions
wiki instruct <workflow>       # print instructions for a specific workflow
```

Available workflows:

| Workflow | Description |
|----------|-------------|
| `help` | Overview of available tools and workflows |
| `ingest` | How to ingest a source and optionally enrich it |
| `research` | How to search the wiki and synthesize an answer |
| `lint` | How to run a lint pass and act on the report |
| `enrich` | How to produce and apply an enrichment.json |

### Examples

```bash
wiki instruct                  # full instructions
wiki instruct ingest           # ingest workflow only
wiki instruct research         # research workflow only
```

---

## 4. MCP Server Integration

The MCP server exposes instructions via the `instructions` field:

```rust
#[tool_handler(
    name = "wiki",
    version = "0.1.0",
    instructions = include_str!("instructions.md")
)]
impl ServerHandler for WikiServer {}
```

The full instructions are sent to the LLM at MCP session start — no explicit
`wiki instruct` call needed in MCP workflows.

---

## 5. ACP Integration

On ACP `initialize`, the wiki injects `src/instructions.md` as the system
context. See [acp-transport.md](acp-transport.md).

---

## 6. Rust Module Changes

| Module | Change |
|--------|--------|
| `cli.rs` | Add `instruct` subcommand with optional `<workflow>` arg |
| `main.rs` | Add `const INSTRUCTIONS: &str = include_str!("instructions.md")` |
| `server.rs` | Pass `INSTRUCTIONS` to `#[tool_handler]` instructions field |
| `acp.rs` | Inject `INSTRUCTIONS` at `initialize` response |

---

## 7. Implementation Status

| Feature | Status |
|---------|--------|
| `wiki instruct` (all) | implemented |
| `wiki instruct <workflow>` | **not implemented** |
| `src/instructions.md` structured by workflow | **not implemented** |
| MCP server instructions field | implemented |
| ACP initialize injection | **not implemented** |
