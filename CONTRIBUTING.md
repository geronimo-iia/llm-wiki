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

## Documentation

| Area                    | Location                                              |
| ----------------------- | ----------------------------------------------------- |
| Specifications          | [docs/specifications/](docs/specifications/README.md) |
| Implementation notes    | [docs/implementation/](docs/implementation/README.md) |
| Architectural decisions | [docs/decisions/](docs/decisions/README.md)           |


## Adding a Feature

1. Read the relevant spec in `docs/specifications/`.
2. Implement in the correct module — see
   [docs/implementation/](docs/implementation/README.md) for the module map.
3. Write tests in `tests/<module>.rs` using `tempfile::tempdir()` for all
   filesystem operations.
4. Run `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt -- --check`.

### Adding an MCP tool

See [docs/implementation/mcp-tool-pattern.md](docs/implementation/mcp-tool-pattern.md).

## No LLM Dependency Rule

The wiki engine makes zero LLM calls. All intelligence is supplied by an
external LLM that calls the wiki via CLI or MCP. Do not add any LLM client
crate as a dependency.

## Dev Standards

See [docs/implementation/rust.md](docs/implementation/rust.md) for toolchain
details, error handling conventions, testing patterns, and code quality rules.

## Release Process

See [docs/guides/release.md](docs/guides/release.md) for the full checklist, branch strategy, and tagging instructions.
