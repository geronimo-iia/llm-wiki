# Implementation

Implementation notes for llm-wiki — non-obvious architecture, patterns,
and external crate usage. These lag behind the code; treat as orientation,
not ground truth. For design contracts see [specifications/](../specifications/README.md).

## Architecture

| Document                                 | Description                                       |
| ---------------------------------------- | ------------------------------------------------- |
| [engine.md](engine.md)                   | EngineState, WikiEngine, space mounting           |
| [manager-pattern.md](manager-pattern.md) | Shared pattern: detect, refresh, cascade          |

## Modules

| Document                                                 | Description                                         |
| -------------------------------------------------------- | --------------------------------------------------- |
| [type-registry.md](type-registry.md)                     | SpaceTypeRegistry, validators, caching              |
| [schema-change-detection.md](schema-change-detection.md) | Schema hash, staleness, per-wiki registry           |
| [index-manager.md](index-manager.md)                     | SpaceIndexManager, rebuild, staleness, recovery     |
| [tantivy.md](tantivy.md)                                 | Dynamic schema, TopDocs, index writer, tokenizer    |

## MCP

| Document                                         | Description                        |
| ------------------------------------------------ | ---------------------------------- |
| [mcp-tool-pattern.md](mcp-tool-pattern.md)       | Patterns for adding new MCP tools  |

## Toolchain

| Document           | Description                                            |
| ------------------ | ------------------------------------------------------ |
| [rust.md](rust.md) | Toolchain, dependencies, code quality, release process |
