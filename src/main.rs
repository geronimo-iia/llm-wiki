use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;

use llm_wiki::cli::{
    Cli, Commands, ConfigAction, ContentAction, IndexAction, LogsAction, SchemaAction, SpacesAction,
};
use llm_wiki::config;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

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
                let report = ops::spaces_create(
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
            SpacesAction::List { name, format } => {
                let global = config::load_global(&config_path)?;
                let entries = ops::spaces_list(&global, name.as_deref());
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
                ops::spaces_remove(&name, delete, &config_path)?;
                println!("Removed wiki \"{name}\"");
                if delete {
                    println!("Deleted wiki directory");
                }
            }
            SpacesAction::SetDefault { name } => {
                ops::spaces_set_default(&name, &config_path)?;
                println!("Default wiki set to \"{name}\"");
            }
        },

        // ── Config ────────────────────────────────────────────────────
        Commands::Config { action } => match action {
            ConfigAction::Get { key } => {
                let val = ops::config_get(&config_path, &key)?;
                println!("{val}");
            }
            ConfigAction::Set {
                key,
                value,
                global: is_global,
                wiki: wiki_name,
            } => {
                let msg =
                    ops::config_set(&config_path, &key, &value, is_global, wiki_name.as_deref())?;
                println!("{msg}");
            }
            ConfigAction::List {
                global: is_global,
                wiki: _,
                format,
            } => {
                if is_global {
                    let s = ops::config_list_global(&config_path)?;
                    println!("{s}");
                } else {
                    let resolved = ops::config_list_resolved(&config_path)?;
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
                let manager = WikiEngine::build(&config_path)?;
                let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;

                match ops::content_read(
                    &engine,
                    &uri,
                    cli.wiki.as_deref(),
                    no_frontmatter,
                    list_assets,
                )? {
                    ops::ContentReadResult::Page(content) => print!("{content}"),
                    ops::ContentReadResult::Assets(assets) => {
                        for a in &assets {
                            println!("{a}");
                        }
                    }
                    ops::ContentReadResult::Binary => {
                        anyhow::bail!("asset is binary — access it directly from the filesystem");
                    }
                }
            }
            ContentAction::Write { uri, file } => {
                let content = if let Some(ref path) = file {
                    std::fs::read_to_string(path)?
                } else {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf)?;
                    buf
                };

                let manager = WikiEngine::build(&config_path)?;
                let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let result = ops::content_write(&engine, &uri, cli.wiki.as_deref(), &content)?;
                println!(
                    "Wrote {} bytes to {}",
                    result.bytes_written,
                    result.path.display()
                );
            }
            ContentAction::New {
                uri,
                section,
                bundle,
                name,
                r#type,
                dry_run,
            } => {
                if dry_run {
                    let manager = WikiEngine::build(&config_path)?;
                    let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    let global = &engine.config;
                    let (entry, slug) =
                        llm_wiki::slug::WikiUri::resolve(&uri, cli.wiki.as_deref(), global)?;
                    let kind = if section {
                        "section"
                    } else if bundle {
                        "bundle"
                    } else {
                        "flat"
                    };
                    println!("Would create {kind} at wiki://{}/{slug}", entry.name);
                } else {
                    let manager = WikiEngine::build(&config_path)?;
                    let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    let result_uri = ops::content_new(
                        &engine,
                        &uri,
                        cli.wiki.as_deref(),
                        section,
                        bundle,
                        name.as_deref(),
                        r#type.as_deref(),
                    )?;
                    println!("Created: {result_uri}");
                }
            }
            ContentAction::Commit {
                slugs,
                all,
                message,
            } => {
                let manager = WikiEngine::build(&config_path)?;
                let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref()).to_string();

                let hash =
                    ops::content_commit(&engine, &wiki_name, &slugs, all, message.as_deref())?;

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
            cross_wiki,
            format,
        } => {
            let manager = WikiEngine::build(&config_path)?;
            let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());

            let results = ops::search(
                &engine,
                wiki_name,
                &ops::SearchParams {
                    query: &query,
                    type_filter: r#type.as_deref(),
                    no_excerpt,
                    top_k,
                    include_sections,
                    cross_wiki,
                },
            )?;

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
            let manager = WikiEngine::build(&config_path)?;
            let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());

            let result = ops::list(
                &engine,
                wiki_name,
                r#type.as_deref(),
                status.as_deref(),
                page,
                page_size,
            )?;

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
            let manager = WikiEngine::build(&config_path)?;
            let report = {
                let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref()).to_string();
                ops::ingest(&engine, &manager, &path, dry_run, &wiki_name)?
            };

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

        // ── Graph ─────────────────────────────────────────────────────
        Commands::Graph {
            format,
            root,
            depth,
            r#type,
            relation,
            output,
        } => {
            let manager = WikiEngine::build(&config_path)?;
            let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());

            let result = ops::graph_build(
                &engine,
                wiki_name,
                &ops::GraphParams {
                    format: format.as_deref(),
                    root,
                    depth,
                    type_filter: r#type.as_deref(),
                    relation,
                    output: output.as_deref(),
                },
            )?;

            // If no output file, print rendered graph
            if output.is_none() {
                print!("{}", result.rendered);
            } else {
                println!("Wrote graph to {}", result.report.output);
            }
        }

        // ── Index ─────────────────────────────────────────────────────
        Commands::Index { action } => match action {
            IndexAction::Rebuild { dry_run, format } => {
                let manager = WikiEngine::build(&config_path)?;
                let wiki_name = {
                    let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    engine.resolve_wiki_name(cli.wiki.as_deref()).to_string()
                };

                if dry_run {
                    let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                    let space = engine.space(&wiki_name)?;
                    let count = walkdir::WalkDir::new(&space.wiki_root)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().is_file()
                                && e.path().extension().and_then(|x| x.to_str()) == Some("md")
                        })
                        .count();
                    println!(
                        "Would index {count} pages from {}",
                        space.wiki_root.display()
                    );
                } else {
                    let report = ops::index_rebuild(&manager, &wiki_name)?;
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
                let manager = WikiEngine::build(&config_path)?;
                let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
                let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref());

                let status = ops::index_status(&engine, wiki_name)?;

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
        Commands::Schema { action } => {
            let manager = WikiEngine::build(&config_path)?;
            let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
            let wiki_name = engine.resolve_wiki_name(cli.wiki.as_deref()).to_string();

            match action {
                SchemaAction::List { format } => {
                    let entries = ops::schema_list(&engine, &wiki_name)?;
                    if is_json(&format) {
                        println!("{}", serde_json::to_string_pretty(&entries)?);
                    } else {
                        for e in &entries {
                            println!("{:<16}{}", e.name, e.description);
                        }
                    }
                }
                SchemaAction::Show {
                    name,
                    template,
                    format: _,
                } => {
                    if template {
                        let tmpl = ops::schema_show_template(&engine, &wiki_name, &name)?;
                        println!("{tmpl}");
                    } else {
                        let content = ops::schema_show(&engine, &wiki_name, &name)?;
                        println!("{content}");
                    }
                }
                SchemaAction::Add { name, schema_path } => {
                    let msg = ops::schema_add(&engine, &wiki_name, &name, Path::new(&schema_path))?;
                    println!("{msg}");
                }
                SchemaAction::Remove {
                    name,
                    delete,
                    delete_pages,
                    dry_run,
                } => {
                    drop(engine);
                    let report = ops::schema_remove(
                        &manager,
                        &wiki_name,
                        &name,
                        delete,
                        delete_pages,
                        dry_run,
                    )?;
                    if is_json(&None::<String>) {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else {
                        if report.dry_run {
                            println!("DRY RUN:");
                        }
                        println!("pages removed from index: {}", report.pages_removed);
                        println!(
                            "page files deleted from disk: {}",
                            report.pages_deleted_from_disk
                        );
                        println!("wiki.toml updated: {}", report.wiki_toml_updated);
                        println!("schema file deleted: {}", report.schema_file_deleted);
                    }
                }
                SchemaAction::Validate { name } => {
                    let issues = ops::schema_validate(&engine, &wiki_name, name.as_deref())?;
                    if issues.is_empty() {
                        println!("ok");
                    } else {
                        for issue in &issues {
                            println!("{issue}");
                        }
                        std::process::exit(1);
                    }
                }
            }
        }

        Commands::Serve { http, acp, dry_run } => {
            if dry_run {
                let mut transports = vec!["stdio".to_string()];
                if http.is_some() {
                    transports.push("http".to_string());
                }
                if acp {
                    transports.push("acp".to_string());
                }
                println!("Would start: [{}]", transports.join("] ["));
                return Ok(());
            }

            let http_port = http
                .and_then(|opt| opt.and_then(|s| s.trim_start_matches(':').parse::<u16>().ok()));

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(llm_wiki::server::serve(&config_path, http_port, acp))?;
        }

        Commands::Logs { action } => match action {
            LogsAction::Tail { lines } => {
                let output = ops::logs_tail(&config_path, lines)?;
                println!("{output}");
            }
            LogsAction::List => {
                let files = ops::logs_list(&config_path)?;
                if files.is_empty() {
                    println!("no log files");
                } else {
                    for f in &files {
                        println!("{f}");
                    }
                }
            }
            LogsAction::Clear => {
                let removed = ops::logs_clear(&config_path)?;
                println!("removed {removed} log file(s)");
            }
        },
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
