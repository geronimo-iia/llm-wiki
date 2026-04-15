//! CLI argument definitions — clap `derive` API.
//!
//! All commands are typed here; dispatch lives in `main.rs`.

use clap::{Parser, Subcommand};

/// `wiki` — git-backed wiki engine. Bring your own LLM.
#[derive(Parser, Debug)]
#[command(name = "wiki", about = "Git-backed wiki engine — bring your own LLM")]
#[command(version)]
pub struct Cli {
    /// Target a specific registered wiki by name (default: the wiki marked `default = true`).
    #[arg(long, global = true, value_name = "NAME")]
    pub wiki: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

/// All `wiki` subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Integrate an Analysis JSON document into the wiki.
    ///
    /// Reads from FILE or from stdin when FILE is `-`.
    Ingest {
        /// Path to `analysis.json`, or `-` to read from stdin.
        #[arg(default_value = "-")]
        file: String,
    },

    /// Full-text search across wiki pages.
    Search {
        /// Search query. May be omitted when using --rebuild-index alone.
        query: Option<String>,

        /// Maximum number of results to display.
        #[arg(long, default_value_t = 20)]
        top: usize,

        /// Search across all registered wikis.
        #[arg(long)]
        all: bool,

        /// Rebuild the tantivy index and exit (or rebuild before searching).
        #[arg(long)]
        rebuild_index: bool,
    },

    /// Return the top-K relevant pages as Markdown context for an external LLM.
    Context {
        /// The question to retrieve context for.
        question: String,

        /// Number of pages to return.
        #[arg(long, default_value_t = 5)]
        top_k: usize,
    },

    /// Structural lint pass: orphan pages, missing stubs, active contradictions.
    ///
    /// Writes `LINT.md` and commits it.
    Lint,

    /// List wiki pages, optionally filtered by type.
    List {
        /// Filter by page type: concept, contradiction, query, source.
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,
    },

    /// List contradiction pages, optionally filtered by status.
    Contradict {
        /// Filter by status: active, resolved, under-analysis.
        #[arg(long)]
        status: Option<String>,
    },

    /// Emit the concept graph as DOT or Mermaid.
    Graph {
        /// Output format.
        #[arg(long, default_value = "dot", value_parser = ["dot", "mermaid"])]
        format: String,
    },

    /// Show what the last ingest changed (git diff wrapper).
    Diff,

    /// Start the MCP server.
    ///
    /// Uses stdio transport by default; pass `--sse` for HTTP SSE.
    Serve {
        /// Listen for SSE connections on this address (e.g. `:8080`).
        #[arg(long, value_name = "ADDR")]
        sse: Option<String>,
    },

    /// Initialise a new wiki repository.
    ///
    /// Creates `concepts/`, `sources/`, `contradictions/`, `queries/`, `raw/`,
    /// and `.wiki/config.toml`. Runs `git init` unless `.git/` already exists.
    Init {
        /// Directory to initialise. Defaults to the current directory.
        path: Option<String>,

        /// Register the new wiki in `~/.wiki/config.toml` after initialisation.
        ///
        /// The first wiki registered becomes `default = true` automatically.
        #[arg(long)]
        register: bool,
    },

    /// Print usage instructions for external LLMs.
    Instruct {
        /// Named workflow to print. Omit for general instructions.
        workflow: Option<String>,
    },

    /// Print the content of a wiki page (frontmatter + body).
    Read {
        /// Page slug (e.g. `concepts/foo` or `skills/semantic-commit`).
        slug: String,

        /// Print only the body, omitting frontmatter.
        #[arg(long)]
        body_only: bool,
    },
}
