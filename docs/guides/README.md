# Guides

User-facing documentation for installing, configuring, and integrating
llm-wiki.

| Guide                                    | Description                                                       |
| ---------------------------------------- | ----------------------------------------------------------------- |
| [getting-started.md](getting-started.md) | End-to-end walkthrough: install → create → write → search → graph |
| [installation.md](installation.md)       | Install llm-wiki (script, cargo, homebrew, asdf)                  |
| [configuration.md](configuration.md)     | Common settings, per-wiki overrides, troubleshooting              |
| [ide-integration.md](ide-integration.md) | Connect to VS Code, Cursor, Windsurf, Zed, Claude Code            |
| [multi-wiki.md](multi-wiki.md)           | Manage multiple wikis, cross-wiki search, wiki:// URIs            |
| [custom-types.md](custom-types.md)       | Add custom page types with JSON Schema                            |
| [search-ranking.md](search-ranking.md)   | Tune search ranking: status multipliers, custom statuses, per-wiki overrides |
| [llms-format.md](llms-format.md)         | LLM-optimized output: when and how to use `format: "llms"` and `wiki_export` |
| [lint.md](lint.md)                       | Catch broken links, orphans, missing fields, stale pages, and unknown types  |
| [redaction.md](redaction.md)             | Scrub secrets from page bodies before commit with `redact: true`             |
| [ci-cd.md](ci-cd.md)                     | Schema validation, index rebuild, and ingest in CI pipelines      |
| [release.md](release.md)                 | Release process and distribution channels                         |
