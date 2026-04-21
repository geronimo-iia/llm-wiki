---
title: "Rust Implementation Guide"
summary: "Project-specific Rust standards for llm-wiki — toolchain, targets, dependencies, code quality, and release process."
read_when:
  - Setting up the llm-wiki development environment
  - Adding a new dependency or module
  - Preparing a release
  - Understanding project-specific conventions
status: active
last_updated: "2025-07-21"
---

# Rust Implementation Guide

## Project Layout

```
llm-wiki/
├── Cargo.toml
├── Cargo.lock
├── clippy.toml
├── rustfmt.toml
├── .tool-versions
├── src/
│   ├── main.rs              # CLI dispatch only — parse args, call ops, format output
│   ├── lib.rs               # module declarations
│   ├── cli.rs               # clap subcommand hierarchy
│   ├── ops.rs               # shared business logic (CLI + MCP call this)
│   ├── engine.rs            # EngineState, WikiEngine
│   ├── config.rs            # GlobalConfig, WikiConfig, resolution
│   ├── slug.rs              # Slug, WikiUri types and resolution
│   ├── frontmatter.rs       # YAML extraction, BTreeMap parsing
│   ├── type_registry.rs     # SpaceTypeRegistry, EdgeDecl, schema hashing
│   ├── default_schemas.rs   # embedded default schemas (include_str!)
│   ├── space_builder.rs     # build SpaceTypeRegistry + IndexSchema from schemas/
│   ├── index_manager.rs     # SpaceIndexManager, rebuild, staleness
│   ├── index_schema.rs      # IndexSchema from type registry
│   ├── search.rs            # tantivy search + list queries
│   ├── ingest.rs            # ingest pipeline
│   ├── graph.rs             # petgraph builder + Mermaid/DOT rendering
│   ├── links.rs             # [[wiki-link]] extraction
│   ├── markdown.rs          # page I/O (read, write, create)
│   ├── spaces.rs            # space management (register, remove)
│   ├── git.rs               # git2 wrappers (init, commit, diff)
│   ├── server.rs            # serve command, transport startup
│   ├── mcp/
│   │   ├── mod.rs           # McpServer, ServerHandler impl
│   │   ├── tools.rs         # tool definitions + dispatch
│   │   ├── handlers.rs      # MCP-specific: parse args, call ops, wrap result
│   │   └── helpers.rs       # arg helpers, ToolResult, collect_page_uris
│   └── acp/
│       ├── mod.rs           # AcpSession, dispatch_workflow, make_tool_id
│       ├── helpers.rs       # send_text, send_tool_call, send_tool_result, resolve_wiki_name
│       ├── research.rs      # step_search, step_read, step_report_results, run_research
│       └── server.rs        # serve_acp (Agent builder wiring)
├── tests/                   # integration tests
├── code-ref/                # previous implementation (reference)
└── docs/
```

See [implementation/](../implementation/README.md) for per-module
design docs.

```
rust 1.93.0   (pinned in .tool-versions)
edition 2021
```

Always use the pinned version. Update deliberately — check for breaking
changes in tantivy, rmcp, and git2 before bumping.


## Supported Targets

| Target                     | Platform            | Release binary |
| -------------------------- | ------------------- | -------------- |
| `x86_64-unknown-linux-gnu` | Linux x86_64        | yes            |
| `x86_64-apple-darwin`      | macOS Intel         | yes            |
| `aarch64-apple-darwin`     | macOS Apple Silicon | yes            |
| `x86_64-pc-windows-msvc`   | Windows x86_64      | planned        |

Windows support is planned but not yet in the release matrix. CI runs on
`ubuntu-latest` only. Cross-platform issues surface at release time via the
matrix build.


## Dependencies

### Runtime

| Crate                                 | Version                                                  | Purpose                          |
| ------------------------------------- | -------------------------------------------------------- | -------------------------------- |
| `clap`                                | 4 (derive)                                               | CLI argument parsing             |
| `anyhow`                              | 1                                                        | Application-level error handling |
| `tracing` + `tracing-subscriber`      | 0.1 / 0.3                                                | Structured logging               |
| `tokio`                               | 1 (full)                                                 | Async runtime                    |
| `serde` + `serde_json` + `serde_yaml` | 1 / 1 / 0.9                                              | Serialization                    |
| `toml`                                | 1.1                                                      | Config file parsing              |
| `jsonschema`                          | 0.46                                                     | JSON Schema validation           |
| `sha2` + `hex`                        | 0.11 / 0.4                                               | Schema hashing                   |
| `tantivy`                             | 0.26                                                     | Full-text search index           |
| `petgraph`                            | 0.8                                                      | Concept graph                    |
| `walkdir`                             | 2                                                        | Filesystem traversal             |
| `chrono`                              | 0.4 (clock, std)                                         | Date/time                        |
| `git2`                                | 0.20                                                     | Git operations                   |
| `rmcp`                                | 1 (server, transport-io, transport-streamable-http-server) | MCP server                       |
| `axum`                                | 0.8                                                      | HTTP server for Streamable HTTP  |
| `agent-client-protocol`               | 0.11                                                     | ACP agent (builder pattern)      |

### Dev

| Crate      | Version | Purpose                   |
| ---------- | ------- | ------------------------- |
| `tempfile` | 3       | Isolated filesystem tests |

### Adding dependencies

- Prefer the crates already in use for similar concerns
- Minimise transitive dependencies -- check `cargo tree` before adding
- Never add a dependency for something in std or already covered above


## Code Quality

### rustfmt.toml

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
use_small_heuristics = "Default"
```

Note: `imports_granularity = "Crate"` and `group_imports = "StdExternalCrate"`
require nightly rustfmt. Omitted for stable toolchain compatibility.

### clippy.toml

```toml
avoid-breaking-exported-api = false
```

### Commands

```bash
cargo fmt                        # format
cargo fmt -- --check             # check formatting (CI)
cargo clippy -- -D warnings      # lint, fail on warnings (CI)
cargo check                      # fast type check
cargo build                      # debug build
cargo build --release            # release build
cargo test                       # all tests
cargo test <name>                # specific test
cargo test -- --nocapture        # with stdout
```


## Error Handling

- `anyhow::Result` for all public functions and CLI dispatch
- `thiserror` for typed errors on module boundaries where callers need to
  match on error kind (e.g. `ingest.rs`, `search.rs`)
- `panic!` only for programmer errors (invariant violations), never for
  user input or I/O failures


## Testing

### Unit tests

In-module, alongside the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_resolution() { ... }
}
```

### Integration tests

In `tests/`, one file per module under test. Use `tempfile::tempdir()` for
all filesystem operations — never write to real paths:

```rust
#[test]
fn test_ingest_commits() {
    let dir = tempfile::tempdir().unwrap();
    // ... set up wiki in dir.path()
}
```

### Function injection for I/O

For functions that call external systems (git, HTTP), accept a function
pointer rather than calling directly. Keeps tests fast and hermetic:

```rust
pub type Fetcher = fn(&str) -> anyhow::Result<String>;

pub fn load(url: &str, fetch: Fetcher) -> anyhow::Result<String> {
    fetch(url)
}
```


## Release Process

### Cargo.toml release profile

```toml
[profile.release]
opt-level     = 3
lto           = true
codegen-units = 1
strip         = true
```

### binstall metadata

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/v{ version }/{ target }.tar.gz"
bin-dir = "llm-wiki"
```

### Steps

1. Bump `version` in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit: `chore: bump version to x.y.z`
4. Tag: `git tag vx.y.z && git push origin vx.y.z`

Tagging triggers the release workflow — builds binaries for all three
targets, creates a GitHub release, and publishes to crates.io.

### CI workflow (`.github/workflows/ci.yml`)

Runs on every push and PR to `main`:

- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo audit`
- `cargo build`
- `cargo test`

### Release workflow (`.github/workflows/release.yml`)

Triggered by `v*` tags. Builds release binaries for:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Packages each as `<target>.tar.gz`, creates a GitHub release, then
publishes to crates.io via `CARGO_REGISTRY_TOKEN`.


## Windows

Windows (`x86_64-pc-windows-msvc`) is a planned target. Known concerns:

- `git2` links against `libgit2` — verify static linking on MSVC
- Path separators — use `std::path::Path` throughout, never string
  concatenation for paths
- Line endings -- the engine normalises CRLF to LF on write

Add to the release matrix when the above are verified.
