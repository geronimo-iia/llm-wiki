use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::{Content, Tool};
use serde_json::{json, Map, Value};

use crate::config::{self, WikiEntry};
use crate::git;
use crate::graph;
use crate::ingest;
use crate::init;
use crate::lint;
use crate::markdown;
use crate::search;
use crate::server::WikiServer;
use crate::spaces;

// ── Static tool definitions ───────────────────────────────────────────────────

fn schema(props: Value, required: &[&str]) -> Arc<Map<String, Value>> {
    let req: Vec<Value> = required
        .iter()
        .map(|s| Value::String(s.to_string()))
        .collect();
    let obj = json!({
        "type": "object",
        "properties": props,
        "required": req,
    });
    Arc::new(obj.as_object().unwrap().clone())
}

fn str_prop(desc: &str) -> Value {
    json!({"type": "string", "description": desc})
}

fn opt_str(desc: &str) -> Value {
    json!({"type": "string", "description": desc})
}

fn opt_bool(desc: &str) -> Value {
    json!({"type": "boolean", "description": desc})
}

fn opt_int(desc: &str) -> Value {
    json!({"type": "integer", "description": desc})
}

pub fn tool_list() -> Vec<Tool> {
    vec![
        Tool::new(
            "wiki_init",
            "Initialize a new wiki repository",
            schema(
                json!({
                    "path": str_prop("Path to create the wiki at"),
                    "name": str_prop("Wiki name — used in wiki:// URIs"),
                    "description": opt_str("Optional one-line description"),
                    "force": opt_bool("Update space entry if name already exists"),
                    "set_default": opt_bool("Set as default wiki"),
                }),
                &["path", "name"],
            ),
        ),
        Tool::new(
            "wiki_config",
            "Get or set configuration values",
            schema(
                json!({
                    "action": str_prop("Action: get, set, or list"),
                    "key": opt_str("Config key (for get/set)"),
                    "value": opt_str("Config value (for set)"),
                    "global": opt_bool("Write to global config"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["action"],
            ),
        ),
        Tool::new(
            "wiki_spaces_list",
            "List all registered wiki spaces",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "wiki_spaces_remove",
            "Remove a wiki space",
            schema(
                json!({
                    "name": str_prop("Wiki name to remove"),
                    "delete": opt_bool("Also delete the wiki directory from disk"),
                }),
                &["name"],
            ),
        ),
        Tool::new(
            "wiki_spaces_set_default",
            "Set the default wiki space",
            schema(
                json!({
                    "name": str_prop("Wiki name to set as default"),
                }),
                &["name"],
            ),
        ),
        Tool::new(
            "wiki_write",
            "Write a file into the wiki tree",
            schema(
                json!({
                    "path": str_prop("File path relative to wiki root"),
                    "content": str_prop("File content"),
                    "wiki": opt_str("Target wiki name (default: default_wiki)"),
                }),
                &["path", "content"],
            ),
        ),
        Tool::new(
            "wiki_ingest",
            "Validate, commit, and index files in the wiki tree",
            schema(
                json!({
                    "path": str_prop("File or folder path, relative to wiki root"),
                    "dry_run": opt_bool("Show what would be committed without committing"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["path"],
            ),
        ),
        Tool::new(
            "wiki_new_page",
            "Create a new page with scaffolded frontmatter",
            schema(
                json!({
                    "uri": str_prop("wiki:// URI for the new page"),
                    "bundle": opt_bool("Create as bundle (folder + index.md)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_new_section",
            "Create a new section with index.md",
            schema(
                json!({
                    "uri": str_prop("wiki:// URI for the new section"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_search",
            "Full-text BM25 search, returns ranked results",
            schema(
                json!({
                    "query": str_prop("Search query"),
                    "no_excerpt": opt_bool("Omit excerpts — refs only"),
                    "include_sections": opt_bool("Include section index pages"),
                    "top_k": opt_int("Max results"),
                    "wiki": opt_str("Target wiki name"),
                    "all": opt_bool("Search across all wikis"),
                }),
                &["query"],
            ),
        ),
        Tool::new(
            "wiki_read",
            "Read full content of a page by slug or URI",
            schema(
                json!({
                    "uri": str_prop("Slug or wiki:// URI"),
                    "no_frontmatter": opt_bool("Strip frontmatter from output"),
                    "list_assets": opt_bool("List co-located assets instead of content"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_list",
            "Paginated page listing with filters",
            schema(
                json!({
                    "type": opt_str("Filter by frontmatter type"),
                    "status": opt_str("Filter by frontmatter status"),
                    "page": opt_int("Page number, 1-based"),
                    "page_size": opt_int("Results per page"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_index_rebuild",
            "Rebuild the tantivy search index",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_index_status",
            "Inspect index health",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_lint",
            "Structural audit, returns LintReport",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                    "dry_run": opt_bool("Show what would be written, no commit"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_graph",
            "Generate concept graph, returns GraphReport",
            schema(
                json!({
                    "format": opt_str("Output format: mermaid | dot"),
                    "root": opt_str("Subgraph from this node (slug)"),
                    "depth": opt_int("Hop limit from root"),
                    "type": opt_str("Comma-separated page types to include"),
                    "output": opt_str("File path for output (default: stdout/return)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
    ]
}

// ── Argument helpers ──────────────────────────────────────────────────────────

fn arg_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn arg_str_req(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    arg_str(args, key).ok_or_else(|| format!("missing required parameter: {key}"))
}

fn arg_bool(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn arg_usize(args: &Map<String, Value>, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

fn resolve_wiki(
    server: &WikiServer,
    args: &Map<String, Value>,
) -> Result<(WikiEntry, config::GlobalConfig), String> {
    let global = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let name = arg_str(args, "wiki");
    let wiki_name = name.as_deref().unwrap_or(&global.global.default_wiki);
    let entry = spaces::resolve_name(wiki_name, &global).map_err(|e| format!("{e}"))?;
    Ok((entry, global))
}

type ToolHandlerResult = Result<(Vec<Content>, Vec<String>), String>;

fn ok_text(text: String) -> ToolHandlerResult {
    Ok((vec![Content::text(text)], vec![]))
}

fn err_text(msg: String) -> Vec<Content> {
    vec![Content::text(format!("error: {msg}"))]
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

/// Result of a tool call: content, is_error, and resource URIs to notify.
pub struct ToolResult {
    pub content: Vec<Content>,
    pub is_error: bool,
    pub notify_uris: Vec<String>,
}

pub fn call(server: &WikiServer, name: &str, args: &Map<String, Value>) -> ToolResult {
    let _span = tracing::info_span!("tool_call", tool = name).entered();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match name {
            "wiki_init" => handle_init(server, args),
            "wiki_config" => handle_config(server, args),
            "wiki_spaces_list" => handle_spaces_list(server),
            "wiki_spaces_remove" => handle_spaces_remove(server, args),
            "wiki_spaces_set_default" => handle_spaces_set_default(server, args),
            "wiki_write" => handle_write(server, args),
            "wiki_ingest" => handle_ingest(server, args),
            "wiki_new_page" => handle_new_page(server, args),
            "wiki_new_section" => handle_new_section(server, args),
            "wiki_search" => handle_search(server, args),
            "wiki_read" => handle_read(server, args),
            "wiki_list" => handle_list(server, args),
            "wiki_index_rebuild" => handle_index_rebuild(server, args),
            "wiki_index_status" => handle_index_status(server, args),
            "wiki_lint" => handle_lint(server, args),
            "wiki_graph" => handle_graph(server, args),
            _ => Err(format!("unknown tool: {name}")),
        }
    }));
    match result {
        Ok(Ok((content, notify_uris))) => {
            tracing::debug!(tool = name, "tool call ok");
            ToolResult {
                content,
                is_error: false,
                notify_uris,
            }
        }
        Ok(Err(msg)) => {
            tracing::warn!(tool = name, error = %msg, "tool call failed");
            ToolResult {
                content: err_text(msg),
                is_error: true,
                notify_uris: vec![],
            }
        }
        Err(_) => {
            tracing::error!(tool = name, "tool handler panicked");
            ToolResult {
                content: err_text("internal error: tool panicked".into()),
                is_error: true,
                notify_uris: vec![],
            }
        }
    }
}

fn handle_init(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let name = arg_str_req(args, "name")?;
    let description = arg_str(args, "description");
    let force = arg_bool(args, "force");
    let set_default = arg_bool(args, "set_default");

    let report = init::init(
        &PathBuf::from(&path),
        &name,
        description.as_deref(),
        force,
        set_default,
        server.config_path(),
    )
    .map_err(|e| format!("{e}"))?;

    let json = serde_json::to_string_pretty(&serde_json::json!({
        "path": report.path,
        "name": report.name,
        "created": report.created,
        "registered": report.registered,
        "committed": report.committed,
    }))
    .map_err(|e| format!("{e}"))?;
    ok_text(json)
}

fn handle_config(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let action = arg_str_req(args, "action")?;
    let config_path = server.config_path();
    match action.as_str() {
        "list" => {
            let g = config::load_global(config_path).map_err(|e| format!("{e}"))?;
            let s = toml::to_string_pretty(&g).map_err(|e| format!("{e}"))?;
            ok_text(s)
        }
        "get" => {
            let key = arg_str_req(args, "key")?;
            let g = config::load_global(config_path).map_err(|e| format!("{e}"))?;
            let resolved = config::resolve(&g, &config::WikiConfig::default());
            ok_text(format!("{}: {}", key, get_value(&resolved, &g, &key)))
        }
        "set" => {
            let key = arg_str_req(args, "key")?;
            let value = arg_str_req(args, "value")?;
            let is_global = arg_bool(args, "global");
            if is_global {
                let mut g = config::load_global(config_path).map_err(|e| format!("{e}"))?;
                config::set_global_config_value(&mut g, &key, &value).map_err(|e| format!("{e}"))?;
                config::save_global(&g, config_path).map_err(|e| format!("{e}"))?;
                ok_text(format!("Set {key} = {value} (global)"))
            } else {
                let (entry, _global) = resolve_wiki(server, args)?;
                let entry_path = PathBuf::from(&entry.path);
                let mut wiki_cfg = config::load_wiki(&entry_path).map_err(|e| format!("{e}"))?;
                config::set_wiki_config_value(&mut wiki_cfg, &key, &value).map_err(|e| format!("{e}"))?;
                config::save_wiki(&wiki_cfg, &entry_path).map_err(|e| format!("{e}"))?;
                ok_text(format!("Set {key} = {value} (wiki: {})", entry.name))
            }
        }
        _ => Err(format!("unknown config action: {action}")),
    }
}

fn handle_spaces_list(server: &WikiServer) -> ToolHandlerResult {
    let g = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let entries = spaces::load_all(&g);
    let s = serde_json::to_string_pretty(&entries).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_spaces_remove(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    let delete = arg_bool(args, "delete");
    spaces::remove(&name, delete, server.config_path()).map_err(|e| format!("{e}"))?;
    ok_text(format!("Removed wiki \"{name}\""))
}

fn handle_spaces_set_default(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    spaces::set_default(&name, server.config_path()).map_err(|e| format!("{e}"))?;
    ok_text(format!("Default wiki set to \"{name}\""))
}

fn handle_write(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, _) = resolve_wiki(server, args)?;
    let path = arg_str_req(args, "path")?;
    let content = arg_str_req(args, "content")?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");
    let full_path = wiki_root.join(&path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{e}"))?;
    }
    std::fs::write(&full_path, &content).map_err(|e| format!("{e}"))?;
    ok_text(format!("Wrote {} bytes to {}", content.len(), path))
}

fn handle_ingest(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, global) = resolve_wiki(server, args)?;
    let path = arg_str_req(args, "path")?;
    let dry_run = arg_bool(args, "dry_run");
    let entry_path = PathBuf::from(&entry.path);
    let wiki_root = entry_path.join("wiki");
    let wiki_cfg = config::load_wiki(&entry_path).unwrap_or_default();
    let resolved = config::resolve(&global, &wiki_cfg);
    let schema_cfg = config::load_schema(&entry_path).unwrap_or_default();
    let opts = ingest::IngestOptions { dry_run };
    let mut report = ingest::ingest(
        std::path::Path::new(&path),
        &opts,
        &wiki_root,
        &schema_cfg,
        &resolved.validation,
    )
    .map_err(|e| format!("{e}"))?;

    // Index update after ingest
    if !dry_run {
        if resolved.index.auto_rebuild {
            let index_path = WikiServer::index_path_for(&entry.name);
            let repo_root = PathBuf::from(&entry.path);
            if let Err(e) = search::rebuild_index(&wiki_root, &index_path, &entry.name, &repo_root) {
                report.warnings.push(format!("index rebuild failed: {e}"));
            }
        } else {
            report.warnings.push("search index is stale \u{2014} run wiki_index_rebuild".into());
        }
    }

    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;

    // Collect URIs of ingested .md files for resource update notifications
    let notify_uris = if !dry_run {
        let ingest_path = wiki_root.join(&path);
        collect_page_uris(&ingest_path, &wiki_root, &entry.name)
    } else {
        vec![]
    };

    Ok((vec![Content::text(s)], notify_uris))
}

fn handle_new_page(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let bundle = arg_bool(args, "bundle");
    let global = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let (entry, slug) = spaces::resolve_uri(&uri, &global).map_err(|e| format!("{e}"))?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");
    let repo_root = PathBuf::from(&entry.path);
    markdown::create_page(&slug, bundle, &wiki_root).map_err(|e| format!("{e}"))?;
    if let Err(e) = git::commit(&repo_root, &format!("new: {uri}")) {
        tracing::warn!(error = %e, uri = %uri, "git commit failed for new page");
    }
    ok_text(format!("wiki://{}/{slug}", entry.name))
}

fn handle_new_section(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let global = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let (entry, slug) = spaces::resolve_uri(&uri, &global).map_err(|e| format!("{e}"))?;
    let wiki_root = PathBuf::from(&entry.path).join("wiki");
    let repo_root = PathBuf::from(&entry.path);
    markdown::create_section(&slug, &wiki_root).map_err(|e| format!("{e}"))?;
    if let Err(e) = git::commit(&repo_root, &format!("new: {uri}")) {
        tracing::warn!(error = %e, uri = %uri, "git commit failed for new section");
    }
    ok_text(format!("wiki://{}/{slug}", entry.name))
}

fn handle_search(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let query = arg_str_req(args, "query")?;
    let global = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let all = arg_bool(args, "all");

    let name = arg_str(args, "wiki");
    let wiki_name = name.as_deref().unwrap_or(&global.global.default_wiki);
    let entry = spaces::resolve_name(wiki_name, &global).map_err(|e| format!("{e}"))?;
    let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default();
    let resolved = config::resolve(&global, &wiki_cfg);

    let opts = search::SearchOptions {
        no_excerpt: arg_bool(args, "no_excerpt"),
        include_sections: arg_bool(args, "include_sections"),
        top_k: arg_usize(args, "top_k").unwrap_or(resolved.defaults.search_top_k as usize),
        ..Default::default()
    };

    let results = if all {
        let wikis: Vec<(String, std::path::PathBuf)> = global
            .wikis
            .iter()
            .map(|w| (w.name.clone(), WikiServer::index_path_for(&w.name)))
            .collect();
        search::search_all(&query, &opts, &wikis).map_err(|e| format!("{e}"))?
    } else {
        let repo_root = PathBuf::from(&entry.path);
        let index_path = WikiServer::index_path_for(&entry.name);

        if let Ok(status) = search::index_status(&entry.name, &index_path, &repo_root) {
            if status.stale && resolved.index.auto_rebuild {
                let wiki_root = repo_root.join("wiki");
                if let Err(e) = search::rebuild_index(&wiki_root, &index_path, &entry.name, &repo_root) {
                    tracing::warn!(wiki = %entry.name, error = %e, "search index rebuild failed");
                }
            }
        }

        let recovery = if resolved.index.auto_recovery {
            let wiki_root_buf = repo_root.join("wiki");
            Some((wiki_root_buf, repo_root.clone()))
        } else {
            None
        };
        let recovery_ctx = recovery.as_ref().map(|(wr, rr)| search::RecoveryContext {
            wiki_root: wr,
            repo_root: rr,
        });

        search::search(&query, &opts, &index_path, &entry.name, recovery_ctx.as_ref()).map_err(|e| format!("{e}"))?
    };

    let s = serde_json::to_string_pretty(&results).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_read(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let global = config::load_global(server.config_path()).map_err(|e| format!("{e}"))?;
    let (entry, slug) = if uri.starts_with("wiki://") {
        spaces::resolve_uri(&uri, &global).map_err(|e| format!("{e}"))?
    } else {
        let name = arg_str(args, "wiki");
        let wiki_name = name.as_deref().unwrap_or(&global.global.default_wiki);
        let e = spaces::resolve_name(wiki_name, &global).map_err(|e| format!("{e}"))?;
        (e, uri.clone())
    };
    let wiki_root = PathBuf::from(&entry.path).join("wiki");

    if arg_bool(args, "list_assets") {
        let assets = markdown::list_assets(&slug, &wiki_root).map_err(|e| format!("{e}"))?;
        return ok_text(assets.join("\n"));
    }

    match markdown::resolve_read_target(&slug, &wiki_root).map_err(|e| format!("{e}"))? {
        markdown::ReadTarget::Page(_) => {
            let wiki_cfg = config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default();
            let resolved = config::resolve(&global, &wiki_cfg);
            let strip = arg_bool(args, "no_frontmatter") || resolved.read.no_frontmatter;
            let content = markdown::read_page(&slug, &wiki_root, strip).map_err(|e| format!("{e}"))?;
            ok_text(content)
        }
        markdown::ReadTarget::Asset(parent_slug, filename) => {
            let bytes = markdown::read_asset(&parent_slug, &filename, &wiki_root).map_err(|e| format!("{e}"))?;
            match String::from_utf8(bytes) {
                Ok(text) => ok_text(text),
                Err(_) => Err(format!("asset {slug} is binary — access it directly from the filesystem")),
            }
        }
    }
}

fn handle_list(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, global) = resolve_wiki(server, args)?;
    let repo_root = PathBuf::from(&entry.path);
    let index_path = WikiServer::index_path_for(&entry.name);
    let wiki_cfg = config::load_wiki(&repo_root).unwrap_or_default();
    let resolved = config::resolve(&global, &wiki_cfg);

    if let Ok(st) = search::index_status(&entry.name, &index_path, &repo_root) {
        if st.stale && resolved.index.auto_rebuild {
            let wiki_root = repo_root.join("wiki");
            if let Err(e) = search::rebuild_index(&wiki_root, &index_path, &entry.name, &repo_root) {
                tracing::warn!(wiki = %entry.name, error = %e, "search index rebuild failed");
            }
        }
    }

    let opts = search::ListOptions {
        r#type: arg_str(args, "type"),
        status: arg_str(args, "status"),
        page: arg_usize(args, "page").unwrap_or(1),
        page_size: arg_usize(args, "page_size")
            .unwrap_or(resolved.defaults.list_page_size as usize),
    };
    let recovery = if resolved.index.auto_recovery {
        let wiki_root_buf = repo_root.join("wiki");
        Some((wiki_root_buf, repo_root.clone()))
    } else {
        None
    };
    let recovery_ctx = recovery.as_ref().map(|(wr, rr)| search::RecoveryContext {
        wiki_root: wr,
        repo_root: rr,
    });
    let result = search::list(&opts, &index_path, &entry.name, recovery_ctx.as_ref()).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&result).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_index_rebuild(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, _) = resolve_wiki(server, args)?;
    let repo_root = PathBuf::from(&entry.path);
    let wiki_root = repo_root.join("wiki");
    let index_path = WikiServer::index_path_for(&entry.name);
    let report = search::rebuild_index(&wiki_root, &index_path, &entry.name, &repo_root)
        .map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_index_status(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, _) = resolve_wiki(server, args)?;
    let repo_root = PathBuf::from(&entry.path);
    let index_path = WikiServer::index_path_for(&entry.name);
    let status =
        search::index_status(&entry.name, &index_path, &repo_root).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&status).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_lint(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, global) = resolve_wiki(server, args)?;
    let dry_run = arg_bool(args, "dry_run");
    let entry_path = PathBuf::from(&entry.path);
    let wiki_root = entry_path.join("wiki");
    let wiki_cfg = config::load_wiki(&entry_path).unwrap_or_default();
    let resolved = config::resolve(&global, &wiki_cfg);

    let report = lint::lint(&wiki_root, &resolved, &entry.name).map_err(|e| format!("{e}"))?;

    if !dry_run {
        if let Err(e) = lint::write_lint_md(&report, &entry_path) {
            tracing::warn!(error = %e, "failed to write LINT.md");
        }
        let date = &report.date;
        let msg = format!(
            "lint: {date} \u{2014} {} orphans, {} stubs, {} empty sections",
            report.orphans.len(),
            report.missing_stubs.len(),
            report.empty_sections.len()
        );
        if let Err(e) = git::commit(&entry_path, &msg) {
            tracing::warn!(error = %e, "lint git commit failed");
        }
    }

    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

fn handle_graph(server: &WikiServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let (entry, global) = resolve_wiki(server, args)?;
    let entry_path = PathBuf::from(&entry.path);
    let wiki_root = entry_path.join("wiki");
    let wiki_cfg = config::load_wiki(&entry_path).unwrap_or_default();
    let resolved = config::resolve(&global, &wiki_cfg);

    let fmt = arg_str(args, "format").unwrap_or_else(|| resolved.graph.format.clone());
    let types: Vec<String> = arg_str(args, "type")
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let filter = graph::GraphFilter {
        root: arg_str(args, "root"),
        depth: arg_usize(args, "depth").or(Some(resolved.graph.depth as usize)),
        types,
    };
    let g = graph::build_graph(&wiki_root, &filter);

    let rendered = match fmt.as_str() {
        "dot" => graph::render_dot(&g),
        _ => graph::render_mermaid(&g),
    };

    let (out, committed) = if let Some(out_path) = arg_str(args, "output") {
        let content = if out_path.ends_with(".md") {
            graph::wrap_graph_md(&rendered, &fmt, &filter)
        } else {
            rendered
        };
        if let Err(e) = std::fs::write(&out_path, &content) {
            tracing::warn!(path = %out_path, error = %e, "graph output write failed");
        }

        // Auto-commit if inside repo root
        let out_canonical = std::fs::canonicalize(&out_path).ok();
        let repo_canonical = entry_path.canonicalize().ok();
        let did_commit = if let (Some(out_c), Some(repo_c)) = (out_canonical, repo_canonical) {
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

        (out_path, did_commit)
    } else {
        ("stdout".to_string(), false)
    };

    let report = graph::GraphReport {
        nodes: g.node_count(),
        edges: g.edge_count(),
        output: out,
        committed,
    };
    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

// ── Resource notification helper ──────────────────────────────────────────────

fn collect_page_uris(
    path: &std::path::Path,
    wiki_root: &std::path::Path,
    wiki_name: &str,
) -> Vec<String> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let slug = markdown::slug_for(path, wiki_root);
            return vec![format!("wiki://{wiki_name}/{slug}")];
        }
        return vec![];
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("md")
        })
        .map(|e| {
            let slug = markdown::slug_for(e.path(), wiki_root);
            format!("wiki://{wiki_name}/{slug}")
        })
        .collect()
}

// ── Config value helper ───────────────────────────────────────────────────────

fn get_value(
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
        "index.auto_recovery" => global.index.auto_recovery.to_string(),
        "graph.format" => resolved.graph.format.clone(),
        "graph.depth" => resolved.graph.depth.to_string(),
        "graph.output" => resolved.graph.output.clone(),
        "serve.sse" => resolved.serve.sse.to_string(),
        "serve.sse_port" => resolved.serve.sse_port.to_string(),
        "serve.acp" => resolved.serve.acp.to_string(),
        "serve.max_restarts" => global.serve.max_restarts.to_string(),
        "serve.restart_backoff" => global.serve.restart_backoff.to_string(),
        "serve.heartbeat_secs" => global.serve.heartbeat_secs.to_string(),
        "validation.type_strictness" => resolved.validation.type_strictness.clone(),
        "lint.fix_missing_stubs" => resolved.lint.fix_missing_stubs.to_string(),
        "lint.fix_empty_sections" => resolved.lint.fix_empty_sections.to_string(),
        "logging.log_path" => global.logging.log_path.clone(),
        "logging.log_rotation" => global.logging.log_rotation.clone(),
        "logging.log_max_files" => global.logging.log_max_files.to_string(),
        "logging.log_format" => global.logging.log_format.clone(),
        _ => format!("unknown key: {key}"),
    }
}
