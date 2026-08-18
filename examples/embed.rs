//! Embedding example — use `llm-wiki-engine` as a library (lib target: `llm_wiki`).
//!
//! Loads the default config, searches a wiki, and lists the first page of results.
//!
//! ```
//! cargo run --example embed -- "mixture of experts"
//! ```
//!
//! The wiki name defaults to the configured default. Override with the `WIKI`
//! environment variable: `WIKI=research cargo run --example embed -- "query"`.

use std::path::PathBuf;

// The package is `llm-wiki-engine`; the lib target is named `llm_wiki`.
use llm_wiki::{SearchResult, WikiEngine};
use llm_wiki::ops::{SearchParams, list, search};

fn main() -> anyhow::Result<()> {
    let query = std::env::args().nth(1).unwrap_or_else(|| "knowledge".into());
    let wiki_override = std::env::var("WIKI").ok();

    // ── 1. Locate and load the global config ──────────────────────────────────
    let home = std::env::var("HOME").expect("HOME not set");
    let config_path = PathBuf::from(home).join(".llm-wiki").join("config.toml");

    let engine = WikiEngine::build(&config_path)?;

    // ── 2. Resolve target wiki name ────────────────────────────────────────────
    let state = engine.state.read().map_err(|_| anyhow::anyhow!("lock poisoned"))?;
    let wiki_name = wiki_override
        .as_deref()
        .unwrap_or_else(|| state.default_wiki_name())
        .to_string();

    println!("Wiki   : {wiki_name}");
    println!("Query  : {query}\n");

    // ── 3. Search ──────────────────────────────────────────────────────────────
    let params = SearchParams {
        query: &query,
        type_filter: None,
        no_excerpt: false,
        top_k: Some(5),
        include_sections: false,
        cross_wiki: false,
    };

    let result: SearchResult = search(&state, &wiki_name, &params)?;

    if result.results.is_empty() {
        println!("No results.");
    } else {
        println!("Results ({} hits):", result.results.len());
        for hit in &result.results {
            println!("  [{:.2}] {} — {}", hit.score, hit.slug, hit.title);
        }
    }

    // ── 4. List first page ─────────────────────────────────────────────────────
    println!("\nAll pages (first 5):");
    let page_list = list(&state, &wiki_name, None, None, 0, Some(5))?;
    for entry in &page_list.pages {
        println!("  {} [{}]", entry.slug, entry.status);
    }
    println!("  … {} total", page_list.total);

    Ok(())
}
