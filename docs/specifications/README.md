---
title: "Specifications Index"
summary: "Index of all llm-wiki specifications — status tracker and permanent navigation."
read_when:
  - Finding the right spec for a concept or component
  - Checking specification progress
status: active
last_updated: "2026-04-27"
---

# Specifications

## Model

Data model and knowledge structure.

| Spec                                                             | Description                                                    |
| ---------------------------------------------------------------- | -------------------------------------------------------------- |
| [wiki-repository-layout.md](model/wiki-repository-layout.md)    | Wiki repo structure, content layers, roots                     |
| [page-content.md](model/page-content.md)                        | Page format, flat vs bundle, slug resolution, body conventions |
| [epistemic-model.md](model/epistemic-model.md)                  | Why types carry epistemic distinctions                         |
| [wiki-toml.md](model/wiki-toml.md)                              | wiki.toml reference — identity, type registry, per-wiki settings |
| [global-config.md](model/global-config.md)                      | config.toml reference — space registry, defaults, global-only settings |
| [type-system.md](model/type-system.md)                          | Type system mechanism, built-in types index                    |
| [types/base.md](model/types/base.md)                            | Base schema and default fallback type                          |
| [types/concept.md](model/types/concept.md)                      | concept and query-result types                                 |
| [types/source.md](model/types/source.md)                        | Source types (paper, article, documentation, ...)              |
| [types/skill.md](model/types/skill.md)                          | Skill type with field aliasing                                 |
| [types/doc.md](model/types/doc.md)                              | Doc type with agent-foundation fields                          |
| [types/section.md](model/types/section.md)                      | Section index type                                             |

## Tools

| Spec                                                   | Description                                                        |
| ------------------------------------------------------ | ------------------------------------------------------------------ |
| [overview.md](tools/overview.md)                       | Tool surface summary, design principle, global flags               |
| [space-management.md](tools/space-management.md)       | init, spaces list/remove/set-default (4 tools)                     |
| [config-management.md](tools/config-management.md)     | wiki_config tool — get/set/list                                    |
| [content-operations.md](tools/content-operations.md)   | read, write, new-page, new-section, commit (5 tools)               |
| [search.md](tools/search.md)                           | Full-text search with optional type filter and `format: "llms"`    |
| [list.md](tools/list.md)                               | Paginated page listing with type and status filters and `format: "llms"` |
| [ingest.md](tools/ingest.md)                           | Validate, index, and optionally commit                             |
| [graph.md](tools/graph.md)                             | Generate concept graph (Mermaid, DOT, or `format: "llms"`)         |
| [export.md](tools/export.md)                           | Export full wiki to file — llms-txt, llms-full, json               |
| [history.md](tools/history.md)                         | Git commit history for a page                                      |
| [lint.md](tools/lint.md)                               | Deterministic index-based lint rules (orphan, broken-link, …)      |
| [stats.md](tools/stats.md)                             | Wiki health dashboard — page counts, orphans, connectivity         |
| [suggest.md](tools/suggest.md)                         | Suggest related pages to link                                      |
| [index.md](tools/index.md)                             | Rebuild and inspect the search index                               |

## Engine

Engine behavior contracts.

| Spec                                              | Description                                                          |
| ------------------------------------------------- | -------------------------------------------------------------------- |
| [engine-state.md](engine/engine-state.md)         | Engine state at ~/.llm-wiki/ — config, indexes, logs                 |
| [index-management.md](engine/index-management.md) | Tantivy schema, field mapping, staleness, versioning, rebuild        |
| [graph.md](engine/graph.md)                       | Petgraph: typed nodes, labeled edges, rendering                      |
| [ingest-pipeline.md](engine/ingest-pipeline.md)   | Page discovery, validate → alias → index → commit flow               |
| [server.md](engine/server.md)                     | Transports (stdio, SSE, ACP), multi-wiki, resilience, logging        |

## Integrations

How external tools connect.

| Spec                                              | Description                                   |
| ------------------------------------------------- | --------------------------------------------- |
| [mcp-clients.md](integrations/mcp-clients.md)     | Cursor, VS Code, Windsurf, generic MCP config |
| [acp-transport.md](integrations/acp-transport.md) | ACP for Zed / VS Code agent panel             |


## Specification Status

| Spec                                | Status   |
| ----------------------------------- | -------- |
| `model/wiki-repository-layout.md`   | ready    |
| `model/page-content.md`             | ready    |
| `model/epistemic-model.md`          | ready    |
| `model/wiki-toml.md`                | ready    |
| `model/global-config.md`            | ready    |
| `model/type-system.md`              | ready    |
| `model/types/base.md`               | ready    |
| `model/types/concept.md`            | ready    |
| `model/types/source.md`             | ready    |
| `model/types/skill.md`              | ready    |
| `model/types/doc.md`                | ready    |
| `model/types/section.md`            | ready    |
| `tools/overview.md`                 | ready    |
| `tools/space-management.md`         | ready    |
| `tools/config-management.md`        | ready    |
| `tools/content-operations.md`       | ready    |
| `tools/search.md`                   | ready    |
| `tools/list.md`                     | ready    |
| `tools/ingest.md`                   | ready    |
| `tools/graph.md`                    | ready    |
| `tools/export.md`                   | ready    |
| `tools/history.md`                  | ready    |
| `tools/lint.md`                     | ready    |
| `tools/stats.md`                    | ready    |
| `tools/suggest.md`                  | ready    |
| `tools/index.md`                    | ready    |
| `engine/engine-state.md`            | ready    |
| `engine/index-management.md`        | ready    |
| `engine/graph.md`                   | ready    |
| `engine/ingest-pipeline.md`         | ready    |
| `engine/server.md`                  | ready    |
| `integrations/mcp-clients.md`       | ready    |
| `integrations/acp-transport.md`     | ready    |

| Status     | Meaning                                       |
| ---------- | --------------------------------------------- |
| `ready`    | Written, reviewed, aligned with design docs   |
| `proposal` | Draft exists, needs review or completion      |
| `plan`     | Not yet written, scope defined in this prompt |


