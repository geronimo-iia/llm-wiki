use clap::{Parser, Subcommand};

/// Root CLI entry point — parses subcommands and global flags.
#[derive(Parser)]
#[command(
    name = "llm-wiki",
    version,
    about = "Git-backed wiki engine with MCP server"
)]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Target a specific wiki
    #[arg(long, global = true)]
    pub wiki: Option<String>,

    /// Path to global config file (default: ~/.llm-wiki/config.toml).
    /// Overrides the LLM_WIKI_CONFIG environment variable.
    #[arg(long, global = true)]
    pub config: Option<std::path::PathBuf>,
}

/// Top-level subcommands available from the `llm-wiki` CLI.
#[derive(Subcommand)]
pub enum Commands {
    /// Manage wiki spaces
    Spaces {
        /// The spaces subcommand.
        #[command(subcommand)]
        action: SpacesAction,
    },
    /// Read and write configuration
    Config {
        /// The config subcommand.
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Content operations (read, write, new, commit)
    Content {
        /// The content subcommand.
        #[command(subcommand)]
        action: ContentAction,
    },
    /// Full-text BM25 search
    Search {
        /// Search query
        query: String,
        /// Filter by frontmatter type
        #[arg(long, name = "type")]
        r#type: Option<String>,
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
        cross_wiki: bool,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
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
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Validate and index files in the wiki tree
    Ingest {
        /// Slug, URI, or path relative to wiki root
        path: String,
        /// Validate only, no commit
        #[arg(long)]
        dry_run: bool,
        /// Redact secrets from file bodies before validation (opt-in; lossy)
        #[arg(long)]
        redact: bool,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Generate a concept graph
    Graph {
        /// Output format: mermaid | dot | llms
        #[arg(long)]
        format: Option<String>,
        /// Subgraph from this node (slug)
        #[arg(long)]
        root: Option<String>,
        /// Hop limit from root
        #[arg(long)]
        depth: Option<usize>,
        /// Comma-separated page types to include
        #[arg(long, name = "type")]
        r#type: Option<String>,
        /// Filter edges by relation label
        #[arg(long)]
        relation: Option<String>,
        /// File path for output (default: stdout)
        #[arg(long)]
        output: Option<String>,
        /// Merge all mounted wikis into a unified graph
        #[arg(long)]
        cross_wiki: bool,
    },
    /// Manage the tantivy search index
    Index {
        /// The index subcommand.
        #[command(subcommand)]
        action: IndexAction,
    },
    /// Git commit history for a page
    History {
        /// Slug or wiki:// URI
        slug: String,
        /// Max entries to return
        #[arg(long, short = 'n')]
        limit: Option<usize>,
        /// Disable rename tracking
        #[arg(long)]
        no_follow: bool,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Wiki health dashboard
    Stats {
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Run deterministic lint rules on the wiki index
    Lint {
        /// Comma-separated rule names: orphan, broken-link, broken-cross-wiki-link,
        /// missing-fields, stale, unknown-type, articulation-point, bridge, periphery
        #[arg(long)]
        rules: Option<String>,
        /// Filter output by severity: error | warning
        #[arg(long)]
        severity: Option<String>,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Suggest related pages to link
    Suggest {
        /// Slug or wiki:// URI
        slug: String,
        /// Max suggestions
        #[arg(long, short = 'n')]
        limit: Option<usize>,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Inspect and manage type schemas
    Schema {
        /// The schema subcommand.
        #[command(subcommand)]
        action: SchemaAction,
    },
    /// Export the full wiki to a file (llms.txt, llms-full, or json)
    Export {
        /// Output path (relative to wiki root or absolute; default: llms.txt)
        #[arg(long)]
        path: Option<String>,
        /// Export format: llms-txt | llms-full | json
        #[arg(long)]
        format: Option<String>,
        /// Page status filter: active | all (default: active, excludes archived)
        #[arg(long)]
        status: Option<String>,
    },
    /// Start the wiki MCP/ACP server
    Serve {
        /// Enable HTTP transport (optional port, e.g. :8080)
        #[arg(long, value_name = "PORT")]
        http: Option<Option<String>>,
        /// Enable ACP transport
        #[arg(long)]
        acp: bool,
        /// Enable filesystem watcher
        #[arg(long)]
        watch: bool,
        /// Print what would be started, no server
        #[arg(long)]
        dry_run: bool,
    },
    /// Auto-ingest on file save (standalone watcher)
    Watch {
        /// Target wiki name
        #[arg(long)]
        wiki: Option<String>,
    },
    /// Inspect and manage server logs
    Logs {
        /// The logs subcommand.
        #[command(subcommand)]
        action: LogsAction,
    },
}

/// Subcommands for `llm-wiki logs`.
#[derive(Subcommand)]
pub enum LogsAction {
    /// Show recent log entries
    Tail {
        /// Number of lines to show (default: 50)
        #[arg(long, default_value = "50")]
        lines: usize,
    },
    /// List log files
    List,
    /// Delete all log files
    Clear,
}

/// Subcommands for `llm-wiki spaces`.
#[derive(Subcommand)]
pub enum SpacesAction {
    /// Create a new wiki repository
    Create {
        /// Path to create the wiki at
        path: String,
        /// Wiki name — used in wiki:// URIs
        #[arg(long)]
        name: String,
        /// Optional one-line description
        #[arg(long)]
        description: Option<String>,
        /// Update space entry if name differs from existing
        #[arg(long)]
        force: bool,
        /// Set as default wiki
        #[arg(long)]
        set_default: bool,
        /// Content directory relative to repo root (default: "wiki")
        #[arg(long)]
        wiki_root: Option<String>,
    },
    /// Register an existing wiki repository without creating files
    Register {
        /// Absolute path to the existing wiki repository
        path: String,
        /// Wiki name — used in wiki:// URIs
        #[arg(long)]
        name: String,
        /// Optional one-line description
        #[arg(long)]
        description: Option<String>,
        /// Content directory relative to repo root (overrides wiki.toml)
        #[arg(long)]
        wiki_root: Option<String>,
    },
    /// List all registered wikis
    List {
        /// Wiki name (omit for all)
        name: Option<String>,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Remove a wiki from the registry
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

/// Subcommands for `llm-wiki config`.
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
        /// Write to global config
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
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
}

/// Subcommands for `llm-wiki content`.
#[derive(Subcommand)]
pub enum ContentAction {
    /// Read a page or asset by slug or wiki:// URI
    Read {
        /// Slug or wiki:// URI
        uri: String,
        /// Strip frontmatter from output
        #[arg(long)]
        no_frontmatter: bool,
        /// List co-located assets instead of content
        #[arg(long)]
        list_assets: bool,
    },
    /// Write a file into the wiki tree
    Write {
        /// Slug or wiki:// URI
        uri: String,
        /// Read content from a file instead of stdin
        #[arg(long)]
        file: Option<String>,
    },
    /// Create a page or section with scaffolded frontmatter
    New {
        /// Slug or wiki:// URI
        uri: String,
        /// Create a section instead of a page
        #[arg(long)]
        section: bool,
        /// Create as bundle (folder + index.md)
        #[arg(long)]
        bundle: bool,
        /// Page title (default: derived from slug)
        #[arg(long)]
        name: Option<String>,
        /// Page type (default: page)
        #[arg(long, name = "type")]
        r#type: Option<String>,
        /// Show what would be created without creating
        #[arg(long)]
        dry_run: bool,
    },
    /// Commit pending changes to git
    Commit {
        /// Page slugs to commit (omit for --all)
        slugs: Vec<String>,
        /// Commit all pending changes
        #[arg(long)]
        all: bool,
        /// Commit message
        #[arg(long, short)]
        message: Option<String>,
    },
}

/// Subcommands for `llm-wiki index`.
#[derive(Subcommand)]
pub enum IndexAction {
    /// Rebuild the search index from committed Markdown
    Rebuild {
        /// Walk and count pages, no write
        #[arg(long)]
        dry_run: bool,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Inspect index health
    Status {
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
}

/// Subcommands for `llm-wiki schema`.
#[derive(Subcommand)]
pub enum SchemaAction {
    /// List all registered types
    List {
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Show JSON Schema or frontmatter template for a type
    Show {
        /// Type name
        name: String,
        /// Print frontmatter template instead of schema
        #[arg(long)]
        template: bool,
        /// Output format: text | json
        #[arg(long)]
        format: Option<String>,
    },
    /// Register a custom type
    Add {
        /// Type name
        name: String,
        /// Path to JSON Schema file
        schema_path: String,
    },
    /// Unregister a type and remove its pages from the index
    Remove {
        /// Type name
        name: String,
        /// Also delete/modify the schema file
        #[arg(long)]
        delete: bool,
        /// Also delete page .md files from disk
        #[arg(long)]
        delete_pages: bool,
        /// Show what would be done without doing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate schema files and index resolution
    Validate {
        /// Validate a specific type only (omit for all)
        name: Option<String>,
    },
}
