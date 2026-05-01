---
title: "Configuration"
summary: "How to configure llm-wiki — common settings, per-wiki overrides, and troubleshooting."
---

# Configuration

llm-wiki uses two configuration files:

| File | Location | Scope | Committed? |
|------|----------|-------|------------|
| `config.toml` | `~/.llm-wiki/config.toml` | Global — all wikis | No (local to machine) |
| `wiki.toml` | `<wiki>/wiki.toml` | Per-wiki — overrides global | Yes (shared via git) |

Both are created automatically by `llm-wiki spaces create`. You
rarely need to edit them by hand — use `llm-wiki config` instead.

## Overriding the config path

By default llm-wiki reads `~/.llm-wiki/config.toml`. Override it with:

```bash
# Flag (highest priority)
llm-wiki --config /path/to/config.toml spaces list

# Environment variable
export LLM_WIKI_CONFIG=/path/to/config.toml
llm-wiki spaces list
```

Useful for CI pipelines, integration tests, or running multiple isolated
environments on the same machine.

## How settings resolve

```
1. CLI flag          (highest priority)
2. wiki.toml         (per-wiki override)
3. config.toml       (global default)
4. Built-in default  (lowest priority)
```

Example: `defaults.search_top_k` is 10 by default. Set it to 20
globally, then override to 5 for a specific wiki:

```bash
llm-wiki config set defaults.search_top_k 20 --global
llm-wiki config set defaults.search_top_k 5 --wiki research
```

## Reading and changing settings

```bash
# List all resolved settings
llm-wiki config list

# Get a specific value
llm-wiki config get defaults.search_top_k

# Set globally
llm-wiki config set defaults.search_top_k 20 --global

# Set per-wiki
llm-wiki config set defaults.search_top_k 5 --wiki research
```

## Common tasks

### Increase search results

Default is 10. For wikis with lots of content:

```bash
llm-wiki config set defaults.search_top_k 20 --global
```

### Use bundles by default

New pages are flat files by default. Switch to bundles (folder +
index.md) for wikis with lots of assets:

```bash
llm-wiki config set defaults.page_mode bundle --wiki research
```

### Disable auto-commit on ingest

By default, `wiki_ingest` commits to git automatically. Disable to
review changes before committing:

```bash
llm-wiki config set ingest.auto_commit false --wiki research
```

Then commit manually with `llm-wiki content commit --all`.

### Enable strict type validation

By default, unknown types produce a warning. Switch to strict mode
to reject pages with unregistered types:

```bash
llm-wiki config set validation.type_strictness strict --wiki research
```

### Tune the filesystem watcher

The watcher debounces file events to avoid redundant ingests. Default
is 500ms. Lower for faster feedback, higher for busy editors:

```bash
llm-wiki config set watch.debounce_ms 300 --global
```

### Tune lint rules

The `stale` rule fires when a page is both old and low-confidence. Adjust
the thresholds per-wiki in `wiki.toml`:

```toml
[lint]
stale_days                 = 180   # relax for slower-moving wikis
stale_confidence_threshold = 0.6   # flag more aggressively
```

See [lint.md](lint.md) for all rules, fix guidance, and CI usage.

### Configure redaction

Disable specific built-in patterns or add custom ones in `wiki.toml`:

```toml
[redact]
disable = ["email"]   # keep email addresses in a contacts wiki

[[redact.patterns]]
name        = "employee-id"
pattern     = "EMP-[0-9]{6}"
replacement = "[REDACTED:employee-id]"
```

See [redaction.md](redaction.md) for all built-in patterns, report format,
and usage guidance.

### Tune search ranking

Search results are scored as `bm25 × status_multiplier × confidence`.
Adjust multipliers per-wiki in `wiki.toml` (or globally in `config.toml`):

```toml
# wiki.toml
[search.status]
archived = 0.0   # suppress archived pages entirely
stub     = 0.6   # demote stubs without hiding them
```

Only declare the entries that differ from the global defaults — the rest
are inherited automatically. See [search-ranking.md](search-ranking.md)
for the full reference.

### Change graph output format

Default is Mermaid. Switch to DOT for Graphviz:

```bash
llm-wiki config set graph.format dot --global
```

### Disable rename tracking in history

`wiki_history` follows renames by default. Disable per-wiki if it
causes issues:

```bash
llm-wiki config set history.follow false --wiki research
```

### Enable auto-rebuild on stale index

By default, a stale index produces a warning. Enable auto-rebuild
so search/list always use a fresh index:

```bash
llm-wiki config set index.auto_rebuild true --global
```

### Set the ACP session cap

ACP sessions persist in memory until the server stops. The default
cap is 20 concurrent sessions, which is generous for single-user IDE
use. Lower it for resource-constrained environments:

```bash
llm-wiki config set serve.acp_max_sessions 5 --global
```

This is a global-only key — it applies to the server process, not
per-wiki.

### Configure logging

For debugging `llm-wiki serve`:

```bash
# JSON logs for machine parsing
llm-wiki config set logging.log_format json --global

# Keep more log files
llm-wiki config set logging.log_max_files 30 --global

# Disable file logging (stderr only)
llm-wiki config set logging.log_path "" --global
```

## Global-only vs overridable

Some settings only make sense globally (server ports, index recovery,
logging). Setting them in `wiki.toml` produces an error:

```
$ llm-wiki config set serve.http_port 9090 --wiki research
error: serve.http_port is a global-only key — use --global
```

Global-only keys: `index.auto_rebuild`, `index.auto_recovery`,
`serve.*`, `logging.*`, `watch.*`.

Everything else is overridable per-wiki.

## Troubleshooting

### Search returns stale results

The index is out of date. Either:

```bash
# Rebuild manually
llm-wiki index rebuild

# Or enable auto-rebuild
llm-wiki config set index.auto_rebuild true --global

# Or use --watch for live indexing
llm-wiki serve --watch
```

### Unknown type warning on ingest

A page has a `type` field that doesn't match any registered schema.
In `loose` mode (default), this is a warning. In `strict` mode, it's
an error. Check registered types:

```bash
llm-wiki schema list
```

### Config changes not taking effect

Check the resolution order — a per-wiki override may be shadowing
your global setting:

```bash
llm-wiki config list              # resolved (global + per-wiki)
llm-wiki config list --global     # global only
```

## Full reference

For the complete key reference, see
[global-config.md](../specifications/model/global-config.md) and
[wiki-toml.md](../specifications/model/wiki-toml.md).
