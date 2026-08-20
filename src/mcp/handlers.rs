use std::sync::LazyLock;

use rmcp::model::ContentBlock as Content;
use serde_json::{Map, Value};

use crate::ops;
use crate::slug::{ReadTarget, WikiUri, resolve_read_target};

use super::McpServer;
use super::helpers::*;

static PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // Two alternatives:
    //   /[a-zA-Z0-9_./-]{3,}  — absolute Unix paths starting with /
    //   ~[a-zA-Z0-9_./~-]{2,} — tilde-prefixed paths (~/ or ~user/)
    // Known limitation: paths containing spaces are not fully redacted.
    // Expanding the character class to include space greedily absorbs adjacent
    // English words — a more robust fix requires a parser, not a regex.
    // Primary protection: all handler call sites already pass errors through this function.
    regex::Regex::new(r"(?:/[a-zA-Z0-9_./-]{3,}|~[a-zA-Z0-9_./~-]{2,})").unwrap()
});

/// Redact filesystem paths from an error message before sending to LLM clients.
///
/// Strips absolute and tilde-prefixed Unix paths so that
/// `failed to open /home/user/wikis/my-wiki/search-index/state.toml: No such file`
/// becomes `failed to open <path>: No such file`, and
/// `config at ~/wikis/foo` becomes `config at <path>`.
fn redact_error(e: impl std::fmt::Display) -> String {
    PATH_RE.replace_all(&format!("{e}"), "<path>").into_owned()
}

// ── Spaces ────────────────────────────────────────────────────────────────────

/// Handle `wiki_spaces_create` — create a new wiki repository and register it.
pub fn handle_spaces_create(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let name = arg_str_req(args, "name")?;
    let description = arg_str(args, "description");
    let force = arg_bool(args, "force");
    let set_default = arg_bool(args, "set_default");
    let wiki_root = arg_str(args, "wiki_root");

    let config_path = {
        let engine = server.engine()?;
        engine.config_path.clone()
    };
    let report = ops::spaces_create(
        &std::path::PathBuf::from(&path),
        &name,
        description.as_deref(),
        force,
        set_default,
        &config_path,
        Some(&server.manager),
        wiki_root.as_deref(),
    )
    .map_err(redact_error)?;

    let json = serde_json::to_string_pretty(&serde_json::json!({
        "path": report.path,
        "name": report.name,
        "created": report.created,
        "registered": report.registered,
        "committed": report.committed,
    }))
    .map_err(redact_error)?;
    ok_text(json)
}

/// Handle `wiki_spaces_register` — register an existing wiki repository without creating files.
pub fn handle_spaces_register(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let name = arg_str_req(args, "name")?;
    let description = arg_str(args, "description");
    let wiki_root = arg_str(args, "wiki_root");

    let config_path = {
        let engine = server.engine()?;
        engine.config_path.clone()
    };
    let report = ops::spaces_register(
        &std::path::PathBuf::from(&path),
        &name,
        description.as_deref(),
        wiki_root.as_deref(),
        &config_path,
        Some(&server.manager),
    )
    .map_err(redact_error)?;

    let json = serde_json::to_string_pretty(&serde_json::json!({
        "path": report.path,
        "name": report.name,
        "registered": report.registered,
    }))
    .map_err(redact_error)?;
    ok_text(json)
}

/// Handle `wiki_spaces_list` — list registered wiki spaces.
pub fn handle_spaces_list(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let name = arg_str(args, "name");
    let entries = ops::spaces_list(&engine.config, name.as_deref());
    let s = serde_json::to_string_pretty(&entries).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_spaces_remove` — unregister (and optionally delete) a wiki space.
pub fn handle_spaces_remove(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    let delete = arg_bool(args, "delete");
    let config_path = {
        let engine = server.engine()?;
        engine.config_path.clone()
    };
    ops::spaces_remove(&name, delete, &config_path, Some(&server.manager)).map_err(redact_error)?;
    ok_text(format!("Removed wiki \"{name}\""))
}

/// Handle `wiki_spaces_set_default` — set the default wiki space.
pub fn handle_spaces_set_default(
    server: &McpServer,
    args: &Map<String, Value>,
) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    let config_path = {
        let engine = server.engine()?;
        engine.config_path.clone()
    };
    ops::spaces_set_default(&name, &config_path, Some(&server.manager)).map_err(redact_error)?;
    ok_text(format!("Default wiki set to \"{name}\""))
}

// ── Config ────────────────────────────────────────────────────────────────────

/// Handle `wiki_config` — get, set, or list configuration values.
pub fn handle_config(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let action = arg_str_req(args, "action")?;
    let engine = server.engine()?;
    let config_path = &engine.config_path;

    match action.as_str() {
        "list" => {
            let s = ops::config_list_global(config_path).map_err(redact_error)?;
            ok_text(s)
        }
        "get" => {
            let key = arg_str_req(args, "key")?;
            let val = ops::config_get(config_path, &key).map_err(redact_error)?;
            ok_text(format!("{key}: {val}"))
        }
        "set" => {
            let key = arg_str_req(args, "key")?;
            let value = arg_str_req(args, "value")?;
            let is_global = arg_bool(args, "global");
            let wiki_name = resolve_wiki_name(&engine, args)?;
            let msg = ops::config_set(config_path, &key, &value, is_global, Some(&wiki_name))
                .map_err(redact_error)?;
            ok_text(msg)
        }
        _ => Err(format!("unknown config action: {action}")),
    }
}

// ── Content ───────────────────────────────────────────────────────────────────

/// Handle `wiki_content_read` — read a page or list its co-located assets.
pub fn handle_content_read(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let engine = server.engine()?;
    let wiki_flag = arg_str(args, "wiki");
    let no_frontmatter = arg_bool(args, "no_frontmatter");
    let list_assets = arg_bool(args, "list_assets");
    let include_backlinks = arg_bool(args, "backlinks");

    match ops::content_read(
        &engine,
        &uri,
        wiki_flag.as_deref(),
        no_frontmatter,
        list_assets,
    )
    .map_err(redact_error)?
    {
        ops::ContentReadResult::Page(content) => {
            if include_backlinks {
                let wiki_name = engine
                    .resolve_wiki_name(wiki_flag.as_deref())
                    .map_err(redact_error)?
                    .to_string();
                let (_entry, slug) = WikiUri::resolve(&uri, wiki_flag.as_deref(), &engine.config)
                    .map_err(redact_error)?;
                let backlinks =
                    ops::backlinks_for(&engine, &wiki_name, slug.as_str()).map_err(redact_error)?;
                let response = serde_json::json!({
                    "content": content,
                    "backlinks": backlinks,
                });
                let s = serde_json::to_string_pretty(&response).map_err(redact_error)?;
                ok_text(s)
            } else {
                ok_text(content)
            }
        }
        ops::ContentReadResult::Assets(assets) => ok_text(assets.join("\n")),
        ops::ContentReadResult::Binary => {
            Err("asset is binary — access it directly from the filesystem".into())
        }
    }
}

/// Handle `wiki_content_write` — write content to a wiki page by slug or URI.
pub fn handle_content_write(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let content = arg_str_req(args, "content")?;
    const MAX_CONTENT_BYTES: usize = 10 * 1024 * 1024;
    if content.len() > MAX_CONTENT_BYTES {
        return Err(format!(
            "content exceeds maximum allowed size of {} bytes (got {})",
            MAX_CONTENT_BYTES,
            content.len()
        ));
    }
    let engine = server.engine()?;
    let wiki_flag = arg_str(args, "wiki");

    let result =
        ops::content_write(&engine, &uri, wiki_flag.as_deref(), &content).map_err(redact_error)?;
    ok_text(format!("Wrote {} bytes to {}", result.bytes_written, uri))
}

/// Handle `wiki_content_new` — create a new page or section with scaffolded frontmatter.
pub fn handle_content_new(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let section = arg_bool(args, "section");
    let bundle = arg_bool(args, "bundle");
    let name = arg_str(args, "name");
    let type_ = arg_str(args, "type");

    let engine = server.engine()?;
    let wiki_flag = arg_str(args, "wiki");

    let result = ops::content_new(
        &engine,
        &uri,
        wiki_flag.as_deref(),
        section,
        bundle,
        name.as_deref(),
        type_.as_deref(),
    )
    .map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&serde_json::json!({
        "uri":    result.uri,
        "slug":   result.slug,
        "bundle": result.bundle,
    }))
    .map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_resolve` — resolve a slug or URI to its filesystem path.
pub fn handle_resolve(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let engine = server.engine()?;
    let wiki_flag = arg_str(args, "wiki");

    let (entry, slug) =
        WikiUri::resolve(&uri, wiki_flag.as_deref(), &engine.config).map_err(redact_error)?;
    let wiki_root = engine
        .space(&entry.name)
        .map(|s| s.wiki_root.clone())
        .unwrap_or_else(|_| entry.path.clone().join("wiki"));

    let (path, exists, bundle) = match resolve_read_target(slug.as_str(), &wiki_root) {
        Ok(ReadTarget::Page(p)) => {
            let bundle = p.ends_with("index.md");
            (p, true, bundle)
        }
        _ => {
            let p = wiki_root.join(format!("{}.md", slug.as_str()));
            (p, false, false)
        }
    };

    let s = serde_json::to_string_pretty(&serde_json::json!({
        "slug":      slug.as_str(),
        "wiki":      entry.name,
        "wiki_root": wiki_root,
        "path":      path,
        "exists":    exists,
        "bundle":    bundle,
    }))
    .map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_content_commit` — commit pending changes to git.
pub fn handle_content_commit(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let message = arg_str(args, "message");

    let slugs: Vec<String> = arg_str(args, "slugs")
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let all = slugs.is_empty();

    let hash = ops::content_commit(&engine, &wiki_name, &slugs, all, message.as_deref())
        .map_err(redact_error)?;
    if hash.is_empty() {
        return ok_text(
            "nothing to commit; run wiki_ingest first if you have unsaved changes".to_string(),
        );
    }
    ok_text(hash)
}

// ── Search ────────────────────────────────────────────────────────────────────

/// Handle `wiki_search` — BM25 full-text search across a wiki.
pub fn handle_search(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let query = arg_str_req(args, "query")?;
    let cross_wiki = arg_bool(args, "cross_wiki");
    let format = arg_str(args, "format");
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let results = ops::search(
        &engine,
        &wiki_name,
        &ops::SearchParams {
            query: &query,
            type_filter: arg_str(args, "type").as_deref(),
            no_excerpt: format.as_deref() == Some("llms") || arg_bool(args, "no_excerpt"),
            top_k: arg_usize(args, "top_k"),
            include_sections: arg_bool(args, "include_sections"),
            cross_wiki,
        },
    )
    .map_err(|e| {
        let msg = redact_error(e);
        if msg.contains("index not open") {
            format!("{msg}; call wiki_index_rebuild to rebuild or wiki_index_status for details")
        } else {
            msg
        }
    })?;

    if format.as_deref() == Some("llms") {
        ok_text(crate::search::render_search_llms(&results))
    } else {
        let s = serde_json::to_string_pretty(&results).map_err(redact_error)?;
        ok_text(s)
    }
}

// ── List ──────────────────────────────────────────────────────────────────────

/// Handle `wiki_list` — paginated page listing with optional type/status filters.
pub fn handle_list(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let format = arg_str(args, "format");

    let result = ops::list(
        &engine,
        &wiki_name,
        arg_str(args, "type").as_deref(),
        arg_str(args, "status").as_deref(),
        arg_usize(args, "page").unwrap_or(1),
        arg_usize(args, "page_size"),
    )
    .map_err(redact_error)?;

    if format.as_deref() == Some("llms") {
        ok_text(crate::search::render_list_llms(&result))
    } else {
        let s = serde_json::to_string_pretty(&result).map_err(redact_error)?;
        ok_text(s)
    }
}

// ── Ingest ────────────────────────────────────────────────────────────────────

/// Handle `wiki_ingest` — validate, redact, commit, and index files in the wiki tree.
pub fn handle_ingest(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let dry_run = arg_bool(args, "dry_run");
    let redact = arg_bool(args, "redact");

    // Read path: ingest (ops handles WikiEngine mutation internally)
    let (report, wiki_name, notify_uris) = {
        let engine = server.engine()?;
        let wiki_name = resolve_wiki_name(&engine, args)?;

        let report =
            ops::ingest_with_redact(&engine, &server.manager, &path, dry_run, redact, &wiki_name)
                .map_err(redact_error)?;

        let notify_uris = if !dry_run {
            let space = engine.space(&wiki_name).map_err(redact_error)?;
            let ingest_path = space.wiki_root.join(&path);
            collect_page_uris(&ingest_path, &space.wiki_root, &wiki_name)
        } else {
            vec![]
        };

        (report, wiki_name, notify_uris)
    };

    let _ = wiki_name; // used above for notify_uris
    let s = serde_json::to_string_pretty(&report).map_err(redact_error)?;
    Ok((vec![Content::text(s)], notify_uris))
}

// ── Index ─────────────────────────────────────────────────────────────────────

/// Handle `wiki_index_rebuild` — rebuild the tantivy search index from scratch.
pub fn handle_index_rebuild(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let wiki_name = {
        let engine = server.engine()?;
        resolve_wiki_name(&engine, args)?
    };

    let report = ops::index_rebuild(&server.manager, &wiki_name).map_err(redact_error)?;
    tracing::info!(
        wiki = %wiki_name,
        pages = report.pages_indexed,
        duration_ms = report.duration_ms,
        "index rebuild completed"
    );
    let s = serde_json::to_string_pretty(&report).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_index_status` — report health and staleness of the search index.
pub fn handle_index_status(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let status = ops::index_status(&engine, &wiki_name).map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&status).map_err(redact_error)?;
    ok_text(s)
}

// ── Graph ─────────────────────────────────────────────────────────────────────

/// Handle `wiki_graph` — build and render the concept graph.
pub fn handle_graph(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let result = ops::graph_build(
        &engine,
        &wiki_name,
        &ops::GraphParams {
            format: arg_str(args, "format").as_deref(),
            root: arg_str(args, "root"),
            depth: arg_usize(args, "depth"),
            type_filter: arg_str(args, "type").as_deref(),
            relation: arg_str(args, "relation"),
            output: arg_str(args, "output").as_deref(),
            cross_wiki: arg_bool(args, "cross_wiki"),
            limit: arg_usize(args, "limit"),
        },
    )
    .map_err(redact_error)?;

    ok_text(result.rendered)
}

// ── History ───────────────────────────────────────────────────────────────────

/// Handle `wiki_history` — return git commit history for a page slug.
pub fn handle_history(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let raw_slug = arg_str_req(args, "slug")?;
    crate::slug::Slug::try_from(raw_slug.as_str()).map_err(|e| format!("invalid slug: {e}"))?;
    let slug = raw_slug;
    let limit = arg_usize(args, "limit");
    let follow = args.get("follow").and_then(|v| v.as_bool());
    let wiki_flag = arg_str(args, "wiki");

    let engine = server.engine()?;
    let result =
        ops::history(&engine, &slug, wiki_flag.as_deref(), limit, follow).map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&result).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_stats` — return aggregate health and coverage stats for a wiki.
pub fn handle_stats(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let detail_str = arg_str(args, "detail");
    let detail = match detail_str.as_deref() {
        Some("full") => ops::StatsDetail::Full,
        _ => ops::StatsDetail::Summary,
    };
    let result =
        ops::stats(&engine, &wiki_name, &ops::StatsOptions { detail }).map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&result).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_lint` — run deterministic lint rules and return findings.
pub fn handle_lint(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let rules = arg_str(args, "rules");
    let severity = arg_str(args, "severity");
    let path_prefix = arg_str(args, "path_prefix");
    let result = ops::run_lint(
        &engine,
        &wiki_name,
        &ops::LintOptions {
            rules: rules.as_deref(),
            severity: severity.as_deref(),
            summary: arg_bool(args, "summary"),
            path_prefix: path_prefix.as_deref(),
            page_size: arg_usize(args, "page_size"),
            cursor: arg_usize(args, "cursor"),
        },
    )
    .map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&result).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_suggest` — suggest related pages to link from a given slug.
pub fn handle_suggest(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let raw_slug = arg_str_req(args, "slug")?;
    crate::slug::Slug::try_from(raw_slug.as_str()).map_err(|e| format!("invalid slug: {e}"))?;
    let slug = raw_slug;
    let limit = arg_usize(args, "limit");
    let wiki_flag = arg_str(args, "wiki");
    let engine = server.engine()?;
    let result = ops::suggest(&engine, &slug, wiki_flag.as_deref(), limit).map_err(redact_error)?;
    let s = serde_json::to_string_pretty(&result).map_err(redact_error)?;
    ok_text(s)
}

/// Handle `wiki_schema` — list, show, add, remove, or validate type schemas.
pub fn handle_schema(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let action = arg_str(args, "action").ok_or("action is required")?;
    let engine = server.engine()?;
    let wiki_name = resolve_wiki_name(&engine, args)?;

    match action.as_str() {
        "list" => {
            let entries = ops::schema_list(&engine, &wiki_name).map_err(redact_error)?;
            let s = serde_json::to_string_pretty(&entries).map_err(redact_error)?;
            ok_text(s)
        }
        "show" => {
            let type_name = arg_str(args, "type").ok_or("type is required for show")?;
            let template = args
                .get("template")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if template {
                let tmpl = ops::schema_show_template(&engine, &wiki_name, &type_name)
                    .map_err(redact_error)?;
                ok_text(tmpl)
            } else {
                let content =
                    ops::schema_show(&engine, &wiki_name, &type_name).map_err(redact_error)?;
                ok_text(content)
            }
        }
        "add" => {
            let type_name = arg_str(args, "type").ok_or("type is required for add")?;
            let schema_path =
                arg_str(args, "schema_path").ok_or("schema_path is required for add")?;
            let msg = ops::schema_add(
                &engine,
                &wiki_name,
                &type_name,
                std::path::Path::new(&schema_path),
            )
            .map_err(redact_error)?;
            ok_text(msg)
        }
        "remove" => {
            let type_name = arg_str(args, "type").ok_or("type is required for remove")?;
            let delete = args
                .get("delete")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let delete_pages = args
                .get("delete_pages")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let dry_run = args
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            drop(engine);
            let report = ops::schema_remove(
                &server.manager,
                &wiki_name,
                &type_name,
                delete,
                delete_pages,
                dry_run,
            )
            .map_err(redact_error)?;
            let s = serde_json::to_string_pretty(&report).map_err(redact_error)?;
            ok_text(s)
        }
        "validate" => {
            let type_name = arg_str(args, "type");
            let issues = ops::schema_validate(&engine, &wiki_name, type_name.as_deref())
                .map_err(redact_error)?;
            if issues.is_empty() {
                ok_text("ok".to_string())
            } else {
                ok_text(issues.join("\n"))
            }
        }
        _ => Err(format!("unknown action: {action}")),
    }
}

// ── Export ────────────────────────────────────────────────────────────────────

/// Handle `wiki_export` — export the full wiki to llms.txt, llms-full, or JSON.
pub fn handle_export(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let wiki = arg_str_req(args, "wiki")?;
    let engine = server.engine()?;

    let format = ops::ExportFormat::parse(arg_str(args, "format").as_deref().unwrap_or("llms-txt"));
    let include_archived = arg_str(args, "status").as_deref() == Some("all");

    let report = ops::export(
        &engine,
        &ops::ExportOptions {
            wiki: wiki.clone(),
            path: arg_str(args, "path"),
            format,
            include_archived,
        },
    )
    .map_err(redact_error)?;

    let s = serde_json::to_string_pretty(&report).map_err(redact_error)?;
    ok_text(s)
}

// ── Info ──────────────────────────────────────────────────────────────────────

/// Handle `wiki_info` — return server version, config path, registered spaces, and index health.
pub fn handle_info(server: &McpServer, _args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine()?;
    let version = env!("CARGO_PKG_VERSION");
    let spaces: Vec<String> = engine.config.wikis.iter().map(|w| w.name.clone()).collect();
    let default_wiki = engine.config.global.default_wiki.clone();
    let mut all_ok = true;
    let index_status: serde_json::Map<String, Value> = engine
        .spaces
        .keys()
        .map(|wiki_name| {
            let entry = match ops::index_status(&engine, wiki_name) {
                Ok(s) if s.openable && s.queryable && !s.stale => {
                    serde_json::json!({"status": "ok"})
                }
                Ok(s) => {
                    all_ok = false;
                    let reason = s.degraded_reason.unwrap_or_else(|| "unknown".to_string());
                    serde_json::json!({"status": "degraded", "reason": reason})
                }
                Err(e) => {
                    all_ok = false;
                    serde_json::json!({"status": "degraded", "reason": redact_error(e)})
                }
            };
            (wiki_name.clone(), entry)
        })
        .collect();
    let info = serde_json::json!({
        "version": version,
        "spaces": spaces,
        "default_wiki": default_wiki,
        "index_status": if all_ok { Value::String("ok".into()) } else { Value::Object(index_status) },
    });
    ok_text(serde_json::to_string_pretty(&info).map_err(redact_error)?)
}

#[cfg(test)]
mod tests {
    use super::redact_error;

    #[test]
    fn redact_absolute_path_in_error() {
        let msg = "failed to open /home/user/wikis/my-wiki/state.toml: No such file";
        assert_eq!(redact_error(msg), "failed to open <path>: No such file");
    }

    #[test]
    fn redact_multiple_paths() {
        let msg = "copy /tmp/build/a.idx to /var/data/b.idx failed";
        assert_eq!(redact_error(msg), "copy <path> to <path> failed");
    }

    #[test]
    fn path_free_message_unchanged() {
        let msg = "permission denied";
        assert_eq!(redact_error(msg), "permission denied");
    }

    #[test]
    fn very_short_path_not_redacted() {
        // regex requires 3+ chars after the leading slash
        let msg = "error at /ab";
        assert_eq!(redact_error(msg), "error at /ab");
    }

    #[test]
    fn tilde_path_fully_redacted() {
        // ~/... paths are matched by the ~[...]{2,} alternative — full redaction.
        let msg = "config not found at ~/.config/llm-wiki/config.toml";
        assert_eq!(redact_error(msg), "config not found at <path>");
    }

    #[test]
    fn tilde_user_path_redacted() {
        let msg = "open ~user/wikis/foo failed";
        assert_eq!(redact_error(msg), "open <path> failed");
    }
}
