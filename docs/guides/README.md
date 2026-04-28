# Guides

User-facing documentation for installing, configuring, and integrating
llm-wiki.

## Getting started

| Guide                                    | Description                                                       |
| ---------------------------------------- | ----------------------------------------------------------------- |
| [getting-started.md](getting-started.md) | End-to-end walkthrough: install → create → write → search → graph |
| [installation.md](installation.md)       | Install llm-wiki (script, cargo, homebrew, asdf)                  |
| [ide-integration.md](ide-integration.md) | Connect to VS Code, Cursor, Windsurf, Zed, Claude Code            |

## Writing and managing content

| Guide                                    | Description                                                                  |
| ---------------------------------------- | ---------------------------------------------------------------------------- |
| [writing-content.md](writing-content.md) | Create and update pages: direct write pattern, wiki_resolve, backlinks       |
| [custom-types.md](custom-types.md)       | Add custom page types with JSON Schema                                       |
| [redaction.md](redaction.md)             | Scrub secrets from page bodies before commit with `redact: true`             |
| [lint.md](lint.md)                       | Catch broken links, orphans, missing fields, stale pages, and unknown types  |

## Configuration and integration

| Guide                                      | Description                                                                  |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| [configuration.md](configuration.md)       | Common settings, per-wiki overrides, troubleshooting                         |
| [multi-wiki.md](multi-wiki.md)             | Manage multiple wikis, cross-wiki search, wiki:// URIs                       |
| [ci-cd.md](ci-cd.md)                       | Schema validation, index rebuild, and ingest in CI pipelines                 |

## Search, graph, and output

| Guide                                      | Description                                                                  |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| [search-ranking.md](search-ranking.md)     | Tune search ranking: status multipliers, custom statuses, per-wiki overrides |
| [llms-format.md](llms-format.md)           | LLM-optimized output: when and how to use `format: "llms"` and `wiki_export` |
| [graph.md](graph.md)                       | Community detection, cross-cluster suggestions, threshold tuning             |

## Operations

| Guide                  | Description                                              |
| ---------------------- | -------------------------------------------------------- |
| [release.md](release.md) | Release process and distribution channels              |
