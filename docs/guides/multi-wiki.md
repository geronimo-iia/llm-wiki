# Multi-Wiki

llm-wiki manages multiple wiki repositories from a single process.
Each wiki has its own type schemas, search index, and git history.

## Create Multiple Wikis

```bash
llm-wiki spaces create ~/wikis/research --name research
llm-wiki spaces create ~/wikis/work --name work
llm-wiki spaces create ~/wikis/personal --name personal
```

The first wiki created becomes the default. Check:

```bash
llm-wiki spaces list
```

```
* research    /Users/you/wikis/research    ML research knowledge base
  work        /Users/you/wikis/work        —
  personal    /Users/you/wikis/personal    —
```

## Target a Specific Wiki

Every command accepts `--wiki <name>`:

```bash
llm-wiki search "scaling laws" --wiki research
llm-wiki list --type concept --wiki work
llm-wiki ingest wiki/ --wiki personal
```

Without `--wiki`, the default wiki is used.

## Change the Default

```bash
llm-wiki spaces set-default work
```

## wiki:// URIs

Pages are addressable across wikis with `wiki://` URIs:

```
wiki://research/concepts/mixture-of-experts
wiki://work/projects/q3-roadmap
wiki://personal/notes/reading-list
```

The format is `wiki://<wiki-name>/<slug>`. Use them in:

```bash
llm-wiki content read wiki://research/concepts/moe
llm-wiki content write wiki://work/projects/new-project
```

And in frontmatter references:

```yaml
sources:
  - wiki://research/sources/switch-transformer
```

## Cross-Wiki Search

Search across all registered wikis:

```bash
llm-wiki search "transformer" --cross-wiki
```

Results from all wikis are merged and ranked by score. Each result
includes its `wiki://` URI so you know which wiki it came from.

## When to Split vs Keep One Wiki

| One wiki                 | Multiple wikis                      |
| ------------------------ | ----------------------------------- |
| All knowledge is related | Distinct domains (work vs personal) |
| Single concept graph     | Separate graphs per domain          |
| Simpler setup            | Different schemas per wiki          |
| One git history          | Separate access control per repo    |

Rules of thumb:
- If pages reference each other → same wiki
- If they never reference each other → separate wikis
- If different people own different domains → separate wikis
- When in doubt → start with one, split later

## Per-Wiki Configuration

Each wiki has its own `wiki.toml` for identity and settings:

```toml
# ~/wikis/research/wiki.toml
name = "research"
description = "ML research knowledge base"

[ingest]
auto_commit = true

[validation]
type_strictness = "strict"
```

Global defaults live in `~/.llm-wiki/config.toml`. Per-wiki settings
override global ones.

```bash
# Set per-wiki
llm-wiki config set validation.type_strictness strict --wiki research

# Set global default
llm-wiki config set defaults.search_top_k 20 --global
```

## Per-Wiki Schemas

Each wiki has its own `schemas/` directory. A research wiki might have
`paper`, `concept`, `query-result`. A work wiki might have `meeting`,
`project`, `decision`.

Custom types are per-wiki — they don't leak across wikis.

## Remove a Wiki

```bash
# Unregister (keeps files)
llm-wiki spaces remove personal

# Unregister and delete files
llm-wiki spaces remove personal --delete
```

Cannot remove the default wiki — set a new default first.

## How It Works

All wikis are mounted at engine startup. The MCP server exposes all
wikis through the same tool surface. Each wiki has its own:

- `SpaceTypeRegistry` (schemas + validators)
- `SpaceIndexManager` (tantivy index)
- Git repository

They share the same `WikiEngine` process and the same transports
(stdio, SSE, ACP).
