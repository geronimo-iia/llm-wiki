use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use llm_wiki::cli::{
    Cli, Commands, ConfigAction, IndexAction, LintAction, NewAction, SpacesAction, INSTRUCTIONS,
};
use llm_wiki::config;
use llm_wiki::git;
use llm_wiki::graph;
use llm_wiki::ingest;
use llm_wiki::lint;
use llm_wiki::markdown;
use llm_wiki::search;
use llm_wiki::server;
use llm_wiki::spaces;

fn global_config_path() -> PathBuf {
    dirs_path()
}

fn dirs_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".wiki").join("config.toml")
}

fn index_path_for(wiki_name: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".wiki")
        .join("indexes")
        .join(wiki_name)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "llm_wiki=info,warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let config_path = global_config_path();

    match cli.command {
        Commands::Serve { sse, acp, dry_run } => {
            let global = config::load_global(&config_path)?;
            let wiki_cfg = if let Some(entry) = global
                .wikis
                .iter()
                .find(|w| w.name == global.global.default_wiki)
            {
                config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default()
            } else {
                config::WikiConfig::default()
            };
            let resolved = config::resolve(&global, &wiki_cfg);

            let sse_enabled = sse.is_some() || resolved.serve.sse;
            let sse_port = sse
                .flatten()
                .and_then(|s| s.trim_start_matches(':').parse::<u16>().ok())
                .unwrap_or(resolved.serve.sse_port);
            let acp_enabled = acp || resolved.serve.acp;

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(server::serve(
                global,
                config_path,
                sse_enabled,
                sse_port,
                acp_enabled,
                dry_run,
            ))?;
        }
        Commands::Instruct { workflow } => {
            if let Some(name) = workflow {
                match llm_wiki::cli::extract_workflow(INSTRUCTIONS, &name) {
                    Some(section) => println!("{section}"),
                    None => eprintln!("Unknown workflow: {name}"),
                }
            } else {
                println!("{INSTRUCTIONS}");
            }
        }
        Commands::Init {
            path,
            name,
            description,
            force,
            set_default,
        } => {
            let path = PathBuf::from(&path);
            let report = llm_wiki::init::init(
                &path,
                &name,
                description.as_deref(),
                force,
                set_default,
                &config_path,
            )?;
            if report.created {
                println!("Created wiki \"{}\" at {}", report.name, report.path);
            } else {
                println!("Wiki \"{}\" at {} already exists", report.name, report.path);
            }
            if report.registered {
                println!("Registered in {}", config_path.display());
            }
            if report.committed {
                println!("Initial commit: init: {}", report.name);
            }
        }
        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                let global = config::load_global(&config_path)?;
                let resolved = config::resolve(&global, &config::WikiConfig::default());
                let value = get_config_value(&resolved, &global, &key);
                println!("{value}");
            }
            ConfigAction::Set {
                key,
                value,
                global: is_global,
                wiki: wiki_name,
            } => {
                if is_global {
                    let mut global = config::load_global(&config_path)?;
                    config::set_global_config_value(&mut global, &key, &value)?;
                    config::save_global(&global, &config_path)?;
                    println!("Set {key} = {value} (global)");
                } else {
                    let global = config::load_global(&config_path)?;
                    let name = wiki_name.as_deref().unwrap_or(&global.global.default_wiki);
                    let entry = spaces::resolve_name(name, &global)?;
                    let entry_path = PathBuf::from(&entry.path);
                    let mut wiki_cfg = config::load_wiki(&entry_path)?;
                    config::set_wiki_config_value(&mut wiki_cfg, &key, &value)?;
                    config::save_wiki(&wiki_cfg, &entry_path)?;
                    println!("Set {key} = {value} (wiki: {name})");
                }
            }
            ConfigAction::List {
                global: is_global,
                wiki: _wiki,
            } => {
                let global = config::load_global(&config_path)?;
                if is_global {
                    let toml_str = toml::to_string_pretty(&global)?;
                    println!("{toml_str}");
                } else {
                    let resolved = config::resolve(&global, &config::WikiConfig::default());
                    let toml_str = toml::to_string_pretty(&resolved)?;
                    println!("{toml_str}");
                }
            }
        },
        Commands::Ingest { path, dry_run } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let wiki_root = PathBuf::from(&entry.path).join("wiki");
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path))?;
            let resolved = config::resolve(&global, &wiki_cfg);
            let schema = config::load_schema(&PathBuf::from(&entry.path))?;

            let opts = ingest::IngestOptions { dry_run };
            let report = ingest::ingest(
                std::path::Path::new(&path),
                &opts,
                &wiki_root,
                &schema,
                &resolved.validation,
            )?;

            println!(
                "Ingested: {} pages, {} assets, {} warnings",
                report.pages_validated,
                report.assets_found,
                report.warnings.len()
            );
            for w in &report.warnings {
                println!("  warn: {w}");
            }
            if dry_run {
                println!("(dry run — nothing committed)");
            } else {
                println!("Commit: {}", report.commit);
                if resolved.index.auto_rebuild {
                    let index_path = index_path_for(wiki_name);
                    let repo_root = PathBuf::from(&entry.path);
                    match search::rebuild_index(&wiki_root, &index_path, wiki_name, &repo_root) {
                        Ok(r) => println!("Index rebuilt: {} pages in {}ms", r.pages_indexed, r.duration_ms),
                        Err(e) => eprintln!("warning: index rebuild failed: {e}"),
                    }
                } else {
                    eprintln!("warning: search index is stale — run `wiki index rebuild`");
                }
            }
        }
        Commands::New { action } => {
            let global = config::load_global(&config_path)?;
            match action {
                NewAction::Page {
                    uri,
                    bundle,
                    dry_run,
                } => {
                    let (entry, slug) = spaces::resolve_uri(&uri, &global)?;
                    let wiki_root = PathBuf::from(&entry.path).join("wiki");
                    let repo_root = PathBuf::from(&entry.path);

                    if dry_run {
                        let form = if bundle { "bundle" } else { "flat" };
                        println!("Would create {form} page at wiki://{}/{slug}", entry.name);
                    } else {
                        let path = markdown::create_page(&slug, bundle, &wiki_root)?;
                        git::commit(&repo_root, &format!("new: {uri}"))?;
                        println!("Created: {}", path.display());
                    }
                }
                NewAction::Section { uri, dry_run } => {
                    let (entry, slug) = spaces::resolve_uri(&uri, &global)?;
                    let wiki_root = PathBuf::from(&entry.path).join("wiki");
                    let repo_root = PathBuf::from(&entry.path);

                    if dry_run {
                        println!("Would create section at wiki://{}/{slug}", entry.name);
                    } else {
                        let path = markdown::create_section(&slug, &wiki_root)?;
                        git::commit(&repo_root, &format!("new: {uri}"))?;
                        println!("Created: {}", path.display());
                    }
                }
            }
        }
        Commands::Search {
            query,
            no_excerpt,
            top_k,
            include_sections,
            all,
            dry_run,
        } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path))?;
            let resolved = config::resolve(&global, &wiki_cfg);

            let effective_top_k = top_k.unwrap_or(resolved.defaults.search_top_k as usize);

            if dry_run {
                println!("query: {query}");
                if all {
                    println!("wiki:  (all)");
                } else {
                    println!("wiki:  {wiki_name}");
                }
                return Ok(());
            }

            let opts = search::SearchOptions {
                no_excerpt,
                include_sections,
                top_k: effective_top_k,
                ..Default::default()
            };

            let results = if all {
                let wikis: Vec<(String, PathBuf)> = global
                    .wikis
                    .iter()
                    .map(|w| (w.name.clone(), index_path_for(&w.name)))
                    .collect();
                search::search_all(&query, &opts, &wikis)?
            } else {
                let repo_root = PathBuf::from(&entry.path);
                let index_path = index_path_for(wiki_name);

                // Staleness check
                if let Ok(status) = search::index_status(wiki_name, &index_path, &repo_root) {
                    if status.stale && resolved.index.auto_rebuild {
                        let wiki_root = repo_root.join("wiki");
                        search::rebuild_index(&wiki_root, &index_path, wiki_name, &repo_root)?;
                    } else if status.stale {
                        eprintln!("warning: search index is stale — run `wiki index rebuild`");
                    }
                }

                search::search(&query, &opts, &index_path, wiki_name)?
            };
            for r in &results {
                println!("slug:  {}", r.slug);
                println!("uri:   {}", r.uri);
                println!("title: {}", r.title);
                println!("score: {:.2}", r.score);
                if let Some(ref excerpt) = r.excerpt {
                    println!("excerpt: {excerpt}");
                }
                println!();
            }
        }
        Commands::Read {
            uri,
            no_frontmatter,
            list_assets,
        } => {
            let global = config::load_global(&config_path)?;
            let (entry, slug) = if uri.starts_with("wiki://") {
                spaces::resolve_uri(&uri, &global)?
            } else {
                let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
                let entry = spaces::resolve_name(wiki_name, &global)?;
                (entry, uri.clone())
            };
            let wiki_root = PathBuf::from(&entry.path).join("wiki");

            if list_assets {
                let assets = markdown::list_assets(&slug, &wiki_root)?;
                for a in &assets {
                    println!("{a}");
                }
            } else {
                match markdown::resolve_read_target(&slug, &wiki_root)? {
                    markdown::ReadTarget::Page(_) => {
                        let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path))?;
                        let resolved = config::resolve(&global, &wiki_cfg);
                        let strip = no_frontmatter || resolved.read.no_frontmatter;
                        let content = markdown::read_page(&slug, &wiki_root, strip)?;
                        print!("{content}");
                    }
                    markdown::ReadTarget::Asset(parent_slug, filename) => {
                        let bytes = markdown::read_asset(&parent_slug, &filename, &wiki_root)?;
                        use std::io::Write;
                        std::io::stdout().write_all(&bytes)?;
                    }
                }
            }
        }
        Commands::List {
            r#type,
            status,
            page,
            page_size,
        } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path))?;
            let resolved = config::resolve(&global, &wiki_cfg);
            let repo_root = PathBuf::from(&entry.path);
            let index_path = index_path_for(wiki_name);

            // Staleness check
            if let Ok(st) = search::index_status(wiki_name, &index_path, &repo_root) {
                if st.stale && resolved.index.auto_rebuild {
                    let wiki_root = repo_root.join("wiki");
                    search::rebuild_index(&wiki_root, &index_path, wiki_name, &repo_root)?;
                } else if st.stale {
                    eprintln!("warning: search index is stale — run `wiki index rebuild`");
                }
            }

            let opts = search::ListOptions {
                r#type,
                status,
                page,
                page_size: page_size.unwrap_or(resolved.defaults.list_page_size as usize),
            };
            let result = search::list(&opts, &index_path, wiki_name)?;
            for p in &result.pages {
                println!(
                    "{:<40} {:<16} {:<8} {}",
                    p.slug, p.r#type, p.status, p.title
                );
            }
            println!(
                "\nPage {}/{} ({} total)",
                result.page,
                (result.total + result.page_size - 1) / result.page_size.max(1),
                result.total
            );
        }
        Commands::Index { action } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let repo_root = PathBuf::from(&entry.path);
            let wiki_root = repo_root.join("wiki");
            let index_path = index_path_for(wiki_name);

            match action {
                IndexAction::Rebuild { dry_run } => {
                    if dry_run {
                        let count = walkdir::WalkDir::new(&wiki_root)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().is_file()
                                    && e.path().extension().and_then(|x| x.to_str()) == Some("md")
                            })
                            .count();
                        println!("Would index {count} pages from {}", wiki_root.display());
                    } else {
                        let report =
                            search::rebuild_index(&wiki_root, &index_path, wiki_name, &repo_root)?;
                        println!(
                            "Indexed {} pages in {}ms",
                            report.pages_indexed, report.duration_ms
                        );
                    }
                }
                IndexAction::Status => {
                    let status = search::index_status(wiki_name, &index_path, &repo_root)?;
                    println!("wiki:     {}", status.wiki);
                    println!("path:     {}", status.path);
                    println!("built:    {}", status.built.as_deref().unwrap_or("never"));
                    println!("pages:    {}", status.pages);
                    println!("sections: {}", status.sections);
                    println!("stale:    {}", if status.stale { "yes" } else { "no" });
                }
            }
        }
        Commands::Lint { action, dry_run } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let entry_path = PathBuf::from(&entry.path);
            let wiki_root = entry_path.join("wiki");
            let wiki_cfg = config::load_wiki(&entry_path)?;
            let resolved = config::resolve(&global, &wiki_cfg);

            match action {
                Some(LintAction::Fix {
                    only,
                    dry_run: fix_dry_run,
                }) => {
                    if fix_dry_run || dry_run {
                        let report = lint::lint(&wiki_root, &resolved, wiki_name)?;
                        println!(
                            "Would fix: {} missing stubs, {} empty sections",
                            report.missing_stubs.len(),
                            report.empty_sections.len()
                        );
                    } else {
                        let report =
                            lint::lint_fix(&wiki_root, &resolved, only.as_deref(), wiki_name)?;
                        println!(
                            "Fixed: {} missing stubs, {} empty sections",
                            report.missing_stubs.len(),
                            report.empty_sections.len()
                        );
                    }
                }
                None => {
                    let report = lint::lint(&wiki_root, &resolved, wiki_name)?;
                    if dry_run {
                        println!(
                            "Lint report: {} orphans, {} stubs, {} empty sections",
                            report.orphans.len(),
                            report.missing_stubs.len(),
                            report.empty_sections.len()
                        );
                    } else {
                        lint::write_lint_md(&report, &entry_path)?;
                        let date = &report.date;
                        let msg = format!(
                            "lint: {date} \u{2014} {} orphans, {} stubs, {} empty sections",
                            report.orphans.len(),
                            report.missing_stubs.len(),
                            report.empty_sections.len()
                        );
                        git::commit(&entry_path, &msg)?;
                        println!("Wrote LINT.md \u{2014} {msg}");
                    }
                }
            }
        }
        Commands::Graph {
            format,
            root,
            depth,
            r#type,
            output,
            dry_run,
        } => {
            let global = config::load_global(&config_path)?;
            let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
            let entry = spaces::resolve_name(wiki_name, &global)?;
            let entry_path = PathBuf::from(&entry.path);
            let wiki_root = entry_path.join("wiki");
            let wiki_cfg = config::load_wiki(&entry_path)?;
            let resolved = config::resolve(&global, &wiki_cfg);

            let fmt = format.unwrap_or_else(|| resolved.graph.format.clone());
            let types: Vec<String> = r#type
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            let filter = graph::GraphFilter {
                root,
                depth: depth.or(Some(resolved.graph.depth as usize)),
                types,
            };
            let g = graph::build_graph(&wiki_root, &filter);

            let rendered = match fmt.as_str() {
                "dot" => graph::render_dot(&g),
                _ => graph::render_mermaid(&g),
            };

            if dry_run {
                println!(
                    "Would output {} nodes, {} edges ({fmt} format)",
                    g.node_count(),
                    g.edge_count()
                );
                return Ok(());
            }

            if let Some(ref out_path) = output {
                let content = if out_path.ends_with(".md") {
                    graph::wrap_graph_md(&rendered, &fmt, &filter)
                } else {
                    rendered
                };
                std::fs::write(out_path, &content)?;

                // Auto-commit if inside repo root
                let out_canonical = std::fs::canonicalize(out_path).ok();
                let repo_canonical = entry_path.canonicalize().ok();
                let committed = if let (Some(out_c), Some(repo_c)) = (out_canonical, repo_canonical) {
                    if out_c.starts_with(&repo_c) {
                        let date = chrono::Local::now().format("%Y-%m-%d");
                        let msg = format!(
                            "graph: {date} \u{2014} {} nodes, {} edges",
                            g.node_count(),
                            g.edge_count()
                        );
                        git::commit(&entry_path, &msg).is_ok()
                    } else {
                        false
                    }
                } else {
                    false
                };

                println!("Wrote graph to {out_path}");
                if committed {
                    println!("Committed.");
                }
            } else {
                print!("{rendered}");
            }
        }
        Commands::Spaces { action } => match action {
            SpacesAction::List => {
                let global = config::load_global(&config_path)?;
                let entries = spaces::load_all(&global);
                if entries.is_empty() {
                    println!("No wikis registered.");
                } else {
                    println!("  {:<12} {:<40} description", "name", "path");
                    for e in &entries {
                        let marker = if e.name == global.global.default_wiki {
                            "*"
                        } else {
                            " "
                        };
                        let desc = e.description.as_deref().unwrap_or("—");
                        println!("{marker} {:<12} {:<40} {desc}", e.name, e.path);
                    }
                }
            }
            SpacesAction::Remove { name, delete } => {
                spaces::remove(&name, delete, &config_path)?;
                println!("Removed wiki \"{name}\"");
                if delete {
                    println!("Deleted wiki directory");
                }
            }
            SpacesAction::SetDefault { name } => {
                spaces::set_default(&name, &config_path)?;
                println!("Default wiki set to \"{name}\"");
            }
        },
    }

    Ok(())
}

fn get_config_value(
    resolved: &config::ResolvedConfig,
    global: &config::GlobalConfig,
    key: &str,
) -> String {
    match key {
        "global.default_wiki" => global.global.default_wiki.clone(),
        "defaults.search_top_k" => resolved.defaults.search_top_k.to_string(),
        "defaults.search_excerpt" => resolved.defaults.search_excerpt.to_string(),
        "defaults.search_sections" => resolved.defaults.search_sections.to_string(),
        "defaults.page_mode" => resolved.defaults.page_mode.clone(),
        "defaults.list_page_size" => resolved.defaults.list_page_size.to_string(),
        "read.no_frontmatter" => resolved.read.no_frontmatter.to_string(),
        "index.auto_rebuild" => resolved.index.auto_rebuild.to_string(),
        "graph.format" => resolved.graph.format.clone(),
        "graph.depth" => resolved.graph.depth.to_string(),
        "graph.output" => resolved.graph.output.clone(),
        "serve.sse" => resolved.serve.sse.to_string(),
        "serve.sse_port" => resolved.serve.sse_port.to_string(),
        "serve.acp" => resolved.serve.acp.to_string(),
        "validation.type_strictness" => resolved.validation.type_strictness.clone(),
        "lint.fix_missing_stubs" => resolved.lint.fix_missing_stubs.to_string(),
        "lint.fix_empty_sections" => resolved.lint.fix_empty_sections.to_string(),
        _ => format!("unknown key: {key}"),
    }
}


