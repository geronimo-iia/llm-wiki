# Plan Phase 1 Implementation

## Context

The llm-wiki specifications are complete (all `ready` in
`docs/specifications/README.md`). Implementation docs cover the
architecture and per-module design. The current codebase needs to move
to `code-ref/` and a fresh `src/` built from the specs.

## Read first

Read these in order before planning:

1. `docs/specifications/README.md` — full spec index
2. `docs/implementation/README.md` — full implementation doc index
3. `docs/implementation/engine.md` — Engine, EngineManager, startup
4. `docs/implementation/rust.md` — project layout, dependencies, modules
5. `docs/roadmap.md` — Phase 1 deliverables

Then skim the implementation docs for each module you'll build:
- `type-registry.md`, `index-manager.md`, `tantivy.md`
- `slug.md`, `frontmatter-parser.md`, `config-loader.md`
- `git.md`, `graph-builder.md`
- `mcp-server.md`, `acp-server.md`, `cli.md`
- `manager-pattern.md`

And the existing code in `src/` (soon `code-ref/`) — check each
implementation doc's "Existing Code" section for what's reusable.

## Your Task

Update `docs/roadmap.md` Phase 1 with a detailed implementation plan.
Break it into ordered steps where each step produces a compilable,
testable increment. No step should be larger than one session of work.

## Constraints

### Build order matters

Modules depend on each other. Build bottom-up:

```
slug.rs (no deps)
  -> config.rs (uses slug for URI resolution)
  -> frontmatter.rs (no deps, uses frontmatter crate)
  -> git.rs (no deps)
  -> type_registry.rs (uses config, frontmatter, jsonschema)
  -> index_schema.rs (uses type_registry)
  -> index_manager.rs (uses index_schema, tantivy, git)
  -> search.rs (uses index_manager, tantivy)
  -> ingest.rs (uses type_registry, index_manager, git)
  -> graph.rs (uses index_manager, type_registry, petgraph)
  -> markdown.rs (uses slug, frontmatter — page I/O only)
  -> spaces.rs (uses config, git — space management only)
  -> engine.rs (composes everything)
  -> cli.rs (uses engine)
  -> mcp/ (uses engine)
  -> acp.rs (uses engine)
  -> server.rs (wires transports)
```

### Each step must

- Compile (`cargo check`)
- Have tests (`cargo test`)
- Be committable with a meaningful message

### Reuse from code-ref/

For each module, check the "Existing Code" table in its implementation
doc. Pull reusable code directly — don't rewrite what works. Note what
you pulled and what you changed.

### What NOT to implement in Phase 1

- JSON Schema validation (Phase 2)
- `x-index-aliases` resolution (Phase 2)
- `x-graph-edges` typed edges (Phase 3)
- Skill registry features (Phase 4)
- Hot reload / file watcher (future)

Phase 1 uses the base frontmatter fields only (`title`, `type`,
`summary`, `status`, `tags`, etc.) with hardcoded field-to-index
mapping. The dynamic type system comes in Phase 2.

### What MUST work at the end of Phase 1

- `llm-wiki spaces create/list/remove/set-default`
- `llm-wiki config get/set/list`
- `llm-wiki content read/write/new/commit`
- `llm-wiki search` with `--type` filter and `--format`
- `llm-wiki list` with `--type`, `--status`, `--format`
- `llm-wiki ingest` with `--format`
- `llm-wiki graph` with `--format`, `--root`, `--depth`, `--type`
- `llm-wiki index rebuild/status`
- `llm-wiki serve` (stdio + SSE)
- `llm-wiki serve --acp`
- All 15 MCP tools working
- Integration tests for each tool

## Output

Update `docs/roadmap.md` Phase 1 with numbered steps. Each step:

```
### Step N: <what>

Modules: <files created or modified>
Pulls from: <code-ref/ files reused>
Tests: <what's tested>
Commit: <message>
```

Keep the existing Phase 2-4 sections unchanged.
