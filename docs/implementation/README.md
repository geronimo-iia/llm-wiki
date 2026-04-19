# Implementation

Implementation notes and design references for llm-wiki. These are not
specifications — see [specifications/](../specifications/README.md) for
the design.

## Architecture

| Document                                 | Description                                                |
| ---------------------------------------- | ---------------------------------------------------------- |
| [engine.md](engine.md)                   | Top-level Engine struct, EngineManager, change propagation |
| [manager-pattern.md](manager-pattern.md) | Shared pattern: detect, refresh, cascade                   |

## Modules

| Document                                       | Description                                              |
| ---------------------------------------------- | -------------------------------------------------------- |
| [type-registry.md](type-registry.md)                     | SpaceTypeRegistry, validators, caching, change detection |
| [index-schema-building.md](index-schema-building.md)     | Deriving tantivy schema from type schemas                |
| [index-manager.md](index-manager.md)                     | SpaceIndexManager, rebuild, staleness, recovery          |
| [tantivy.md](tantivy.md)                       | Dynamic schema, TopDocs, index writer, tokenizer         |
| [graph-builder.md](graph-builder.md)           | Petgraph from index, typed edges, Mermaid/DOT rendering  |
| [frontmatter-parser.md](frontmatter-parser.md) | YAML extraction, untyped parsing, body splitting         |
| [slug.md](slug.md)                             | Slug and WikiUri types, resolution, URI parsing          |
| [git.md](git.md)                               | git2 wrappers: init, commit, diff, HEAD                  |
| [config-loader.md](config-loader.md)           | Two-level config, resolution order, get/set by key       |

## Servers

| Document                       | Description                                                      |
| ------------------------------ | ---------------------------------------------------------------- |
| [mcp-server.md](mcp-server.md) | rmcp setup, tool registration, resource namespacing, stdio + SSE |
| [acp-server.md](acp-server.md) | WikiAgent, session management, streaming, prompt dispatch        |

## CLI and Toolchain

| Document           | Description                                            |
| ------------------ | ------------------------------------------------------ |
| [cli.md](cli.md)   | Clap derive structure, subcommand hierarchy            |
| [rust.md](rust.md) | Toolchain, dependencies, code quality, release process |

## SDK References

| Document                 | Description                       |
| ------------------------ | --------------------------------- |
| [acp-sdk.md](acp-sdk.md) | agent-client-protocol crate notes |
