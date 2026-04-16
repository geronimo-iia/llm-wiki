---
title: "Roadmap"
summary: "Phase-by-phase delivery plan for llm-wiki. Each phase is independently shippable and unlocks a concrete capability."
read_when:
  - Understanding the delivery order and scope of each phase
  - Deciding what to implement next
  - Reviewing overall project direction
status: active
last_updated: "2025-07-15"
---

# Roadmap

Full task list with checkboxes: [tasks.md](tasks.md)
Completed phases: [archive/](archive/)

---

## Current Status

All phases pending. Implementation starts from Phase 1.

---

## Target Architecture

```
src/
├── main.rs         # CLI entry point — dispatch only
├── lib.rs          # module declarations
├── cli.rs          # clap Command enum — all subcommands and flags
├── config.rs       # GlobalConfig, WikiConfig, ValidationConfig, two-level resolution
├── spaces.rs       # Spaces, WikiEntry, resolve_name(), resolve_uri()
├── git.rs          # init_repo(), commit(), current_head(), diff_last()
├── frontmatter.rs  # parse/write, scaffold, validate, generate_minimal
├── markdown.rs     # read_page, list_assets, read_asset, promote_to_bundle, slug helpers
├── links.rs        # extract_links()
├── ingest.rs       # IngestOptions, validate → git add → commit → index
├── search.rs       # PageRef, PageSummary, PageList, tantivy index, search(), list(),
│                   # IndexStatus, IndexReport, state.toml
├── lint.rs         # LintReport, MissingConnection, all lint checks, LINT.md write
├── graph.rs        # build_graph(), render_mermaid/dot, GraphReport, in_degree()
├── server.rs       # WikiServer, startup, stdio + SSE transport wiring
├── mcp.rs          # all MCP tools, resources, prompts
└── acp.rs          # WikiAgent, AcpSession, workflow dispatch
```

---

## Phase 1 — Foundation: Schema + Config + Spaces

**Goal:** Config and spaces load correctly. `wiki init`, `wiki config`, and
`wiki spaces` work end-to-end.

**Modules:** `config.rs`, `spaces.rs`, `git.rs`, `cli.rs`, `mcp.rs`

**Deliverable:** `cargo test` green. `wiki init` creates a wiki and registers it.

**Tasks:** [tasks.md § Phase 1](tasks.md#phase-1--foundation-schema--config--spaces)

---

## Phase 2 — Core Write Loop: Ingest + Page Creation

**Goal:** `wiki ingest <path>` validates, commits, and indexes files already
in the wiki tree. `wiki new page/section` creates scaffolded pages.

**Modules:** `frontmatter.rs`, `markdown.rs`, `ingest.rs`, `cli.rs`, `mcp.rs`

**Deliverable:** Author writes a file into the wiki tree, `wiki ingest`
validates, commits, and indexes it.

**Tasks:** [tasks.md § Phase 2](tasks.md#phase-2--core-write-loop-ingest--page-creation)

---

## Phase 3 — Frontmatter Validation + Type Taxonomy

**Goal:** Engine validates frontmatter on ingest. Unified type taxonomy
enforced. `validation.type_strictness` respected. Frontmatter authoring
guide embedded in instructions.

**Modules:** `frontmatter.rs`, `ingest.rs`, `src/instructions.md`

**Deliverable:** `wiki ingest` validates frontmatter and warns on missing
required fields or deprecated `source-summary` type.

**Tasks:** [tasks.md § Phase 3](tasks.md#phase-3--frontmatter-validation--type-taxonomy)

---

## Phase 4 — Search + Read + Index

**Goal:** `wiki search`, `wiki read`, `wiki list`, `wiki index` work.
Unified `PageRef` return type. `state.toml` written on rebuild.

**Modules:** `search.rs`, `cli.rs`, `mcp.rs`

**Deliverable:** `wiki search "MoE scaling"` returns `Vec<PageRef>` with
`wiki://` URIs. `wiki read wiki://test/concepts/foo` returns full page content.

**Tasks:** [tasks.md § Phase 4](tasks.md#phase-4--search--read--index)

---

## Phase 5 — Lint + Graph

**Goal:** `wiki lint` produces a `LintReport` with all five checks and
commits `LINT.md` at the repository root. `wiki graph` emits Mermaid or DOT.

**Modules:** `links.rs`, `lint.rs`, `graph.rs`, `cli.rs`, `mcp.rs`

**Deliverable:** `wiki lint` writes `LINT.md`. `wiki graph` outputs Mermaid
to stdout.

**Tasks:** [tasks.md § Phase 5](tasks.md#phase-5--lint--graph)

---

## Phase 6 — MCP Server + Session Bootstrap

**Goal:** `wiki serve` works with all registered wikis mounted. All MCP
tools, resources, and prompts live. Session bootstrap complete.

**Modules:** `server.rs`, `mcp.rs`, `src/instructions.md`, `cli.rs`

**Deliverable:** Claude Code can use all wiki tools via MCP. Session
bootstrap orients the LLM from the wiki's current state.

**Tasks:** [tasks.md § Phase 6](tasks.md#phase-6--mcp-server--session-bootstrap)

---

## Phase 7 — ACP Transport

**Goal:** `wiki serve --acp` works as a native Zed / VS Code agent.

**Modules:** `acp.rs`, `server.rs`, `Cargo.toml`

**Deliverable:** `wiki serve --acp` starts. Zed agent panel connects and
streams ingest/research workflows.

**Tasks:** [tasks.md § Phase 7](tasks.md#phase-7--acp-transport)

---

## Phase 8 — Claude Plugin

**Goal:** `.claude-plugin/` is complete and installable. All slash commands
work.

**Deliverable:** `claude plugin add /path/to/llm-wiki` → `/llm-wiki:ingest`
works.

**Tasks:** [tasks.md § Phase 8](tasks.md#phase-8--claude-plugin)

---

## Phase 9 — Documentation

**Goal:** Project documentation reflects the current design and is useful
to a new contributor or user arriving cold.

**Deliverable:** A new contributor can read `README.md`, understand what
llm-wiki does, run it, and know where to start contributing.

**Tasks:** [tasks.md § Phase 9](tasks.md#phase-9--documentation)

---

## What Each Phase Unlocks

| After phase | You can… |
|-------------|----------|
| 1 | Initialize wikis, manage spaces and config |
| 2 | Validate, commit, and index files in the wiki tree; create pages and sections |
| 3 | Frontmatter validation on ingest, unified type taxonomy enforced, authoring guide in instructions |
| 4 | Search, read pages and assets, manage the index |
| 5 | Audit wiki structure, visualize concept graph |
| 6 | Use the wiki from Claude Code with full MCP access, crystallize sessions, session bootstrap |
| 7 | `wiki serve --acp` — native Zed / VS Code streaming agent |
| 8 | `/llm-wiki:ingest` and `/llm-wiki:crystallize` as one-command slash workflows |
| 9 | README, CONTRIBUTING, and CHANGELOG reflect the current design |
