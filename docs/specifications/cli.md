---
title: "CLI Reference"
summary: "Complete command-line reference for wiki — all commands, subcommands, and flags."
read_when:
  - Looking up a specific command, subcommand, or flag
  - Implementing the CLI in cli.rs
  - Understanding the full command surface of the wiki binary
status: active
last_updated: "2025-07-15"
---

# CLI Reference

All commands follow the pattern `wiki <command> [subcommand] [args] [flags]`.
Global flags apply to all commands.

---

## Global Flags

```
wiki [--wiki <name>]    # target a specific wiki (default: global.default_wiki)
```

---

## `wiki init`

Initialize a new wiki repository.

```
wiki init <path>
          --name <name>              # wiki name — required
          [--description <text>]     # one-line description
          [--force]                  # update registry if name differs
          [--set-default]            # set as default_wiki
```

See [init.md](init.md).

---

## `wiki new`

Create pages and sections with scaffolded frontmatter.

```
wiki new page <wiki:// URI>     # flat page with minimal frontmatter
             [--bundle]         # bundle folder + index.md instead
             [--dry-run]

wiki new section <wiki:// URI>  # directory + index.md with frontmatter
                [--dry-run]
```

Parent sections are auto-created if missing. Wiki name can be omitted for
the default wiki: `wiki://concepts/mixture-of-experts`.
See [page-creation.md](page-creation.md).

---

## `wiki ingest`

Add content to the wiki.

```
wiki ingest <path>                    # file or folder
            [--target <name|uri>]     # wiki name or wiki:// URI (default: default wiki)
            [--update]                # overwrite existing page
            [--append]                # append to existing page body
            [--analysis <file>]       # optional enrichment JSON
            [--dry-run]
```

See [ingest.md](ingest.md).

---

## `wiki read`

Fetch the full content of a single page.

```
wiki read <slug|uri>
          [--no-frontmatter]        # strip frontmatter from output
          [--wiki <name>]
```

Accepts slug (`concepts/mixture-of-experts`) or `wiki://` URI
(`wiki://research/concepts/mixture-of-experts` or `wiki://concepts/mixture-of-experts`
for the default wiki). See [read.md](read.md).

---

## `wiki search`

Full-text BM25 search.

```
wiki search "<query>"
            [--no-excerpt]          # refs only, no excerpt
            [--top-k <n>]           # default: from config (built-in: 10)
            [--include-sections]    # include section index pages
            [--all]                 # search across all registered wikis
            [--wiki <name>]
            [--dry-run]
```

Returns `Vec<PageRef>`: slug, `wiki://` URI, title, score, excerpt.
See [search.md](search.md).

---

## `wiki list`

Paginated enumeration of wiki pages.

```
wiki list
         [--type <type>]            # filter by frontmatter type
         [--status <status>]        # filter by frontmatter status
         [--page <n>]               # page number, 1-based (default: 1)
         [--page-size <n>]          # default: from config (built-in: 20)
         [--wiki <name>]
```

See [list.md](list.md).

---

## `wiki lint`

Structural audit of the wiki.

```
wiki lint
         [--wiki <name>]
         [--dry-run]

wiki lint fix
             [--only <check>]       # missing-stubs | empty-sections
             [--dry-run]
             [--wiki <name>]
```

Checks: orphan pages, missing stubs, empty sections. Writes and commits
`LINT.md`. See [lint.md](lint.md).

---

## `wiki graph`

Generate a concept graph.

```
wiki graph
          [--format <fmt>]          # mermaid | dot (default: from config)
          [--root <slug|uri>]       # subgraph from this node
          [--depth <n>]             # hop limit (default: from config)
          [--type <types>]          # comma-separated page types
          [--output <path>]         # file path or wiki:// URI (default: stdout)
          [--dry-run]
          [--wiki <name>]
```

See [graph.md](graph.md).

---

## `wiki index`

Manage the tantivy search index.

```
wiki index rebuild
               [--wiki <name>]
               [--dry-run]

wiki index status
               [--wiki <name>]
```

See [index.md](index.md).

---

## `wiki config`

Read and write configuration.

```
wiki config get <key>
wiki config set <key> <value>
                [--global]          # write to ~/.wiki/config.toml
                [--wiki <name>]     # write to per-wiki config
wiki config list
             [--global]
             [--wiki <name>]
```

See [configuration.md](configuration.md).

---

## `wiki registry`

Manage the wiki registry.

```
wiki registry list

wiki registry remove <name>
                     [--delete]     # also delete local directory

wiki registry set-default <name>    # alias for wiki config set global.default_wiki
```

See [registry.md](registry.md).

---

## `wiki serve`

Start the wiki server.

```
wiki serve
          [--sse [:<port>]]         # enable SSE transport (default port: from config)
          [--acp]                   # enable ACP transport
          [--dry-run]
```

stdio is always active. SSE and ACP are opt-in and can run simultaneously.
All registered wikis are mounted at startup. See [serve.md](serve.md).

---

## `wiki instruct`

Print embedded workflow instructions.

```
wiki instruct                       # all instructions
wiki instruct <workflow>            # help | ingest | research | lint | enrich
```

See [instruct.md](instruct.md).
