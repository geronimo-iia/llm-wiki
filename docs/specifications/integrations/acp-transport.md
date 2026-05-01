---
title: "ACP Transport"
summary: "ACP server for Zed and VS Code agent panels — session-oriented, streaming."
read_when:
  - Integrating llm-wiki with Zed or VS Code agent panel
status: ready
last_updated: "2026-05-01"
---

# ACP Transport

ACP (Agent Client Protocol) is a session-oriented, streaming protocol
over stdio/NDJSON. `llm-wiki serve --acp` makes llm-wiki a first-class
IDE agent with zero MCP configuration required.


## Why ACP

MCP is request/response — the IDE calls a tool, gets a result, no
streaming. ACP is session-oriented and streaming — every step of a
multi-step workflow streams back as events visible to the user.

| Concern                  | MCP stdio           | ACP stdio            |
| ------------------------ | ------------------- | -------------------- |
| Zed agent panel          | requires MCP config | native — zero config |
| Streaming workflow steps | not visible         | streams as events    |
| Session continuity       | stateless           | named sessions       |
| Cancel mid-workflow      | not supported       | `cancel` message     |

**Important:** ACP `tool_call` events are IDE notifications — they show
the user what the server is doing. They are not LLM-invoked tool calls.
The LLM sends one prompt; the server runs a fixed Rust workflow and streams
progress back. The LLM has no agency mid-workflow.

Multi-step agentic work (deciding which tools to call based on results)
requires MCP. ACP and MCP are complementary, not exclusive.


## Protocol

ACP is NDJSON over stdio. Key messages:

| Message      | Direction     | Purpose                                    |
| ------------ | ------------- | ------------------------------------------ |
| `initialize` | client → wiki | Start session, wiki sends capabilities     |
| `newSession` | client → wiki | Create named session                       |
| `prompt`     | client → wiki | Submit user message                        |
| `cancel`     | client → wiki | Cancel active run                          |
| `message`    | wiki → client | Streaming assistant text                   |
| `tool_call`  | wiki → client | Streaming tool invocation (visible in IDE) |
| `done`       | wiki → client | Run complete                               |


## Session Model

Sessions are transient conversation threads stored in memory for the
process lifetime. A session targets a specific wiki from the spaces
config (default wiki if not specified).


## Zed Configuration

Zed needs **both** ACP and MCP configured to use the full workflow:

```json
{
  "agent_servers": {
    "llm-wiki": {
      "type": "custom",
      "command": "llm-wiki",
      "args": ["serve", "--acp"],
      "env": {}
    }
  },
  "context_servers": {
    "llm-wiki-mcp": {
      "command": "llm-wiki",
      "args": ["serve"]
    }
  }
}
```

- **ACP** (`agent_servers`) — streaming panel, workflow triggers, skill loading
- **MCP** (`context_servers`) — LLM tool calls for agentic work (write pages,
  search, resolve slugs, etc.)

With ACP only, the LLM is limited to triggering fixed server-side workflows.
With both, the LLM can follow skills (loaded via `llm-wiki:use`) and call
any of the 22 MCP tools directly.


## Workflows

Prompts are dispatched by prefix. Bare prompts (no `llm-wiki:` prefix) default
to the `research` workflow.

```
llm-wiki:<workflow> <args>
```

### Dispatch rules

| Prefix | Workflow | Args |
|--------|----------|------|
| `llm-wiki:research <query>` | research | search query |
| `llm-wiki:lint [rules]` | lint | comma-separated rule names, or empty for all |
| `llm-wiki:graph [slug]` | graph | optional root slug for subgraph |
| `llm-wiki:ingest [path]` | ingest | path to file or directory; defaults to wiki root |
| `llm-wiki:use <slug>` | use | slug of page to read in full |
| `llm-wiki:help` | help | — |
| `<bare prompt>` | research | full prompt text as query |
| `llm-wiki:<unknown>` | — | error → help hint |

### `research`

Searches the wiki and reads the top result.

```
→ tool_call: wiki_search("<query>")
→ tool_result: top matches
→ tool_call: wiki_content_read("<top-slug>")
→ message: slug list
→ done
```

Streams a gap note when no results found. Multi-step synthesis with
decisions belongs in the `research` skill (MCP + skills), not here.

### `lint`

Runs lint rules and streams each finding.

```
→ tool_call: wiki_lint [rules=<rules>]
→ tool_result: "<N> findings (<E> errors, <W> warnings)"
→ message: "[<severity>] <slug>: <message>"  (one per finding)
→ done
```

`rules` is a comma-separated subset of: `orphan`, `broken-link`,
`missing-fields`, `stale`, `unknown-type`, `broken-cross-wiki-link`.
Empty or absent means all rules.

### `graph`

Renders the concept graph in `llms` format.

```
→ tool_call: wiki_graph [root=<slug>]
→ tool_result: "Graph: <N> nodes, <E> edges"
→ message: <llms-format graph description>
→ done
```

If `slug` is provided, renders a subgraph rooted at that slug (default depth
from config). Empty prompt renders the full wiki graph.

### `ingest`

Validates and indexes pages, then reports the result.

```
→ tool_call: wiki_ingest: <path>
→ tool_result: "Ingested: <N> pages validated, <W> warnings, commit=<sha|none>"
→ done
```

Path defaults to the wiki root when omitted. Warnings are counted but not
individually streamed (use `lint` to inspect individual findings).

### `use`

Reads a single page and streams its full body to the IDE. Primary use:
loading a skill from the wiki into the agent context so the LLM can follow
its instructions using MCP tools.

```
→ tool_call: wiki_content_read("<slug>")
→ tool_result: <page body>
→ done
```

Requires a slug argument. Responds with a usage hint if omitted.

Example:
```
llm-wiki:use skills/research   → loads research skill
llm-wiki:use skills/ingest     → loads ingest skill
llm-wiki:use skills/crystallize → loads crystallize skill
```

After loading a skill, the LLM follows its instructions using MCP tool calls.

### `help`

Returns a plain-text listing of all available workflows with one-line
descriptions. Also returned (with an "unknown workflow" prefix) for any
unrecognised `llm-wiki:<x>` prefix.

### Error contract

All workflows follow the same error contract: on `ops` failure, the tool call
status is set to `Failed` with the error message in the result body, and the
workflow exits cleanly (no panic, no `done` suppressed). The IDE always
receives a `done` event.


## ACP + MCP + Skills: combined workflow

The three layers are designed to work together:

```
llm-wiki:use skills/crystallize     ← ACP: load skill body into LLM context
                ↓
LLM reads skill, follows instructions using MCP:
  wiki_content_new("concept/moe-scaling")   ← MCP tool call
  wiki_content_write(slug, body)            ← MCP tool call
                ↓
llm-wiki:ingest                     ← ACP: trigger ingest, stream progress
```

ACP handles streaming and IDE visibility. MCP gives the LLM tool access.
Skills tell the LLM what to do.


## What ACP Does Not Replace

- **MCP stdio** — agentic tool use, Claude Code, batch ingest
- **MCP SSE** — remote multi-client access (ACP is stdio-only)

See [server.md](../engine/server.md) for transport configuration and
[mcp-clients.md](mcp-clients.md) for MCP client setup.
