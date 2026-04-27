use rmcp::model::Content;
use serde_json::{Map, Value};

use crate::ops;

use super::McpServer;
use super::helpers::*;

// ── Spaces ────────────────────────────────────────────────────────────────────

pub fn handle_spaces_create(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let name = arg_str_req(args, "name")?;
    let description = arg_str(args, "description");
    let force = arg_bool(args, "force");
    let set_default = arg_bool(args, "set_default");

    let config_path = {
        let engine = server.engine();
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

pub fn handle_spaces_list(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
    let name = arg_str(args, "name");
    let entries = ops::spaces_list(&engine.config, name.as_deref());
    let s = serde_json::to_string_pretty(&entries).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

pub fn handle_spaces_remove(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    let delete = arg_bool(args, "delete");
    let config_path = {
        let engine = server.engine();
        engine.config_path.clone()
    };
    ops::spaces_remove(&name, delete, &config_path, Some(&server.manager))
        .map_err(|e| format!("{e}"))?;
    ok_text(format!("Removed wiki \"{name}\""))
}

pub fn handle_spaces_set_default(
    server: &McpServer,
    args: &Map<String, Value>,
) -> ToolHandlerResult {
    let name = arg_str_req(args, "name")?;
    let config_path = {
        let engine = server.engine();
        engine.config_path.clone()
    };
    ops::spaces_set_default(&name, &config_path, Some(&server.manager))
        .map_err(|e| format!("{e}"))?;
    ok_text(format!("Default wiki set to \"{name}\""))
}

// ── Config ────────────────────────────────────────────────────────────────────

pub fn handle_config(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let action = arg_str_req(args, "action")?;
    let engine = server.engine();
    let config_path = &engine.config_path;

    match action.as_str() {
        "list" => {
            let s = ops::config_list_global(config_path).map_err(|e| format!("{e}"))?;
            ok_text(s)
        }
        "get" => {
            let key = arg_str_req(args, "key")?;
            let val = ops::config_get(config_path, &key).map_err(|e| format!("{e}"))?;
            ok_text(format!("{key}: {val}"))
        }
        "set" => {
            let key = arg_str_req(args, "key")?;
            let value = arg_str_req(args, "value")?;
            let is_global = arg_bool(args, "global");
            let wiki_name = resolve_wiki_name(&engine, args)?;
            let msg = ops::config_set(config_path, &key, &value, is_global, Some(&wiki_name))
                .map_err(|e| format!("{e}"))?;
            ok_text(msg)
        }
        _ => Err(format!("unknown config action: {action}")),
    }
}

// ── Content ───────────────────────────────────────────────────────────────────

pub fn handle_content_read(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let engine = server.engine();
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
    .map_err(|e| format!("{e}"))?
    {
        ops::ContentReadResult::Page(content) => {
            if include_backlinks {
                use crate::slug::WikiUri;
                let wiki_name = engine.resolve_wiki_name(wiki_flag.as_deref()).to_string();
                let (_entry, slug) = WikiUri::resolve(&uri, wiki_flag.as_deref(), &engine.config)
                    .map_err(|e| format!("{e}"))?;
                let backlinks = ops::backlinks_for(&engine, &wiki_name, slug.as_str())
                    .map_err(|e| format!("{e}"))?;
                let response = serde_json::json!({
                    "content": content,
                    "backlinks": backlinks,
                });
                let s = serde_json::to_string_pretty(&response).map_err(|e| format!("{e}"))?;
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

pub fn handle_content_write(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let content = arg_str_req(args, "content")?;
    let engine = server.engine();
    let wiki_flag = arg_str(args, "wiki");

    let result = ops::content_write(&engine, &uri, wiki_flag.as_deref(), &content)
        .map_err(|e| format!("{e}"))?;
    ok_text(format!(
        "Wrote {} bytes to {}",
        result.bytes_written,
        result.path.display()
    ))
}

pub fn handle_content_new(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let uri = arg_str_req(args, "uri")?;
    let section = arg_bool(args, "section");
    let bundle = arg_bool(args, "bundle");
    let name = arg_str(args, "name");
    let type_ = arg_str(args, "type");

    let engine = server.engine();
    let wiki_flag = arg_str(args, "wiki");

    let result_uri = ops::content_new(
        &engine,
        &uri,
        wiki_flag.as_deref(),
        section,
        bundle,
        name.as_deref(),
        type_.as_deref(),
    )
    .map_err(|e| format!("{e}"))?;
    ok_text(result_uri)
}

pub fn handle_content_commit(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let message = arg_str(args, "message");

    let slugs: Vec<String> = arg_str(args, "slugs")
        .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let all = slugs.is_empty();

    let hash = ops::content_commit(&engine, &wiki_name, &slugs, all, message.as_deref())
        .map_err(|e| format!("{e}"))?;
    ok_text(hash)
}

// ── Search ────────────────────────────────────────────────────────────────────

pub fn handle_search(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let query = arg_str_req(args, "query")?;
    let cross_wiki = arg_bool(args, "cross_wiki");
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let results = ops::search(
        &engine,
        &wiki_name,
        &ops::SearchParams {
            query: &query,
            type_filter: arg_str(args, "type").as_deref(),
            no_excerpt: arg_bool(args, "no_excerpt"),
            top_k: arg_usize(args, "top_k"),
            include_sections: arg_bool(args, "include_sections"),
            cross_wiki,
        },
    )
    .map_err(|e| format!("{e}"))?;

    let s = serde_json::to_string_pretty(&results).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

// ── List ──────────────────────────────────────────────────────────────────────

pub fn handle_list(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let result = ops::list(
        &engine,
        &wiki_name,
        arg_str(args, "type").as_deref(),
        arg_str(args, "status").as_deref(),
        arg_usize(args, "page").unwrap_or(1),
        arg_usize(args, "page_size"),
    )
    .map_err(|e| format!("{e}"))?;

    let s = serde_json::to_string_pretty(&result).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

// ── Ingest ────────────────────────────────────────────────────────────────────

pub fn handle_ingest(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let path = arg_str_req(args, "path")?;
    let dry_run = arg_bool(args, "dry_run");

    // Read path: ingest (ops handles WikiEngine mutation internally)
    let (report, wiki_name, notify_uris) = {
        let engine = server.engine();
        let wiki_name = resolve_wiki_name(&engine, args)?;

        let report = ops::ingest(&engine, &server.manager, &path, dry_run, &wiki_name)
            .map_err(|e| format!("{e}"))?;

        let notify_uris = if !dry_run {
            let space = engine.space(&wiki_name).map_err(|e| format!("{e}"))?;
            let ingest_path = space.wiki_root.join(&path);
            collect_page_uris(&ingest_path, &space.wiki_root, &wiki_name)
        } else {
            vec![]
        };

        (report, wiki_name, notify_uris)
    };

    let _ = wiki_name; // used above for notify_uris
    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
    Ok((vec![Content::text(s)], notify_uris))
}

// ── Index ─────────────────────────────────────────────────────────────────────

pub fn handle_index_rebuild(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let wiki_name = {
        let engine = server.engine();
        resolve_wiki_name(&engine, args)?
    };

    let report = ops::index_rebuild(&server.manager, &wiki_name).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

pub fn handle_index_status(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;

    let status = ops::index_status(&engine, &wiki_name).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&status).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

// ── Graph ─────────────────────────────────────────────────────────────────────

pub fn handle_graph(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
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
        },
    )
    .map_err(|e| format!("{e}"))?;

    let s = serde_json::to_string_pretty(&result.report).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

// ── History ───────────────────────────────────────────────────────────────────

pub fn handle_history(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let slug = arg_str_req(args, "slug")?;
    let limit = arg_usize(args, "limit");
    let follow = args.get("follow").and_then(|v| v.as_bool());
    let wiki_flag = arg_str(args, "wiki");

    let engine = server.engine();
    let result = ops::history(&engine, &slug, wiki_flag.as_deref(), limit, follow)
        .map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&result).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

pub fn handle_stats(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;
    let result = ops::stats(&engine, &wiki_name).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&result).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

pub fn handle_suggest(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let slug = arg_str_req(args, "slug")?;
    let limit = arg_usize(args, "limit");
    let wiki_flag = arg_str(args, "wiki");
    let engine = server.engine();
    let result =
        ops::suggest(&engine, &slug, wiki_flag.as_deref(), limit).map_err(|e| format!("{e}"))?;
    let s = serde_json::to_string_pretty(&result).map_err(|e| format!("{e}"))?;
    ok_text(s)
}

pub fn handle_schema(server: &McpServer, args: &Map<String, Value>) -> ToolHandlerResult {
    let action = arg_str(args, "action").ok_or("action is required")?;
    let engine = server.engine();
    let wiki_name = resolve_wiki_name(&engine, args)?;

    match action.as_str() {
        "list" => {
            let entries = ops::schema_list(&engine, &wiki_name).map_err(|e| format!("{e}"))?;
            let s = serde_json::to_string_pretty(&entries).map_err(|e| format!("{e}"))?;
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
                    .map_err(|e| format!("{e}"))?;
                ok_text(tmpl)
            } else {
                let content = ops::schema_show(&engine, &wiki_name, &type_name)
                    .map_err(|e| format!("{e}"))?;
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
            .map_err(|e| format!("{e}"))?;
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
            .map_err(|e| format!("{e}"))?;
            let s = serde_json::to_string_pretty(&report).map_err(|e| format!("{e}"))?;
            ok_text(s)
        }
        "validate" => {
            let type_name = arg_str(args, "type");
            let issues = ops::schema_validate(&engine, &wiki_name, type_name.as_deref())
                .map_err(|e| format!("{e}"))?;
            if issues.is_empty() {
                ok_text("ok".to_string())
            } else {
                ok_text(issues.join("\n"))
            }
        }
        _ => Err(format!("unknown action: {action}")),
    }
}
