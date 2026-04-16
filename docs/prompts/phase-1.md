# Phase 1 — Foundation: Schema + Config + Spaces

## Context

You are implementing Phase 1 of llm-wiki, a Rust CLI and MCP server.
The codebase is a fresh start. `src-beta/` contains an outdated prior
implementation — do not read it, do not copy from it.

## Rules

- Implement only what is listed in the tasks below. Nothing else.
- Every type, function name, and signature must match the spec exactly.
- Do not add fields, methods, or behaviour not described in the specs.
- Do not modify any file under `docs/`.
- After each module is complete, run `cargo test` and fix errors before
  moving to the next module.

## Specs to read before starting

Read these files in full before writing any code:

- `docs/specifications/rust-modules.md`
- `docs/implementation/rust.md`
- `docs/specifications/commands/configuration.md`
- `docs/specifications/commands/spaces.md`
- `docs/specifications/commands/init.md`
- `docs/specifications/commands/cli.md`
- `docs/specifications/core/repository-layout.md`

## Tasks

Work through these in order. Check off each task in `docs/tasks.md` as
you complete it.

### 1. `src/config.rs`

Implement all types and functions listed under `### config.rs` in
`docs/tasks.md` Phase 1. The full struct definitions and field names are
in `docs/specifications/commands/configuration.md`.

### 2. `src/spaces.rs`

Implement all functions listed under `### spaces.rs` in `docs/tasks.md`
Phase 1. Signatures are in `docs/specifications/commands/spaces.md` and
`docs/specifications/commands/read.md` (for `resolve_uri`).

### 3. `src/git.rs`

Implement all functions listed under `### git.rs` in `docs/tasks.md`
Phase 1.

### 4. `src/lib.rs`

Declare all modules: `config`, `spaces`, `git`, `cli`.

### 5. `src/cli.rs`

Implement the clap `Commands` enum for Phase 1 commands only:
`init`, `config`, `spaces`. See `docs/specifications/commands/cli.md`
for exact flags and subcommands.

### 6. `src/main.rs`

Dispatch Phase 1 commands to their engine functions. No logic here —
dispatch only.

### 7. `src/mcp.rs` and `src/server.rs`

Stub only — empty `WikiServer` struct and the five Phase 1 MCP tools
(`wiki_init`, `wiki_config`, `wiki_spaces_list`, `wiki_spaces_remove`,
`wiki_spaces_set_default`) wired to the engine functions.

### 8. `tests/config.rs`

Write all tests listed under `### tests/config.rs` in `docs/tasks.md`
Phase 1. Use `tempfile::tempdir()` for all filesystem operations.

### 9. `tests/spaces.rs`

Write all tests listed under `### tests/spaces.rs` in `docs/tasks.md`
Phase 1.

### 10. `tests/git.rs`

Write all tests listed under `### tests/git.rs` in `docs/tasks.md`
Phase 1.

## Exit criteria

Before marking Phase 1 complete:

- [x] `cargo fmt -- --check` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo test` passes
- [x] `wiki init ~/wikis/test --name test` creates the directory structure
      and registers the wiki in `~/.wiki/config.toml`
- [x] `wiki config list` prints resolved config
- [x] `wiki spaces list` lists registered wikis
