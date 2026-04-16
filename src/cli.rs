use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wiki", about = "Git-backed wiki engine with MCP server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Target a specific wiki
    #[arg(long, global = true)]
    pub wiki: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new wiki repository
    Init {
        /// Path to create the wiki at
        path: String,
        /// Wiki name — required, used in wiki:// URIs
        #[arg(long)]
        name: String,
        /// Optional one-line description
        #[arg(long)]
        description: Option<String>,
        /// Update space entry if name differs from existing
        #[arg(long)]
        force: bool,
        /// Set as default_wiki in ~/.wiki/config.toml
        #[arg(long)]
        set_default: bool,
    },
    /// Read and write configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage wiki spaces
    Spaces {
        #[command(subcommand)]
        action: SpacesAction,
    },
    /// Validate, commit, and index files in the wiki tree
    Ingest {
        /// File or folder path, relative to wiki root
        path: String,
        /// Show what would be committed without committing
        #[arg(long)]
        dry_run: bool,
    },
    /// Create pages and sections
    New {
        #[command(subcommand)]
        action: NewAction,
    },
    /// Full-text BM25 search
    Search {
        /// Search query
        query: String,
        /// Omit excerpts — refs only
        #[arg(long)]
        no_excerpt: bool,
        /// Max results (default: from config)
        #[arg(long)]
        top_k: Option<usize>,
        /// Include section index pages in results
        #[arg(long)]
        include_sections: bool,
        /// Search across all registered wikis
        #[arg(long)]
        all: bool,
        /// Print query plan, no search
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch the full content of a single page
    Read {
        /// Slug or wiki:// URI
        uri: String,
        /// Strip frontmatter from output
        #[arg(long)]
        no_frontmatter: bool,
        /// List co-located assets of a bundle page
        #[arg(long)]
        list_assets: bool,
    },
    /// Paginated enumeration of wiki pages
    List {
        /// Filter by frontmatter type
        #[arg(long, name = "type")]
        r#type: Option<String>,
        /// Filter by frontmatter status
        #[arg(long)]
        status: Option<String>,
        /// Page number, 1-based
        #[arg(long, default_value = "1")]
        page: usize,
        /// Results per page
        #[arg(long)]
        page_size: Option<usize>,
    },
    /// Manage the tantivy search index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    /// Structural audit of the wiki
    Lint {
        #[command(subcommand)]
        action: Option<LintAction>,
        /// Show what would be written, no commit
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate a concept graph
    Graph {
        /// Output format: mermaid | dot
        #[arg(long)]
        format: Option<String>,
        /// Subgraph from this node (slug or wiki:// URI)
        #[arg(long)]
        root: Option<String>,
        /// Hop limit from root
        #[arg(long)]
        depth: Option<usize>,
        /// Comma-separated page types to include
        #[arg(long, name = "type")]
        r#type: Option<String>,
        /// File path or wiki:// URI (default: stdout)
        #[arg(long)]
        output: Option<String>,
        /// Print what would be written, no commit
        #[arg(long)]
        dry_run: bool,
    },
    /// Start the wiki MCP/ACP server
    Serve {
        /// Enable SSE transport (optional port, e.g. :8080)
        #[arg(long, value_name = "PORT")]
        sse: Option<Option<String>>,
        /// Enable ACP transport
        #[arg(long)]
        acp: bool,
        /// Print what would be started, no server
        #[arg(long)]
        dry_run: bool,
    },
    /// Print embedded workflow instructions
    Instruct {
        /// Workflow name: help, new, ingest, research, lint, crystallize, frontmatter
        workflow: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum NewAction {
    /// Create a new page with scaffolded frontmatter
    Page {
        /// wiki:// URI for the new page
        uri: String,
        /// Create as bundle (folder + index.md) instead of flat file
        #[arg(long)]
        bundle: bool,
        /// Show what would be created without creating
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a new section with an index page
    Section {
        /// wiki:// URI for the new section
        uri: String,
        /// Show what would be created without creating
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print a config value
    Get {
        /// Config key (e.g. defaults.search_top_k)
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
        /// Write to ~/.wiki/config.toml
        #[arg(long)]
        global: bool,
        /// Write to per-wiki config
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Print all resolved config
    List {
        /// Global config only
        #[arg(long)]
        global: bool,
        /// Per-wiki config only
        #[arg(long)]
        wiki: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum IndexAction {
    /// Rebuild the search index from committed Markdown
    Rebuild {
        /// Walk and count pages, no write
        #[arg(long)]
        dry_run: bool,
    },
    /// Inspect index health
    Status,
}

#[derive(Subcommand)]
pub enum LintAction {
    /// Run auto-fixes for missing stubs and empty sections
    Fix {
        /// Only fix: missing-stubs | empty-sections
        #[arg(long)]
        only: Option<String>,
        /// Show what would be fixed, no commit
        #[arg(long)]
        dry_run: bool,
    },
}

pub const INSTRUCTIONS: &str = include_str!("assets/instructions.md");

pub fn extract_workflow(instructions: &str, name: &str) -> Option<String> {
    let header = format!("## {name}");
    let mut found = false;
    let mut result = Vec::new();

    for line in instructions.lines() {
        if found {
            if line.starts_with("## ") {
                break;
            }
            result.push(line);
        } else if line.trim() == header {
            found = true;
            result.push(line);
        }
    }

    if found {
        while result.last().is_some_and(|l| l.is_empty()) {
            result.pop();
        }
        Some(result.join("\n"))
    } else {
        None
    }
}

#[derive(Subcommand)]
pub enum SpacesAction {
    /// List all registered wikis
    List,
    /// Remove a wiki entry
    Remove {
        /// Wiki name to remove
        name: String,
        /// Also delete the wiki directory from disk
        #[arg(long)]
        delete: bool,
    },
    /// Set the default wiki
    SetDefault {
        /// Wiki name to set as default
        name: String,
    },
}
