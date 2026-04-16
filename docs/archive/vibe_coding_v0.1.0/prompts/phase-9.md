# Phase 9 — Documentation

## Context

Phases 1–8 are complete. The tool is fully functional and the plugin
works. You are now rewriting the user-facing documentation to reflect
the current design.

## Rules

- Rewrite only the three files listed below. Nothing else.
- Do not modify any file under `docs/specifications/`, `docs/implementation/`,
  `src/`, or `.claude-plugin/`.
- Base all content on what the tool actually does — read the specs and
  the implementation, do not invent features.
- No marketing language. Plain, direct prose.

## Specs to read before starting

Read these files to understand what the tool does:

- `docs/specifications/overview.md`
- `docs/specifications/features.md`
- `docs/specifications/commands/cli.md`
- `docs/specifications/integrations/mcp-clients.md`
- `docs/specifications/integrations/claude-plugin.md`
- `docs/implementation/rust.md`

## Tasks

### 1. `README.md`

Rewrite from scratch. Structure:

1. One-paragraph description of what llm-wiki is and the problem it solves
2. Quick start — `wiki init`, `wiki serve`, connect an MCP client
3. Core concepts — wiki, page, slug, `wiki://` URI, ingest, search
4. MCP client setup — config snippets for Cursor, VS Code, Claude Code
5. CLI reference — link to `docs/specifications/commands/cli.md`
6. How it works — the DKR model in 3–4 sentences

No badges, no feature matrices, no roadmap in the README.

### 2. `CONTRIBUTING.md`

Rewrite from scratch. Structure:

1. Prerequisites — Rust 1.93.0 (pinned in `.tool-versions`), `asdf` or `rustup`
2. Build and test — `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`
3. Module architecture — brief description of each module, link to
   `docs/specifications/rust-modules.md`
4. Adding a feature — read the spec first, implement in the right module,
   write tests in `tests/<module>.rs`, check exit criteria in `docs/tasks.md`
5. Release process — from `docs/implementation/rust.md` §Release Process

### 3. `CHANGELOG.md`

Rewrite from scratch as a feature-oriented changelog. Structure:

```
# Changelog

## [Unreleased]

### Added
- (list features as they land)

## [0.1.0] — TBD

### Added
- wiki init — initialize a new wiki repository
- wiki ingest — validate, commit, and index pages
- wiki new page / section — create scaffolded pages
- wiki search — full-text BM25 search with PageRef return type
- wiki read — fetch page content by slug or wiki:// URI
- wiki list — paginated page enumeration with type/status filters
- wiki index rebuild / status — tantivy index management
- wiki lint — structural audit with LINT.md output
- wiki graph — concept graph in Mermaid or DOT format
- wiki serve — MCP server (stdio + SSE) for Claude Code and other agents
- wiki instruct — embedded workflow instructions for LLMs
- wiki config — two-level configuration (global + per-wiki)
- wiki spaces — multi-wiki management
- Claude Code plugin with slash commands
```

Do not list git commits. List capabilities.

## Exit criteria

Before marking Phase 9 complete:

- [ ] A new contributor can read `README.md` and run `wiki init` within
      5 minutes without reading any other file
- [ ] `CONTRIBUTING.md` references `docs/implementation/rust.md` for
      dev standards
- [ ] `CHANGELOG.md` describes what the tool can do, not what changed
      in each commit
