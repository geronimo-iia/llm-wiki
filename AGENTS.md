---
title: "Agent Context"
summary: "Orientation for AI agents and contributors working on llm-wiki."
last_updated: "2026-08-04"
---

# Agent Context

Quick orientation for AI agents starting a fresh session. Not a spec — read
`docs/implementation/` for deep dives, `docs/guides/release.md` for release process.

## Codebase map

| Path | Purpose |
|---|---|
| `src/engine.rs` | `WikiEngine` + `EngineState` + `SpaceContext`; `mount_space` wires everything at startup |
| `src/graph.rs` | `WikiGraphCache` enum, `get_or_build_graph`, community helpers |
| `src/ops/` | One file per user-facing operation; called by both CLI (`main.rs`) and MCP (`handlers.rs`) |
| `src/mcp/handlers.rs` | MCP tool dispatch; thin wrappers over `ops::*` |
| `src/main.rs` | CLI dispatch; thin wrappers over `ops::*` |
| `src/index_manager.rs` | Tantivy index lifecycle; `last_commit()`, `generation()`, `reload_reader()` |
| `tests/` | Rust integration tests (`cargo test`) |
| `tests-integration/` | Python end-to-end suite (`make -C tests-integration test-engine`) |

## Key types

- `WikiEngine` — `Arc<RwLock<EngineState>>`; cheap to clone, share across tasks
- `SpaceContext` — all runtime state for one mounted wiki; lives inside `EngineState.spaces`
- `WikiGraphCache` — `NoSnapshot(GenerationCache<WikiGraph>)` or `WithSnapshot(GraphState<WikiGraph>)`
- `SpaceIndexManager` — tantivy index lifecycle; `last_commit()` reads `state.toml`

## CLI vs MCP lifetime

CLI is **short-lived** (fresh process per invocation). MCP server is **long-lived**
(generation counter accumulates). Design decisions that depend on process lifetime
must account for both. See `docs/invariants.md`.

## ops layer is the single source of truth

Both `main.rs` and `handlers.rs` call `ops::*`. Side effects (graph cache refresh,
logging) belong in `ops::*`, not in the callers. Never add `graph_cache.rebuild()`
in `main.rs` or `handlers.rs` — `ops::index_rebuild` owns it.

## Graph snapshot key

`WithSnapshot` uses `index_manager.last_commit()` as the snapshot key — the git HEAD
SHA written to `state.toml` at each index rebuild. It is stable across process restarts
and changes on every `index rebuild`. Do **not** use `generation()` as a snapshot key —
it resets to 0 on every process start. See `docs/invariants.md#snapshot-key-stability`.

## Test patterns

- **Three-engine pattern** (snapshot regression): drop and recreate `WikiEngine::build`
  against the same tmpdir to simulate separate process lifetimes
- **CLI binary tests**: `Command::new(env!("CARGO_BIN_EXE_llm-wiki"))` — see `tests/cli.rs`
- **ops tests**: `tests/ops/helpers.rs::setup_wiki` — 2 pages, git committed, config returned
- **Snapshot tests**: set `graph.snapshot = false` in test config to avoid `.snap.lz4` files
  in tmpdir; or use a real tmpdir and let rotation prune them

## CI

| Workflow | Trigger | What it runs |
|---|---|---|
| `ci.yml` | push to `main`, `feat/*`, `dev/*`; PR to `main` | `cargo fmt`, `cargo clippy`, `cargo test` |
| `integration.yml` | push/PR to `main`; manual dispatch | Python suite in `tests-integration/` |
| `release.yml` | tag `v*` | cross-compile 5 targets, GitHub release, crates.io publish |

`fix/*` branches: CI runs only when the PR opens against `main`, not on push.

## Release checklist

See `docs/guides/release.md`. Short version:
1. `cargo test` + `cargo fmt --check` + `cargo clippy` + `cargo build --release --locked`
2. `cargo doc --no-deps` zero warnings
3. CHANGELOG `[Unreleased]` → `[x.y.z] — YYYY-MM-DD`
4. Bump `Cargo.toml` version, commit, tag `vx.y.z`, push tag
