//! `wiki` binary entry point — parse the CLI and dispatch to module stubs.

use anyhow::Result;
use clap::Parser;
use llm_wiki::analysis::{PageType, Status};
use llm_wiki::cli::{Cli, Command};
use llm_wiki::config::WikiConfig;
use llm_wiki::context::context;
use llm_wiki::contradiction;
use llm_wiki::git;
use llm_wiki::graph;
use llm_wiki::ingest::{ingest, Input};
use llm_wiki::init::{init_wiki, mcp_config_snippet};
use llm_wiki::lint;
use llm_wiki::markdown::resolve_slug;
use llm_wiki::registry::{WikiRegistry, global_config_path, register_wiki};
use llm_wiki::search::{build_index, search, search_all};
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

/// Truncate a string to `max_chars` columns, appending `…` if truncated.
fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{}…", truncated)
    }
}

/// Parse a status string (from CLI `--status`) into a [`Status`] variant.
fn parse_status(s: &str) -> Option<Status> {
    match s {
        "active" => Some(Status::Active),
        "resolved" => Some(Status::Resolved),
        "under-analysis" => Some(Status::UnderAnalysis),
        _ => None,
    }
}

/// Parse a page-type string (from CLI `--type`) into a [`PageType`] variant.
fn parse_page_type(s: &str) -> Option<PageType> {
    match s {
        "concept" => Some(PageType::Concept),
        "source" => Some(PageType::SourceSummary),
        "contradiction" => Some(PageType::Contradiction),
        "query" => Some(PageType::QueryResult),
        _ => None,
    }
}

/// Serde-deserialise just the `type` field from a YAML frontmatter block.
fn page_type_from_yaml(yaml: &str) -> Option<PageType> {
    let val: serde_yaml::Value = serde_yaml::from_str(yaml).ok()?;
    let type_str = val.get("type")?.as_str()?;
    match type_str {
        "concept" => Some(PageType::Concept),
        "source-summary" => Some(PageType::SourceSummary),
        "query-result" => Some(PageType::QueryResult),
        "contradiction" => Some(PageType::Contradiction),
        _ => None,
    }
}

/// Extract the title from a YAML frontmatter block, falling back to the slug.
fn title_from_yaml(yaml: &str) -> String {
    serde_yaml::from_str::<serde_yaml::Value>(yaml)
        .ok()
        .and_then(|v| v.get("title")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Extract the raw YAML block from a frontmatter-delimited Markdown string.
fn yaml_block(content: &str) -> Option<String> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---\n")?;
    Some(after_open[..end].to_string())
}

// ── Registry helpers ──────────────────────────────────────────────────────────

/// Try to load `~/.wiki/config.toml`.  Returns `None` if the file does not
/// exist (single-wiki mode).  Exits with an error message if the file exists
/// but is malformed.
fn load_registry() -> Option<WikiRegistry> {
    let path = global_config_path();
    if !path.exists() {
        return None;
    }
    match WikiRegistry::load(&path) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("error: failed to load {}: {e:#}", path.display());
            process::exit(1);
        }
    }
}

/// Resolve the target `WikiConfig` for a command.
///
/// Resolution:
/// 1. Try `~/.wiki/config.toml` → call `registry.resolve(wiki_name)`.
/// 2. If the file does not exist → fall back to `WikiConfig { root: cwd, name: "wiki" }`.
fn resolve_wiki_config(wiki_name: Option<&str>) -> WikiConfig {
    if let Some(registry) = load_registry() {
        match registry.resolve(wiki_name) {
            Ok(config) => return config.clone(),
            Err(e) => {
                eprintln!("error: {e:#}");
                process::exit(1);
            }
        }
    }
    // Single-wiki fallback.
    WikiConfig {
        root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        name: "wiki".into(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        // ── Phase 1 ──────────────────────────────────────────────────────────
        Command::Ingest { file } => {
            let input = if file == "-" {
                Input::Stdin
            } else {
                Input::File(PathBuf::from(&file))
            };

            let config = resolve_wiki_config(cli.wiki.as_deref());

            match ingest(input, &config).await {
                Ok(report) => {
                    println!("{report}");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // ── Phase 2 ──────────────────────────────────────────────────────────
        Command::Search {
            query,
            top,
            rebuild_index,
            all,
        } => {
            // `wiki search --all` fans out to all registered wikis.
            if all {
                let q = match query.as_deref() {
                    Some(q) if !q.trim().is_empty() => q,
                    _ => {
                        eprintln!("error: provide a search query with --all");
                        process::exit(1);
                    }
                };

                let registry = match load_registry() {
                    Some(r) => r,
                    None => {
                        eprintln!(
                            "error: --all requires a registry; \
                             run `wiki init --register` to add wikis"
                        );
                        process::exit(1);
                    }
                };

                let results = match search_all(&registry, q, top) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("error: {e:#}");
                        process::exit(1);
                    }
                };

                if results.is_empty() {
                    println!("No results.");
                } else {
                    println!(
                        "{:<20} {:<40} {:<40} {:>8}",
                        "WIKI", "SLUG", "TITLE", "SCORE"
                    );
                    println!("{}", "-".repeat(112));
                    for r in results.into_iter().take(top) {
                        println!(
                            "{:<20} {:<40} {:<40} {:>8.4}",
                            truncate(&r.wiki_name, 20),
                            truncate(&r.slug, 40),
                            truncate(&r.title, 40),
                            r.score
                        );
                    }
                }
                process::exit(0);
            }

            // Single-wiki search (existing behaviour).
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            let index_dir = wiki_root.join(".wiki").join("search-index");

            if rebuild_index && query.is_none() {
                build_index(&wiki_root, &index_dir)?;
                println!("Index rebuilt.");
                process::exit(0);
            }

            let q = match query.as_deref() {
                Some(q) if !q.trim().is_empty() => q,
                _ => {
                    eprintln!("error: provide a search query or use --rebuild-index");
                    process::exit(1);
                }
            };

            let results = search(q, &wiki_root, rebuild_index)?;

            if results.is_empty() {
                println!("No results.");
            } else {
                println!("{:<45} {:<45} {:>8}", "SLUG", "TITLE", "SCORE");
                println!("{}", "-".repeat(100));
                for r in results.into_iter().take(top) {
                    println!(
                        "{:<45} {:<45} {:>8.4}",
                        truncate(&r.slug, 45),
                        truncate(&r.title, 45),
                        r.score
                    );
                }
            }
            process::exit(0);
        }

        Command::Context { question, top_k } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            let output = context(&question, &wiki_root, top_k)?;
            print!("{output}");
            process::exit(0);
        }

        // ── Phase 3 ──────────────────────────────────────────────────────────

        // wiki lint — write LINT.md, commit, print summary.
        Command::Lint => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            match lint::lint(&wiki_root) {
                Ok(report) => {
                    println!(
                        "lint: {} orphan(s), {} missing stub(s), {} active contradiction(s)",
                        report.orphan_pages.len(),
                        report.missing_stubs.len(),
                        report.active_contradictions.len()
                    );
                    println!("LINT.md written and committed.");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // wiki contradict [--status <status>] — list contradiction pages.
        Command::Contradict { status } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;

            let filter = status.as_deref().and_then(parse_status);
            if let Some(ref raw) = status {
                if filter.is_none() {
                    eprintln!(
                        "error: unknown status `{raw}`; expected active, resolved, or under-analysis"
                    );
                    process::exit(1);
                }
            }

            match contradiction::list(&wiki_root, filter) {
                Ok(items) => {
                    if items.is_empty() {
                        println!("No contradiction pages found.");
                    } else {
                        println!(
                            "{:<50} {:<35} {:<15} {:<12}",
                            "SLUG", "TITLE", "STATUS", "DIMENSION"
                        );
                        println!("{}", "-".repeat(115));
                        for c in &items {
                            let status_str = match c.status {
                                Status::Active => "active",
                                Status::Resolved => "resolved",
                                Status::UnderAnalysis => "under-analysis",
                            };
                            let dim_str = format!("{:?}", c.dimension).to_lowercase();
                            println!(
                                "{:<50} {:<35} {:<15} {:<12}",
                                truncate(&c.slug, 50),
                                truncate(&c.title, 35),
                                status_str,
                                dim_str
                            );
                        }
                    }
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // wiki list [--type <type>] — list all wiki pages.
        Command::List { r#type } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;

            let type_filter: Option<PageType> =
                r#type.as_deref().and_then(parse_page_type);
            if let Some(ref raw) = r#type {
                if type_filter.is_none() {
                    eprintln!(
                        "error: unknown type `{raw}`; expected concept, source, contradiction, or query"
                    );
                    process::exit(1);
                }
            }

            // Walk all .md files, parse frontmatter for type/title.
            let mut rows: Vec<(String, String, String)> = Vec::new();

            for entry in walkdir::WalkDir::new(&wiki_root).follow_links(false) {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path();
                if path.extension() != Some(std::ffi::OsStr::new("md")) {
                    continue;
                }
                let rel = match path.strip_prefix(&wiki_root) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                // Skip internal directories.
                if rel.starts_with(".wiki") || rel.starts_with("raw") {
                    continue;
                }
                // Skip LINT.md (not a wiki page).
                if rel == std::path::Path::new("LINT.md") {
                    continue;
                }

                let slug = rel.with_extension("").to_string_lossy().into_owned();
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let yaml = match yaml_block(&content) {
                    Some(y) => y,
                    None => continue,
                };

                let page_type = page_type_from_yaml(&yaml);
                // Apply type filter.
                if let Some(ref ft) = type_filter {
                    if page_type.as_ref() != Some(ft) {
                        continue;
                    }
                }

                let title = title_from_yaml(&yaml);
                let type_str = match page_type {
                    Some(PageType::Concept) => "concept",
                    Some(PageType::SourceSummary) => "source-summary",
                    Some(PageType::QueryResult) => "query-result",
                    Some(PageType::Contradiction) => "contradiction",
                    None => "unknown",
                };
                rows.push((slug, title, type_str.to_string()));
            }

            rows.sort_by(|a, b| a.0.cmp(&b.0));

            if rows.is_empty() {
                println!("No pages found.");
            } else {
                println!("{:<50} {:<40} {:<15}", "SLUG", "TITLE", "TYPE");
                println!("{}", "-".repeat(107));
                for (slug, title, type_str) in &rows {
                    println!(
                        "{:<50} {:<40} {:<15}",
                        truncate(slug, 50),
                        truncate(title, 40),
                        type_str
                    );
                }
            }
            process::exit(0);
        }

        // wiki graph [--format dot|mermaid] — print concept graph.
        Command::Graph { format } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            match graph::build_graph(&wiki_root) {
                Ok(g) => {
                    let output = if format == "mermaid" {
                        graph::mermaid_output(&g)
                    } else {
                        graph::dot_output(&g)
                    };
                    println!("{output}");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // wiki diff — print the diff introduced by the last commit.
        Command::Diff => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            match git::diff_last(&wiki_root) {
                Ok(diff) => {
                    if diff.is_empty() {
                        println!("(no diff — repository has only one commit)");
                    } else {
                        print!("{diff}");
                    }
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // ── Phase 4+ ─────────────────────────────────────────────────────────

        // wiki serve [--sse ADDR] — start MCP server.
        Command::Serve { sse } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;

            if let Some(addr_str) = sse {
                // ── SSE transport (Phase 6) ───────────────────────────────────
                use rmcp::transport::sse_server::SseServer;
                use std::net::SocketAddr;

                // Parse `:8080` → `0.0.0.0:8080`; fall back to full address parse.
                let addr: SocketAddr = if let Some(port_str) = addr_str.strip_prefix(':') {
                    let port: u16 = port_str.parse().map_err(|_| {
                        anyhow::anyhow!("invalid port in SSE address: {}", addr_str)
                    })?;
                    SocketAddr::from(([0, 0, 0, 0], port))
                } else {
                    addr_str.parse().map_err(|_| {
                        anyhow::anyhow!("invalid SSE address: {}", addr_str)
                    })?
                };

                let sse_server = SseServer::serve(addr).await.map_err(|e| {
                    anyhow::anyhow!("failed to start SSE server on {}: {e}", addr)
                })?;

                eprintln!(
                    "MCP SSE server listening on {} (GET /sse  POST /message)",
                    sse_server.config.bind
                );

                // Wire up registry if available; fall back to single-wiki mode.
                let ct = match load_registry() {
                    Some(registry) => {
                        let reg = Arc::new(registry);
                        let root = reg
                            .resolve(cli.wiki.as_deref())
                            .map(|c| c.root.clone())
                            .unwrap_or(wiki_root);
                        let reg2 = reg.clone();
                        sse_server.with_service(move || {
                            llm_wiki::server::WikiServer::new_with_registry(
                                root.clone(),
                                reg2.clone(),
                            )
                        })
                    }
                    None => {
                        let root = wiki_root.clone();
                        sse_server.with_service(move || {
                            llm_wiki::server::WikiServer::new(root.clone())
                        })
                    }
                };

                // Block until Ctrl-C, then cancel the SSE server gracefully.
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        ct.cancel();
                    }
                }
            } else {
                // ── stdio transport (Phase 4) ─────────────────────────────────
                use rmcp::ServiceExt;
                use rmcp::transport::stdio;

                let server = llm_wiki::server::WikiServer::new(wiki_root);
                let service = server
                    .serve(stdio())
                    .await
                    .map_err(|e| anyhow::anyhow!("failed to start MCP server: {e}"))?;

                tokio::select! {
                    _ = service.waiting() => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
            }

            process::exit(0);
        }

        // ── Phase 5 ──────────────────────────────────────────────────────────

        // wiki init [PATH] [--register] — initialise a new wiki repository.
        Command::Init { path, register } => {
            let root = match path {
                Some(p) => PathBuf::from(p),
                None => std::env::current_dir()?,
            };
            match init_wiki(&root) {
                Ok(()) => {
                    println!("Wiki initialised at {}", root.display());

                    // --register: add this wiki to ~/.wiki/config.toml.
                    if register {
                        let wiki_name = root
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "wiki".to_string());
                        let config_path = global_config_path();
                        match register_wiki(&wiki_name, &root, &config_path) {
                            Ok(()) => {
                                println!(
                                    "Registered '{}' in {}",
                                    wiki_name,
                                    config_path.display()
                                );
                            }
                            Err(e) => {
                                eprintln!("warning: failed to register wiki: {e:#}");
                            }
                        }
                    }

                    println!();
                    println!(
                        "Next: add the MCP server to your Claude Code config \
                         (~/.claude/settings.json):"
                    );
                    println!();
                    println!("{}", mcp_config_snippet(&root));
                    println!();
                    println!(
                        "Then run `/llm-wiki:init` in Claude Code to complete setup."
                    );
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    process::exit(1);
                }
            }
        }

        // wiki instruct [WORKFLOW] — print LLM usage instructions.
        //
        // Without an argument: full instructions.md.
        // With an argument:    only the `## {workflow}-workflow` section.
        Command::Instruct { workflow } => {
            const INSTRUCTIONS: &str = include_str!("instructions.md");

            match workflow.as_deref() {
                None | Some("") => {
                    print!("{INSTRUCTIONS}");
                }
                Some(name) => {
                    let header = format!("## {name}-workflow");
                    match INSTRUCTIONS.find(header.as_str()) {
                        Some(start) => {
                            let section = &INSTRUCTIONS[start..];
                            // Cut at the start of the next `## ` heading.
                            let end = section[header.len()..]
                                .find("\n## ")
                                .map(|pos| header.len() + pos)
                                .unwrap_or(section.len());
                            print!("{}", &section[..end]);
                        }
                        None => {
                            eprintln!(
                                "error: unknown workflow `{name}`; \
                                 available: help, init, ingest, research, lint, contradiction"
                            );
                            process::exit(1);
                        }
                    }
                }
            }

            process::exit(0);
        }

        // wiki read <slug> [--body-only] — print page content.
        Command::Read { slug, body_only } => {
            let wiki_root = resolve_wiki_config(cli.wiki.as_deref()).root;
            match resolve_slug(&wiki_root, &slug) {
                None => {
                    eprintln!("error: page not found: `{slug}`");
                    process::exit(1);
                }
                Some(path) => {
                    let content = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("error: failed to read {}: {e}", path.display());
                            process::exit(1);
                        }
                    };
                    if body_only {
                        use llm_wiki::markdown::parse_frontmatter;
                        match parse_frontmatter(&content) {
                            Ok((_fm, body)) => print!("{body}"),
                            Err(_) => print!("{content}"),
                        }
                    } else {
                        print!("{content}");
                    }
                    process::exit(0);
                }
            }
        }
    }
}
