---
title: "Embedding llm-wiki-engine"
summary: "Use llm-wiki-engine as a library in your own Rust binary."
status: ready
last_updated: "2026-08-21"
---

# Embedding llm-wiki-engine

`llm-wiki-engine` is a regular Rust library crate. Add it to your `Cargo.toml` and call the engine
directly — no server process, no MCP layer.

## Add the dependency

```toml
[dependencies]
llm-wiki-engine = "1.0"
anyhow = "1"
```

## Minimal example

Load the default config, search a wiki, and list the first page of results:

```rust
use std::path::PathBuf;

use llm_wiki_engine::ops::{SearchParams, list, search};
use llm_wiki_engine::WikiEngine;

fn main() -> anyhow::Result<()> {
    let config_path = PathBuf::from(std::env::var("HOME")?)
        .join(".llm-wiki")
        .join("config.toml");

    let engine = WikiEngine::build(&config_path)?;

    engine.with_state(|state| {
        let wiki_name = state.resolve_wiki_name(None)?.to_string();

        let result = search(
            state,
            &wiki_name,
            &SearchParams {
                query: "async rust",
                type_filter: None,
                no_excerpt: false,
                top_k: Some(5),
                include_sections: false,
                cross_wiki: false,
            },
        )?;

        for hit in &result.results {
            println!("[{:.2}] {} — {}", hit.score, hit.slug, hit.title);
        }

        // List first page of results
        let page_list = list(state, &wiki_name, None, None, 1, Some(5))?;
        for entry in &page_list.pages {
            println!("  {} [{}]", entry.slug, entry.status);
        }

        Ok(())
    })?;

    Ok(())
}
```

Run the bundled example against any configured wiki:

```bash
WIKI=rust-kb cargo run --example embed -- "async rust"
```

## Key types

| Type | Module | Purpose |
|---|---|---|
| `WikiEngine` | `llm_wiki_engine` | Owns all wiki spaces; built once, shared via `Arc`. Use `with_state` to read engine state. |
| `EngineState` | `llm_wiki_engine::engine` | View of mounted spaces; passed to `ops::*` via `with_state` closure |
| `SearchParams` | `llm_wiki_engine::ops` | Search query + options |
| `SearchResult` | `llm_wiki_engine` | Ranked hits with excerpts |
| `WikiStats` | `llm_wiki_engine::ops` | Aggregate stats from `ops::stats()` |

## Concurrency

`WikiEngine` wraps its state in `Arc<RwLock<EngineState>>`. The `state` field is
`pub(crate)` — use `with_state` to access it. `with_state` acquires a read lock,
runs your closure, and releases the lock. Index writes (ingest, rebuild) use a
write lock internally — do not call `ingest` or `refresh_index` from inside a
`with_state` closure.

```rust
let engine = Arc::new(WikiEngine::build(&config_path)?);

// Read lock acquired and released inside with_state
let result = engine.with_state(|state| {
    search(state, "my-wiki", &SearchParams {
        query: "query",
        type_filter: None,
        no_excerpt: true,
        top_k: Some(10),
        include_sections: false,
        cross_wiki: false,
    })
})?;
```

See [engine-state-embedding-api](../decisions/1.0.0/engine-state-embedding-api.md)
for the rationale behind `with_state` and the `pub(crate)` visibility change.

## Available ops

All MCP tools are thin wrappers around functions in `llm_wiki_engine::ops`:

| Function | Equivalent MCP tool |
|---|---|
| `ops::search` | `wiki_search` |
| `ops::list` | `wiki_list` |
| `ops::stats` | `wiki_stats` |
| `ops::run_lint` | `wiki_lint` |
| `ops::graph_build` | `wiki_graph` |
| `ops::suggest` | `wiki_suggest` |
| `ops::history` | `wiki_history` |
| `ops::ingest` | `wiki_ingest` |

See `src/ops/` for the full function signatures.
