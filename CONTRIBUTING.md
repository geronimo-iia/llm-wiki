# Contributing to llm-wiki

## Prerequisites

- Rust 1.93.0 — pinned in `.tool-versions`. Install via [asdf](https://asdf-vm.com/) or [rustup](https://rustup.rs/).
- `git`

## Build and Test

```bash
cargo build                      # debug build
cargo build --release            # release build
cargo test                       # all tests
cargo clippy -- -D warnings      # lint — must pass with zero warnings
cargo fmt -- --check             # check formatting
cargo fmt                        # auto-format
```

## Module Architecture

```
src/
├── main.rs          dispatch only — no logic
├── lib.rs           module declarations
├── cli.rs           clap Command enum — all subcommands and flags
├── config.rs        GlobalConfig, WikiConfig, two-level config resolution
├── spaces.rs        multi-wiki management, wiki:// URI resolution
├── git.rs           init, commit, head, diff via git2
├── frontmatter.rs   parse/write YAML frontmatter, validation, scaffolding
├── markdown.rs      page read/write, slug resolution, bundle promotion
├── links.rs         extract links from frontmatter and [[wikilinks]]
├── ingest.rs        validate → git add → commit → index pipeline
├── search.rs        tantivy index, BM25 search, paginated list
├── lint.rs          orphan/stub/section/connection detection, LINT.md
├── graph.rs         petgraph concept graph, Mermaid/DOT rendering
├── server.rs        WikiServer startup, stdio + SSE transport
├── mcp/             MCP tools, resources, prompts
│   ├── mod.rs         ServerHandler impl
│   └── tools.rs       tool definitions and handlers
└── acp.rs           ACP agent, session management, workflow dispatch
```

See [docs/specifications/rust-modules.md](docs/specifications/rust-modules.md)
for the full module responsibility table.

## Adding a Feature

1. Read the relevant spec in `docs/specifications/`.
2. Implement in the correct module per the module map.
3. Write tests in `tests/<module>.rs` using `tempfile::tempdir()` for all
   filesystem operations.
4. Check exit criteria in [docs/tasks.md](docs/tasks.md).
5. Run `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt -- --check`.

## No LLM Dependency Rule

The wiki engine makes zero LLM calls. All intelligence is supplied by an
external LLM that calls the wiki via CLI or MCP. Do not add any LLM client
crate as a dependency.

## Dev Standards

See [docs/implementation/rust.md](docs/implementation/rust.md) for toolchain
details, error handling conventions, testing patterns, and code quality rules.

## Release Process

1. Bump `version` in `Cargo.toml`.
2. Update `CHANGELOG.md`.
3. Commit: `chore: bump version to x.y.z`.
4. Tag: `git tag vx.y.z && git push origin vx.y.z`.

Tagging triggers the release workflow — builds binaries for Linux x86_64,
macOS Intel, and macOS Apple Silicon, creates a GitHub release, and publishes
to crates.io.
