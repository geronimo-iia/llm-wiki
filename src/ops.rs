use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

use crate::config::{self, GlobalConfig, WikiConfig};
use crate::engine::{Engine, EngineManager};
use crate::git;
use crate::graph;
use crate::ingest;
use crate::markdown;
use crate::indexing;
use crate::search;
use crate::slug::{resolve_read_target, ReadTarget, Slug, WikiUri};
use crate::spaces;

// ── Spaces ────────────────────────────────────────────────────────────────────

pub fn spaces_create(
    path: &Path,
    name: &str,
    description: Option<&str>,
    force: bool,
    set_default: bool,
    config_path: &Path,
) -> Result<spaces::CreateReport> {
    spaces::create(path, name, description, force, set_default, config_path)
}

pub fn spaces_list(config: &GlobalConfig) -> Vec<config::WikiEntry> {
    spaces::load_all(config)
}

pub fn spaces_remove(name: &str, delete: bool, config_path: &Path) -> Result<()> {
    spaces::remove(name, delete, config_path)
}

pub fn spaces_set_default(name: &str, config_path: &Path) -> Result<()> {
    spaces::set_default_wiki(name, config_path)
}

// ── Config ────────────────────────────────────────────────────────────────────

pub fn config_get(config_path: &Path, key: &str) -> Result<String> {
    let g = config::load_global(config_path)?;
    let resolved = config::resolve(&g, &WikiConfig::default());
    Ok(config::get_config_value(&resolved, &g, key))
}

pub fn config_set(
    config_path: &Path,
    key: &str,
    value: &str,
    global: bool,
    wiki_name: Option<&str>,
) -> Result<String> {
    if global {
        let mut g = config::load_global(config_path)?;
        config::set_global_config_value(&mut g, key, value)?;
        config::save_global(&g, config_path)?;
        Ok(format!("Set {key} = {value} (global)"))
    } else {
        let g = config::load_global(config_path)?;
        let name = wiki_name.unwrap_or(&g.global.default_wiki);
        let entry = spaces::resolve_name(name, &g)?;
        let entry_path = PathBuf::from(&entry.path);
        let mut wiki_cfg = config::load_wiki(&entry_path)?;
        config::set_wiki_config_value(&mut wiki_cfg, key, value)?;
        config::save_wiki(&wiki_cfg, &entry_path)?;
        Ok(format!("Set {key} = {value} (wiki: {name})"))
    }
}

pub fn config_list_global(config_path: &Path) -> Result<String> {
    let g = config::load_global(config_path)?;
    Ok(toml::to_string_pretty(&g)?)
}

pub fn config_list_resolved(config_path: &Path) -> Result<config::ResolvedConfig> {
    let g = config::load_global(config_path)?;
    Ok(config::resolve(&g, &WikiConfig::default()))
}

// ── Content ───────────────────────────────────────────────────────────────────

pub enum ContentReadResult {
    Page(String),
    Assets(Vec<String>),
    Binary,
}

pub fn content_read(
    engine: &Engine,
    uri: &str,
    wiki_flag: Option<&str>,
    no_frontmatter: bool,
    list_assets: bool,
) -> Result<ContentReadResult> {
    let (entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");

    if list_assets {
        let assets = markdown::list_assets(&slug, &wiki_root)?;
        return Ok(ContentReadResult::Assets(assets));
    }

    match resolve_read_target(slug.as_str(), &wiki_root)? {
        ReadTarget::Page(_) => {
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default();
            let resolved = config::resolve(&engine.config, &wiki_cfg);
            let strip = no_frontmatter || resolved.read.no_frontmatter;
            let content = markdown::read_page(&slug, &wiki_root, strip)?;
            Ok(ContentReadResult::Page(content))
        }
        ReadTarget::Asset(parent_slug, filename) => {
            let parent = Slug::try_from(parent_slug.as_str())?;
            let bytes = markdown::read_asset(&parent, &filename, &wiki_root)?;
            match String::from_utf8(bytes) {
                Ok(text) => Ok(ContentReadResult::Page(text)),
                Err(_) => Ok(ContentReadResult::Binary),
            }
        }
    }
}

pub struct WriteResult {
    pub bytes_written: usize,
    pub path: PathBuf,
}

pub fn content_write(
    engine: &Engine,
    uri: &str,
    wiki_flag: Option<&str>,
    content: &str,
) -> Result<WriteResult> {
    let (_entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&_entry.path).join("wiki");
    let path = markdown::write_page(slug.as_str(), content, &wiki_root)?;
    Ok(WriteResult {
        bytes_written: content.len(),
        path,
    })
}

pub fn content_new(
    engine: &Engine,
    uri: &str,
    wiki_flag: Option<&str>,
    section: bool,
    bundle: bool,
    name: Option<&str>,
    type_: Option<&str>,
) -> Result<String> {
    let (entry, slug) = WikiUri::resolve(uri, wiki_flag, &engine.config)?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");

    if section {
        markdown::create_section(&slug, &wiki_root)?;
    } else {
        markdown::create_page(&slug, bundle, &wiki_root, name, type_)?;
    }
    Ok(format!("wiki://{}/{slug}", entry.name))
}

pub fn content_commit(
    engine: &Engine,
    wiki_name: &str,
    slugs: &[String],
    all: bool,
    message: Option<&str>,
) -> Result<String> {
    let space = engine.space(wiki_name)?;

    if slugs.is_empty() && !all {
        bail!("specify slugs or --all");
    }

    if all {
        let msg = message.unwrap_or("commit: all");
        return git::commit(&space.repo_root, msg);
    }

    let mut paths = Vec::new();
    for s in slugs {
        let slug = Slug::try_from(s.as_str())?;
        let resolved = slug.resolve(&space.wiki_root)?;
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
    let default_msg = format!("commit: {}", slugs.join(", "));
    let msg = message.unwrap_or(&default_msg);
    git::commit_paths(&space.repo_root, &path_refs, msg)
}

// ── Search ────────────────────────────────────────────────────────────────────

pub struct SearchParams<'a> {
    pub query: &'a str,
    pub type_filter: Option<&'a str>,
    pub no_excerpt: bool,
    pub top_k: Option<usize>,
    pub include_sections: bool,
    pub all: bool,
}

pub fn search(
    engine: &Engine,
    wiki_name: &str,
    params: &SearchParams<'_>,
) -> Result<Vec<search::PageRef>> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = search::SearchOptions {
        no_excerpt: params.no_excerpt,
        include_sections: params.include_sections,
        top_k: params
            .top_k
            .unwrap_or(resolved.defaults.search_top_k as usize),
        r#type: params.type_filter.map(|s| s.to_string()),
    };

    if params.all {
        let wikis: Vec<(String, PathBuf)> = engine
            .spaces
            .values()
            .map(|s| (s.name.clone(), s.index_path.clone()))
            .collect();
        return search::search_all(params.query, &opts, &wikis, &space.schema);
    }

    let recovery = if engine.config.index.auto_recovery {
        Some((space.wiki_root.clone(), space.repo_root.clone()))
    } else {
        None
    };
    let recovery_ctx = recovery.as_ref().map(|(wr, rr)| indexing::RecoveryContext {
        wiki_root: wr,
        repo_root: rr,
    });
    search::search(
        params.query,
        &opts,
        &space.index_path,
        wiki_name,
        &space.schema,
        recovery_ctx.as_ref(),
    )
}

// ── List ──────────────────────────────────────────────────────────────────────

pub fn list(
    engine: &Engine,
    wiki_name: &str,
    type_filter: Option<&str>,
    status: Option<&str>,
    page: usize,
    page_size: Option<usize>,
) -> Result<search::PageList> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = search::ListOptions {
        r#type: type_filter.map(|s| s.to_string()),
        status: status.map(|s| s.to_string()),
        page,
        page_size: page_size.unwrap_or(resolved.defaults.list_page_size as usize),
    };
    let recovery = if engine.config.index.auto_recovery {
        Some((space.wiki_root.clone(), space.repo_root.clone()))
    } else {
        None
    };
    let recovery_ctx = recovery.as_ref().map(|(wr, rr)| indexing::RecoveryContext {
        wiki_root: wr,
        repo_root: rr,
    });
    search::list(
        &opts,
        &space.index_path,
        wiki_name,
        &space.schema,
        recovery_ctx.as_ref(),
    )
}

// ── Ingest ────────────────────────────────────────────────────────────────────

pub fn ingest(
    engine: &Engine,
    manager: &EngineManager,
    path: &str,
    dry_run: bool,
    wiki_name: &str,
) -> Result<ingest::IngestReport> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let opts = ingest::IngestOptions {
        dry_run,
        auto_commit: resolved.ingest.auto_commit,
    };
    let report = ingest::ingest(
        Path::new(path),
        &opts,
        &space.wiki_root,
        &engine.type_registry,
        &resolved.validation,
    )?;

    if !dry_run {
        if let Err(e) = manager.on_ingest(wiki_name) {
            tracing::warn!(error = %e, "incremental index update failed after ingest");
        }
    }

    Ok(report)
}

// ── Index ─────────────────────────────────────────────────────────────────────

pub fn index_rebuild(manager: &EngineManager, wiki_name: &str) -> Result<indexing::IndexReport> {
    manager.rebuild_index(wiki_name)
}

pub fn index_status(engine: &Engine, wiki_name: &str) -> Result<indexing::IndexStatus> {
    let space = engine.space(wiki_name)?;
    indexing::index_status(wiki_name, &space.index_path, &space.repo_root)
}

// ── Graph ─────────────────────────────────────────────────────────────────────

pub struct GraphResult {
    pub rendered: String,
    pub report: graph::GraphReport,
}

pub struct GraphParams<'a> {
    pub format: Option<&'a str>,
    pub root: Option<String>,
    pub depth: Option<usize>,
    pub type_filter: Option<&'a str>,
    pub relation: Option<String>,
    pub output: Option<&'a str>,
}

pub fn graph_build(
    engine: &Engine,
    wiki_name: &str,
    params: &GraphParams<'_>,
) -> Result<GraphResult> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let fmt = params.format.unwrap_or(&resolved.graph.format);
    let types: Vec<String> = params
        .type_filter
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let filter = graph::GraphFilter {
        root: params.root.clone(),
        depth: params.depth.or(Some(resolved.graph.depth as usize)),
        types,
        relation: params.relation.clone(),
    };
    let g = graph::build_graph(&space.index_path, &space.schema, &filter)?;

    let rendered = match fmt {
        "dot" => graph::render_dot(&g),
        _ => graph::render_mermaid(&g),
    };

    let out = if let Some(out_path) = params.output {
        let content = if out_path.ends_with(".md") {
            graph::wrap_graph_md(&rendered, fmt, &filter)
        } else {
            rendered.clone()
        };
        std::fs::write(out_path, &content)?;
        out_path.to_string()
    } else {
        "stdout".to_string()
    };

    Ok(GraphResult {
        rendered,
        report: graph::GraphReport {
            nodes: g.node_count(),
            edges: g.edge_count(),
            output: out,
        },
    })
}
