---
title: "CLI Implementation"
summary: "Clap derive structure, subcommand hierarchy, mapping from old to new."
status: draft
last_updated: "2025-07-17"
---

# CLI Implementation

Implementation reference for the CLI. Not a specification — see
[tools/overview.md](../specifications/tools/overview.md) for the tool
surface.

## Subcommand Hierarchy

```
llm-wiki
├── spaces
│   ├── create <path> --name --description --force --set-default
│   ├── list --format
│   ├── remove <name> --delete
│   └── set-default <name>
├── config
│   ├── get <key>
│   ├── set <key> <value> --global --wiki
│   └── list --global --wiki --format
├── content
│   ├── read <slug|uri> --no-frontmatter --list-assets --format --wiki
│   ├── write <slug|uri> --file --wiki
│   ├── new <slug|uri> --section --bundle --name --type --dry-run --wiki
│   └── commit [<slug>...] --all --message --wiki
├── search "<query>" --type --no-excerpt --top-k --include-sections --all --format --wiki
├── list --type --status --page --page-size --format --wiki
├── ingest <slug|uri> --dry-run --format --wiki
├── graph --format --root --depth --type --relation --output --wiki
├── index
│   ├── rebuild --wiki --format --dry-run
│   └── status --wiki --format
└── serve --sse --acp
```

## Global Flags

```rust
#[arg(long, global = true)]
pub wiki: Option<String>,
```

Available on all commands. Overridden when input is a `wiki://` URI.

## Clap Structure

```rust
#[derive(Parser)]
#[command(name = "llm-wiki")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    wiki: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Spaces { #[command(subcommand)] action: SpacesAction },
    Config { #[command(subcommand)] action: ConfigAction },
    Content { #[command(subcommand)] action: ContentAction },
    Search { ... },
    List { ... },
    Ingest { ... },
    Graph { ... },
    Index { #[command(subcommand)] action: IndexAction },
    Serve { ... },
}
```

### Subcommand enums

```rust
enum SpacesAction {
    Create { path, name, description, force, set_default },
    List { format },
    Remove { name, delete },
    SetDefault { name },
}

enum ConfigAction {
    Get { key },
    Set { key, value, global, wiki },
    List { global, wiki, format },
}

enum ContentAction {
    Read { uri, no_frontmatter, list_assets, format },
    Write { uri, file },
    New { uri, section, bundle, name, r#type, dry_run },
    Commit { slugs, all, message },
}

enum IndexAction {
    Rebuild { dry_run, format },
    Status { format },
}
```

## Existing Code

The current `src/cli.rs` needs restructuring:

| Old                                 | New                     | Change                             |
| ----------------------------------- | ----------------------- | ---------------------------------- |
| `Commands::Init`                    | `SpacesAction::Create`  | Moved under `spaces`               |
| `Commands::New { Page, Section }`   | `ContentAction::New`    | Merged with `--section` flag       |
| `Commands::Read`                    | `ContentAction::Read`   | Moved under `content`              |
| `Commands::Commit`                  | `ContentAction::Commit` | Moved under `content`              |
| `Commands::Search`                  | `Commands::Search`      | Add `--format`, `--type`           |
| `Commands::List`                    | `Commands::List`        | Add `--format`                     |
| `Commands::Ingest`                  | `Commands::Ingest`      | Add `--format`, accept `slug\|uri` |
| `Commands::Graph`                   | `Commands::Graph`       | Add `--relation`                   |
| `Commands::Lint`                    | remove                  | Moved to skills                    |
| `Commands::Instruct`                | remove                  | Moved to skills                    |
| `IndexAction::Check`                | remove                  | Folded into `Status`               |
| `INSTRUCTIONS` / `extract_workflow` | remove                  | Skills handle this                 |

### New additions

- `Commands::Content` with `ContentAction` subcommands
- `ContentAction::Write` (was MCP-only, now has CLI)
- `ContentAction::New` with `--section`, `--name`, `--type` flags
- `--format` flag on search, list, ingest, index, spaces list, config list
