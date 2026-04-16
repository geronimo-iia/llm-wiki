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

## `llm-wiki init`

Initialize a new wiki repository.

```
llm-wiki init <path>
          --name <name>              # wiki name — required
          [--description <text>]     # one-line description
          [--force]                  # update space entry if name differs
          [--set-default]            # set as default_wiki
```

See [init.md](init.md).

---

## `llm-wiki new`

Create pages and sections with scaffolded frontmatter.

```
llm-wiki new page <wiki:// URI>     # flat page with minimal frontmatter
             [--bundle]         # bundle folder + index.md instead
             [--dry-run]

llm-wiki new section <wiki:// URI>  # directory + index.md with frontmatter
                [--dry-run]
```

Parent sections are auto-created if missing. Wiki name can be omitted for
the default wiki: `wiki://concepts/mixture-of-experts`.
See [page-creation.md](page-creation.md).

---

## `llm-wiki ingest`

Validate and index files already in the wiki tree. Commits only when
`ingest.auto_commit` is `true` (the default).

```
llm-wiki ingest <path>                    # file or folder, relative to wiki root
            [--dry-run]
```

See [ingest.md](ingest.md).

---

## `llm-wiki read`

Fetch the full content of a single page.

```
llm-wiki read <slug|uri>
          [--no-frontmatter]        # strip frontmatter from output
          [--wiki <name>]
```

Accepts slug (`concepts/mixture-of-experts`) or `wiki://` URI
(`wiki://research/concepts/mixture-of-experts` or `wiki://concepts/mixture-of-experts`
for the default wiki). See [read.md](read.md).

---

## `llm-wiki search`

Full-text BM25 search.

```
llm-wiki search "<query>"
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

## `llm-wiki list`

Paginated enumeration of wiki pages.

```
llm-wiki list
         [--type <type>]            # filter by frontmatter type
         [--status <status>]        # filter by frontmatter status
         [--page <n>]               # page number, 1-based (default: 1)
         [--page-size <n>]          # default: from config (built-in: 20)
         [--wiki <name>]
```

See [list.md](list.md).

---

## `llm-wiki lint`

Structural audit of the wiki.

```
llm-wiki lint
         [--wiki <name>]
         [--dry-run]

llm-wiki lint fix
             [--only <check>]       # missing-stubs | empty-sections
             [--dry-run]
             [--wiki <name>]
```

Checks: orphan pages, missing stubs, empty sections. Writes
`LINT.md`. See [lint.md](lint.md).

---

## `llm-wiki graph`

Generate a concept graph.

```
llm-wiki graph
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

## `llm-wiki index`

Manage the tantivy search index.

```
llm-wiki index rebuild
               [--wiki <name>]
               [--dry-run]

llm-wiki index status
               [--wiki <name>]

llm-wiki index check
               [--wiki <name>]
```

See [index.md](index.md).

---

## `llm-wiki config`

Read and write configuration.

```
llm-wiki config get <key>
llm-wiki config set <key> <value>
                [--global]          # write to ~/.llm-wiki/config.toml
                [--wiki <name>]     # write to per-wiki config
llm-wiki config list
             [--global]
             [--wiki <name>]
```

See [configuration.md](configuration.md).

---

## `llm-wiki spaces`

Manage wiki spaces.

```
llm-wiki spaces list

llm-wiki spaces remove <name>
                   [--delete]     # also delete local directory

llm-wiki spaces set-default <name>    # alias for llm-wiki config set global.default_wiki
```

See [spaces.md](spaces.md).

---

## `llm-wiki serve`

Start the llm-wiki server.

```
llm-wiki serve
          [--sse [:<port>]]         # enable SSE transport (default port: from config)
          [--acp]                   # enable ACP transport
          [--dry-run]
```

stdio is always active. SSE and ACP are opt-in and can run simultaneously.
All registered wikis are mounted at startup. See [serve.md](serve.md).

---

## `llm-wiki instruct`

Print embedded workflow instructions.

```
llm-wiki instruct                       # all instructions
llm-wiki instruct <workflow>            # help | ingest | research | lint | crystallize | commit | frontmatter
```

See [instruct.md](instruct.md).

---

## `llm-wiki commit`

Commit pending changes to git.

```
llm-wiki commit [<slug>...]             # commit specific pages by slug
            --all                   # commit all pending changes
            [-m, --message <msg>]   # commit message
            [--wiki <name>]
```

See [commit.md](commit.md).
