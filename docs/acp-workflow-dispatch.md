---
title: "ACP Workflow Dispatch"
summary: "How ACP prompt dispatch works — slash commands, engine-executed vs skill-delegated workflows, instruction streaming."
status: draft
last_updated: "2025-07-15"
---

# ACP Workflow Dispatch

The ACP agent receives user prompts and dispatches them to workflows.
Some workflows are executed by the engine directly. Others delegate to
the IDE's LLM by streaming skill instructions.

---

## 1. The Problem

The current dispatch uses keyword matching on prompt text. This is fragile:
- "add some context" triggers ingest
- Any path-like string triggers ingest
- No way to explicitly request a workflow

The ACP agent is not an LLM — it can search and lint, but it can't write
pages or synthesize knowledge. Workflows like ingest and crystallize
require an LLM to do the actual work.

---

## 2. Slash Command Dispatch

The user (or the IDE) prefixes the prompt with a slash command:

```
/llm-wiki:<workflow> [prompt text]
```

This matches the Claude plugin convention (`/llm-wiki:ingest`,
`/llm-wiki:research`, etc.) — consistent across ACP and Claude.

### Parsing

```rust
fn parse_workflow(prompt: &str) -> (&str, &str) {
    if let Some(rest) = prompt.strip_prefix("/llm-wiki:") {
        let (cmd, text) = rest.split_once(' ').unwrap_or((rest, ""));
        (cmd.trim(), text.trim())
    } else {
        ("research", prompt)  // default: research
    }
}
```

Examples:
- `/llm-wiki:ingest the semantic-commit skill` → workflow `ingest`, text `the semantic-commit skill`
- `/llm-wiki:lint` → workflow `lint`, text empty
- `what do we know about MoE?` → workflow `research`, text `what do we know about MoE?`

### Fallback

When no slash command is present, fall back to keyword matching as today.
This preserves backward compatibility — existing prompts still work.

```rust
fn dispatch_workflow(prompt: &str) -> (&str, &str) {
    // Try slash command first
    if prompt.starts_with("/llm-wiki:") {
        return parse_slash_command(prompt);
    }
    // Fallback: keyword matching
    let workflow = keyword_match(prompt);
    (workflow, prompt)
}
```

---

## 3. Two Types of Workflows

### Engine-executed

The ACP agent calls engine functions directly and streams results.
No LLM needed — the engine does the work.

| Workflow | What it does |
|----------|-------------|
| `research` | `search::search()` + `markdown::read_page()`, streams results |
| `lint` | `lint::lint()`, streams report |

These are already implemented with streaming (Tasks B and C).

### Skill-delegated

The ACP agent streams skill instructions from `llm-wiki instruct <workflow>`.
The IDE's LLM reads the instructions and executes the workflow using MCP
tools (`wiki_write`, `wiki_ingest`, `wiki_commit`, etc.).

| Workflow | Instructions from |
|----------|------------------|
| `ingest` | `llm-wiki instruct ingest` |
| `crystallize` | `llm-wiki instruct crystallize` |
| `new` | `llm-wiki instruct new` |
| `commit` | `llm-wiki instruct help` (commit section) |
| `help` | `llm-wiki instruct help` |
| `frontmatter` | `llm-wiki instruct frontmatter` |

The engine provides the playbook, the LLM executes it.

### Why the split?

- **Research** and **lint** are read-only queries the engine can answer
  fully — no LLM judgment needed.
- **Ingest** and **crystallize** require an LLM to read sources, synthesize
  pages, decide structure, write frontmatter. The engine can't do this.
- **New** and **commit** are simple commands, but in ACP context the user
  is asking the IDE's LLM to orchestrate them — the instructions tell it how.

---

## 4. Streaming Sequence

### Engine-executed (research, lint)

Already implemented:

```
→ AgentMessageChunk: "Searching for: {query}..."
→ ToolCall:          wiki_search
→ ToolCallUpdate:    Completed / Failed
→ ToolCall:          wiki_read (if results)
→ ToolCallUpdate:    Completed / Failed
→ AgentMessageChunk: summary
```

### Skill-delegated (ingest, crystallize, new, commit, help, frontmatter)

```
→ AgentMessageChunk: "Here are the instructions for the {workflow} workflow:"
→ AgentMessageChunk: <contents of llm-wiki instruct {workflow}>
```

The IDE's LLM reads these instructions as context and proceeds to execute
the workflow using MCP tools.

---

## 5. Dispatch Table

| Slash command | Type | Action |
|--------------|------|--------|
| `/llm-wiki:research` | Engine-executed | `run_research()` |
| `/llm-wiki:lint` | Engine-executed | `run_lint()` |
| `/llm-wiki:ingest` | Skill-delegated | Stream `instruct ingest` |
| `/llm-wiki:crystallize` | Skill-delegated | Stream `instruct crystallize` |
| `/llm-wiki:new` | Skill-delegated | Stream `instruct new` |
| `/llm-wiki:commit` | Skill-delegated | Stream commit instructions |
| `/llm-wiki:help` | Skill-delegated | Stream `instruct help` |
| `/llm-wiki:frontmatter` | Skill-delegated | Stream `instruct frontmatter` |
| (no prefix) | Fallback | Keyword match → engine-executed or research default |

---

## 6. Implementation Sketch

```rust
impl WikiAgent {
    async fn dispatch(
        &self,
        session_id: &acp::SessionId,
        prompt: &str,
        wiki_entry: Option<&WikiEntry>,
        wiki_name: &str,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let (workflow, text) = Self::parse_dispatch(prompt);

        match workflow {
            // Engine-executed
            "research" => self.run_research(session_id, text, wiki_entry, wiki_name).await,
            "lint" => self.run_lint(session_id, wiki_entry, wiki_name).await,

            // Skill-delegated
            "ingest" | "crystallize" | "new" | "help" | "frontmatter" | "commit" => {
                self.run_skill(session_id, workflow).await
            }

            // Unknown
            _ => {
                self.send_message(
                    session_id,
                    &format!("Unknown workflow: {workflow}. Use /llm-wiki:help for available commands."),
                ).await?;
                Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
            }
        }
    }

    fn parse_dispatch(prompt: &str) -> (&str, &str) {
        if let Some(rest) = prompt.strip_prefix("/llm-wiki:") {
            let (cmd, text) = rest.split_once(' ').unwrap_or((rest, ""));
            (cmd.trim(), text.trim())
        } else {
            // Fallback: keyword matching
            let workflow = Self::dispatch_workflow(prompt);
            (workflow, prompt)
        }
    }

    async fn run_skill(
        &self,
        session_id: &acp::SessionId,
        workflow: &str,
    ) -> Result<acp::PromptResponse, acp::Error> {
        let instructions = crate::cli::extract_workflow(
            crate::cli::INSTRUCTIONS,
            workflow,
        );

        match instructions {
            Some(text) => {
                self.send_message(session_id, &text).await?;
            }
            None => {
                self.send_message(
                    session_id,
                    &format!("No instructions found for workflow: {workflow}"),
                ).await?;
            }
        }

        self.clear_active_run(&session_id.to_string());
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }
}
```

---

## 7. Impact on Existing Code

| File | Change |
|------|--------|
| `src/acp.rs` | Replace `dispatch_workflow` + `catch_unwind` block with `parse_dispatch` + `dispatch` method. Add `run_skill`. Remove ingest/crystallize placeholder strings. |
| `tests/acp.rs` | Update ingest/crystallize tests to verify skill instructions are streamed. Add slash command parsing tests. |
| `docs/specifications/integrations/acp-transport.md` | Update §3.3 dispatch table. Update §3.4 ingest/crystallize section. Close open question #1 (workflow dispatch). |

---

## 8. Open Questions

1. **Should skill-delegated workflows also stream a ToolCall?**
   E.g. `ToolCall("instruct: ingest", Execute)` before the instructions.
   Pro: visible in IDE tool call panel. Con: it's not really a tool call.

2. **Should the prompt text be included in skill instructions?**
   E.g. for `/llm-wiki:ingest the semantic-commit skill`, should the
   instructions include "Target: the semantic-commit skill"? Or let the
   IDE's LLM figure it out from context?

3. **Should `/llm-wiki:research` accept the query in the same message?**
   Currently research uses the full prompt as the query. With slash
   commands: `/llm-wiki:research what is MoE?` → query is "what is MoE?".
   This already works with the proposed parsing.
