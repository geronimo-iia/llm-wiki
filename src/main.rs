use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;

use llm_wiki::cli::{
    Cli, Commands, ConfigAction, ContentAction, IndexAction, SpacesAction,
};
use llm_wiki::config;
use llm_wiki::engine::EngineManager;
use llm_wiki::git;
use llm_wiki::graph;
use llm_wiki::ingest;
use llm_wiki::markdown;
use llm_wiki::search;
use llm_wiki::slug::{resolve_read_target, ReadTarget, Slug, WikiUri};
use llm_wiki::spaces;

fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".llm-wiki").join("config.toml")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = global_config_path();

    let _log_guard = init_logging(&cli.command, &config_path);

    match cli.command {
        // ── Spaces ────────────────────────────────────────────────────
        Commands::Spaces { action } => match action {
            SpacesAction::Create {
                path,
                name,
                description,
                force,
                set_default,
            } => {
                let report = spaces::create(
                    &PathBuf::from(&path),
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
                    println!("Initial commit: create: {}", report.name);
                }
            }
            SpacesAction::List { format } => {
                let global = config::load_global(&config_path)?;
                let entries = spaces::load_all(&global);
                if is_json(&format) {
                    println!("{}", serde_json::to_string_pretty(&entries)?);
                } else if entries.is_empty() {
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
                spaces::set_default_wiki(&name, &config_path)?;
                println!("Default wiki set to \"{name}\"");
            }
        },

        // ── Config ────────────────────────────────────────────────────
        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                let global = config::load_global(&config_path)?;
                let resolved = config::resolve(&global, &config::WikiConfig::default());
                println!("{}", config::get_config_value(&resolved, &global, &key));
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
                wiki: _,
                format,
            } => {
                let global = config::load_global(&config_path)?;
                if is_global {
                    let s = toml::to_string_pretty(&global)?;
                    println!("{s}");
                } else {
                    let resolved = config::resolve(&global, &config::WikiConfig::default());
                    if is_json(&format) {
                        println!("{}", serde_json::to_string_pretty(&resolved)?);
                    } else {
                        println!("{}", toml::to_string_pretty(&resolved)?);
                    }
                }
            }
        },

        // ── Content ───────────────────────────────────────────────────
        Commands::Content { action } => match action {
            ContentAction::Read {
                uri,
                no_frontmatter,
                list_assets,
            } => {
                let global = config::load_global(&config_path)?;
                let (entry, slug) = WikiUri::resolve(&uri, cli.wiki.as_deref(), &global)?;
                let wiki_root = PathBuf::from(&entry.path).join("wiki");

                if list_assets {
                    let assets = markdown::list_assets(&slug, &wiki_root)?;
                    for a in &assets {
                        println!("{a}");
                    }
                } else {
                    match resolve_read_target(slug.as_str(), &wiki_root)? {
                        ReadTarget::Page(_) => {
                            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path))?;
                            let resolved = config::resolve(&global, &wiki_cfg);
                            let strip = no_frontmatter || resolved.read.no_frontmatter;
                            let content = markdown::read_page(&slug, &wiki_root, strip)?;
                            print!("{content}");
                        }
                        ReadTarget::Asset(parent_slug, filename) => {
                            let parent = Slug::try_from(parent_slug.as_str())?;
                            let bytes = markdown::read_asset(&parent, &filename, &wiki_root)?;
                            use std::io::Write;
                            std::io::stdout().write_all(&bytes)?;
                        }
                    }
                }
            }
            ContentAction::Write { uri, file } => {
                let global = config::load_global(&config_path)?;
                let (entry, _slug) = WikiUri::resolve(&uri, cli.wiki.as_deref(), &global)?;
                let wiki_root = PathBuf::from(&entry.path).join("wiki");

                let content = if let Some(ref path) = file {
                    std::fs::read_to_string(path)?
                } else {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf)?;
                    buf
                };

                // Extract the slug part from the URI for write_page
                let slug_str = if uri.starts_with("wiki://") {
                    let parsed = WikiUri::parse(&uri)?;
                    parsed.slug.as_str().to_string()
                } else {
                    uri.clone()
                };

                let path = markdown::write_page(&slug_str, &content, &wiki_root)?;
                println!("Wrote {} bytes to {}", content.len(), path.display());
            }
            ContentAction::New {
                uri,
                section,
                bundle,
                name,
                r#type,
                dry_run,
            } => {
                let global = config::load_global(&config_path)?;
                let (entry, slug) = WikiUri::resolve(&uri, cli.wiki.as_deref(), &global)?;
                let wiki_root = PathBuf::from(&entry.path).join("wiki");

                if dry_run {
                    let kind = if section { "section" } else if bundle { "bundle" } else { "flat" };
                    println!("Would create {kind} at wiki://{}/{slug}", entry.name);
                } else if section {
                    let path = markdown::create_section(&slug, &wiki_root)?;
                    println!("Created: {}", path.display());
                } else {
                    let path = markdown::create_page(
                        &slug,
                        bundle,
                        &wiki_root,
                        name.as_deref(),
                        r#type.as_deref(),
                    )?;
                    println!("Created: {}", path.display());
                }
            }
            ContentAction::Commit {
                slugs,
                all,
                message,
            } => {
                let global = config::load_global(&config_path)?;
                let wiki_name = cli.wiki.as_deref().unwrap_or(&global.global.default_wiki);
                let entry = spaces::resolve_name(wiki_name, &global)?;
                let repo_root = PathBuf::from(&entry.path);
                let wiki_root = repo_root.join("wiki");

                if slugs.is_empty() && !all {
                    anyhow::bail!("specify slugs or --all");
                }

                let hash = if all {
                    let msg = message.unwrap_or_else(|| "commit: all".into());
                    git::commit(&repo_root, &msg)?
                } else {
                    let mut paths = Vec::new();
                    for s in &slugs {
                        let slug = Slug::try_from(s.as_str())?;
                        let resolved = slug.resolve(&wiki_root)?;
                        if resolved.file_name() == Some(std::ffi::OsStr::new("index.md")) {
                            let bundle_dir = resolved.parent().unwrap();
                            for entry in walkdir::WalkDir::new(bundle_dir)
                                .into_iter()
                                .filter_map(|e| e.ok())
                            {
                                if entry.path().is_file() {
                                    paths.push(entry.path().to_path_buf());
                                }
                            }
                        } else {
                            paths.push(resolved);
                        }
                    }
                    let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
                    let msg = message.unwrap_or_else(|| format!("commit: {}", slugs.join(", ")));
                    git::commit_paths(&repo_root, &path_refs, &msg)?
                };

                if hash.is_empty() {
                    println!("Nothing to commit");
                } else {
                    println!("{hash}");
                }
            }
        },

        // ── Search ────────────────────────────────────────────────────
        Commands::Search {
            query,
            r#type,
            no_excerpt,
            top_k,
            include_sections,
            all,
            format,
        } => {
            let manager = EngineManager::build(&config_path)?;
            let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());
            let space = engine.space(wiki_name)?;
            let resolved = space.resolved_config(&engine.config);

            let opts = search::SearchOptions {
                no_excerpt,
                include_sections,
                top_k: top_k.unwrap_or(resolved.defaults.search_top_k as usize),
                r#type,
            };

            let results = if all {
                let wikis: Vec<(String, PathBuf)> = engine
                    .spaces
                    .values()
                    .map(|s| (s.name.clone(), s.index_path.clone()))
                    .collect();
                search::search_all(&query, &opts, &wikis, &space.schema)?
            } else {
                search::search(
                    &query,
                    &opts,
                    &space.index_path,
                    wiki_name,
                    &space.schema,
                    None,
                )?
            };

            if is_json(&format) {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
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
        }

        // ── List ──────────────────────────────────────────────────────
        Commands::List {
            r#type,
            status,
            page,
            page_size,
            format,
        } => {
            let manager = EngineManager::build(&config_path)?;
            let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());
            let space = engine.space(wiki_name)?;
            let resolved = space.resolved_config(&engine.config);

            let opts = search::ListOptions {
                r#type,
                status,
                page,
                page_size: page_size.unwrap_or(resolved.defaults.list_page_size as usize),
            };
            let result = search::list(&opts, &space.index_path, wiki_name, &space.schema, None)?;

            if is_json(&format) {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
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
        }

        // ── Ingest ────────────────────────────────────────────────────
        Commands::Ingest {
            path,
            dry_run,
            format,
        } => {
            let manager = EngineManager::build(&config_path)?;
            let wiki_name = {
                let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
                engine.resolve_wiki_name(cli.wiki.as_deref()).to_string()
            };
            {
                let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let space = engine.space(&wiki_name)?;
                let resolved = space.resolved_config(&engine.config);

                let opts = ingest::IngestOptions {
                    dry_run,
                    auto_commit: resolved.ingest.auto_commit,
                };
                let report = ingest::ingest(
                    std::path::Path::new(&path),
                    &opts,
                    &space.wiki_root,
                    &engine.type_registry,
                    &resolved.validation,
                )?;

                if is_json(&format) {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
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
                    } else if !report.commit.is_empty() {
                        println!("Commit: {}", report.commit);
                    }
                }
            }

            // Index update after ingest (lock released above)
            if !dry_run {
                match manager.on_ingest(&wiki_name) {
                    Ok(r) => {
                        tracing::debug!(updated = r.updated, deleted = r.deleted, "index updated");
                    }
                    Err(e) => {
                        eprintln!("warning: index update failed ({e}), run `llm-wiki index rebuild`");
                    }
                }
            }
        }

        // ── Graph ─────────────────────────────────────────────────────
        Commands::Graph {
            format,
            root,
            depth,
            r#type,
            relation,
            output,
        } => {
            let manager = EngineManager::build(&config_path)?;
            let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());
            let space = engine.space(wiki_name)?;
            let resolved = space.resolved_config(&engine.config);

            let fmt = format.unwrap_or_else(|| resolved.graph.format.clone());
            let types: Vec<String> = r#type
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            let filter = graph::GraphFilter {
                root,
                depth: depth.or(Some(resolved.graph.depth as usize)),
                types,
                relation,
            };
            let g = graph::build_graph(&space.index_path, &space.schema, &filter)?;

            let rendered = match fmt.as_str() {
                "dot" => graph::render_dot(&g),
                _ => graph::render_mermaid(&g),
            };

            if let Some(ref out_path) = output {
                let content = if out_path.ends_with(".md") {
                    graph::wrap_graph_md(&rendered, &fmt, &filter)
                } else {
                    rendered
                };
                std::fs::write(out_path, &content)?;
                println!("Wrote graph to {out_path}");
            } else {
                print!("{rendered}");
            }
        }

        // ── Index ─────────────────────────────────────────────────────
        Commands::Index { action } => match action {
            IndexAction::Rebuild { dry_run, format } => {
                let manager = EngineManager::build(&config_path)?;
                let wiki_name = {
                    let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    engine.resolve_wiki_name(cli.wiki.as_deref()).to_string()
                };

                if dry_run {
                    let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    let space = engine.space(&wiki_name)?;
                    let count = walkdir::WalkDir::new(&space.wiki_root)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().is_file()
                                && e.path().extension().and_then(|x| x.to_str()) == Some("md")
                        })
                        .count();
                    println!("Would index {count} pages from {}", space.wiki_root.display());
                } else {
                    let report = manager.rebuild_index(&wiki_name)?;
                    if is_json(&format) {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        println!(
                            "Indexed {} pages in {}ms",
                            report.pages_indexed, report.duration_ms
                        );
                    }
                }
            }
            IndexAction::Status { format } => {
                let manager = EngineManager::build(&config_path)?;
                let engine = manager.engine.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());
                let space = engine.space(wiki_name)?;

                let status =
                    search::index_status(wiki_name, &space.index_path, &space.repo_root)?;

                if is_json(&format) {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!("wiki:      {}", status.wiki);
                    println!("path:      {}", status.path);
                    println!("built:     {}", status.built.as_deref().unwrap_or("never"));
                    println!("pages:     {}", status.pages);
                    println!("sections:  {}", status.sections);
                    println!("stale:     {}", if status.stale { "yes" } else { "no" });
                    println!("openable:  {}", if status.openable { "yes" } else { "no" });
                    println!("queryable: {}", if status.queryable { "yes" } else { "no" });
                }
            }
        },

        // ── Serve ─────────────────────────────────────────────────────
        Commands::Serve { sse, acp, dry_run } => {
            if dry_run {
                let mut transports = vec!["stdio".to_string()];
                if sse.is_some() {
                    transports.push("sse".to_string());
                }
                if acp {
                    transports.push("acp".to_string());
                }
                println!("Would start: [{}]", transports.join("] ["));
                return Ok(());
            }
            // Serve implementation comes in Steps 14-16
            eprintln!("serve not yet implemented — coming in Steps 14-16");
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_json(format: &Option<String>) -> bool {
    format.as_deref() == Some("json")
}

fn init_logging(
    command: &Commands,
    config_path: &std::path::Path,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::prelude::*;

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "llm_wiki=info,warn".into());

    let is_serve = matches!(command, Commands::Serve { .. });

    if !is_serve {
        tracing_subscriber::fmt()
            .compact()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .init();
        return None;
    }

    let logging_cfg = config::load_global(config_path)
        .map(|g| g.logging)
        .unwrap_or_default();

    if logging_cfg.log_path.is_empty() {
        if logging_cfg.log_format == "json" {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        } else {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
        return None;
    }

    let log_path = std::path::PathBuf::from(&logging_cfg.log_path);
    if let Err(e) = std::fs::create_dir_all(&log_path) {
        eprintln!(
            "warning: failed to create log directory {}: {e}",
            log_path.display()
        );
        tracing_subscriber::fmt()
            .compact()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .init();
        return None;
    }

    let rotation = match logging_cfg.log_rotation.as_str() {
        "hourly" => tracing_appender::rolling::Rotation::HOURLY,
        "never" => tracing_appender::rolling::Rotation::NEVER,
        _ => tracing_appender::rolling::Rotation::DAILY,
    };

    let mut builder = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(rotation)
        .filename_prefix("wiki")
        .filename_suffix("log");

    if logging_cfg.log_max_files > 0 {
        builder = builder.max_log_files(logging_cfg.log_max_files as usize);
    }

    let file_appender = match builder.build(&log_path) {
        Ok(appender) => appender,
        Err(e) => {
            eprintln!(
                "warning: failed to create log file in {}: {e}",
                log_path.display()
            );
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
            return None;
        }
    };

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    if logging_cfg.log_format == "json" {
        let stderr_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_writer(std::io::stderr);
        let file_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_writer(non_blocking);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(file_layer)
            .init();
    } else {
        let stderr_layer = tracing_subscriber::fmt::layer()
            .compact()
            .with_writer(std::io::stderr);
        let file_layer = tracing_subscriber::fmt::layer()
            .compact()
            .with_writer(non_blocking);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .with(file_layer)
            .init();
    }

    Some(guard)
}
