---
title: "Todo"
summary: "Session startup commands for each implementation phase."
read_when:
  - Starting a new implementation session
  - Picking up a phase after a break
status: active
last_updated: "2025-07-15"
---

# Todo

Start a new chat session for each phase. Paste the block below, then
send it. Do not mix phases in the same session.

---

## Phase 1 — Foundation: Schema + Config + Spaces

```
@docs/prompts/phase-1.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/commands/configuration.md
@docs/specifications/commands/spaces.md
@docs/specifications/commands/init.md
@docs/specifications/commands/cli.md
@docs/specifications/core/repository-layout.md

Implement Phase 1 following the prompt and task list above. Start with src/config.rs.
```

---

## Phase 2 — Core Write Loop: Ingest + Page Creation

```
@docs/prompts/phase-2.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/core/page-content.md
@docs/specifications/core/repository-layout.md
@docs/specifications/core/frontmatter-authoring.md
@docs/specifications/pipelines/ingest.md
@docs/specifications/pipelines/asset-ingest.md
@docs/specifications/commands/page-creation.md
@docs/specifications/commands/cli.md

Implement Phase 2 following the prompt and task list above. Start with src/frontmatter.rs.
```

---

## Phase 3 — Frontmatter Validation + Type Taxonomy

```
@docs/prompts/phase-3.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/specifications/core/page-content.md
@docs/specifications/core/frontmatter-authoring.md
@docs/specifications/core/source-classification.md
@docs/specifications/commands/configuration.md
@docs/specifications/commands/instruct.md
@docs/specifications/llm/session-bootstrap.md
@docs/specifications/llm/backlink-quality.md
@docs/specifications/pipelines/ingest.md

Implement Phase 3 following the prompt and task list above. Start with the validation additions to src/frontmatter.rs.
```

---

## Phase 4 — Search + Read + Index

```
@docs/prompts/phase-4.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/commands/search.md
@docs/specifications/commands/read.md
@docs/specifications/commands/list.md
@docs/specifications/commands/index.md
@docs/specifications/commands/cli.md
@docs/specifications/core/repository-layout.md

Implement Phase 4 following the prompt and task list above. Start with src/search.rs.
```

---

## Phase 5 — Lint + Graph

```
@docs/prompts/phase-5.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/commands/lint.md
@docs/specifications/commands/graph.md
@docs/specifications/commands/cli.md
@docs/specifications/llm/backlink-quality.md
@docs/specifications/core/source-classification.md

Implement Phase 5 following the prompt and task list above. Start with src/links.rs.
```

---

## Phase 6 — MCP Server + Session Bootstrap

```
@docs/prompts/phase-6.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/features.md
@docs/specifications/commands/serve.md
@docs/specifications/commands/instruct.md
@docs/specifications/llm/session-bootstrap.md
@docs/specifications/llm/backlink-quality.md
@docs/specifications/integrations/acp-transport.md

Implement Phase 6 following the prompt and task list above. Start with src/server.rs.
```

---

## Phase 7 — ACP Transport

```
@docs/prompts/phase-7.md
@docs/tasks.md
@docs/specifications/rust-modules.md
@docs/implementation/rust.md
@docs/specifications/integrations/acp-transport.md

Implement Phase 7 following the prompt and task list above. Start with Cargo.toml, then src/acp.rs.
```

---

## Phase 8 — Claude Plugin

```
@docs/prompts/phase-8.md
@docs/tasks.md
@docs/specifications/integrations/claude-plugin.md

Implement Phase 8 following the prompt and task list above. Start with .claude-plugin/plugin.json.
```

---

## Phase 9 — Documentation

```
@docs/prompts/phase-9.md
@docs/tasks.md
@docs/specifications/overview.md
@docs/specifications/features.md
@docs/specifications/commands/cli.md
@docs/specifications/integrations/mcp-clients.md
@docs/specifications/integrations/claude-plugin.md
@docs/implementation/rust.md

Implement Phase 9 following the prompt and task list above. Start with README.md.
```
