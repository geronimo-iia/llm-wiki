use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

use super::McpServer;
use super::handlers;
use super::helpers::{ToolResult, check_param_lengths, err_text};

// ── Schema helpers ────────────────────────────────────────────────────────────

fn schema(props: Value, required: &[&str]) -> Arc<Map<String, Value>> {
    let mut map = Map::new();
    map.insert("type".to_string(), Value::String("object".to_string()));
    map.insert("properties".to_string(), props);
    map.insert(
        "required".to_string(),
        Value::Array(
            required
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
    );
    Arc::new(map)
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

// ── Tool definitions ──────────────────────────────────────────────────────────

/// Return the complete list of MCP tool definitions for registration.
pub fn tool_list() -> Vec<Tool> {
    vec![
        Tool::new(
            "wiki_spaces_create",
            "Initialize a new wiki repository.",
            schema(
                json!({
                    "path": str_prop("Path to create the wiki at"),
                    "name": str_prop("Wiki name — used in wiki:// URIs"),
                    "description": opt_str("Optional one-line description"),
                    "force": opt_bool("Update space entry if name already exists"),
                    "set_default": opt_bool("Set as default wiki"),
                    "wiki_root": opt_str("Content directory relative to repo root (default: \"wiki\")"),
                }),
                &["path", "name"],
            ),
        ),
        Tool::new(
            "wiki_spaces_register",
            "Register an existing wiki repository without creating files.",
            schema(
                json!({
                    "path": str_prop("Absolute path to the existing wiki repository"),
                    "name": str_prop("Wiki name — used in wiki:// URIs"),
                    "description": opt_str("Optional one-line description"),
                    "wiki_root": opt_str("Content directory (overrides wiki.toml; must already exist)"),
                }),
                &["path", "name"],
            ),
        ),
        Tool::new(
            "wiki_spaces_list",
            "List all registered wiki spaces.",
            schema(
                json!({
                    "name": opt_str("Wiki name (omit for all)"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_spaces_remove",
            "Unregister a wiki space from the config. Pass delete=true to also delete the wiki directory from disk (irreversible).",
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
            "Set the default wiki space.",
            schema(
                json!({
                    "name": str_prop("Wiki name to set as default"),
                }),
                &["name"],
            ),
        ),
        Tool::new(
            "wiki_config",
            concat!(
                "Get, set, or list configuration values. ",
                "action=\"get\" returns the current value of a key; ",
                "action=\"set\" writes a new value; ",
                "action=\"list\" shows all key/value pairs. ",
                "Example keys: \"global.default_wiki\", \"defaults.search_top_k\", ",
                "\"index.auto_rebuild\", \"graph.format\". ",
                "Omit --wiki to target the global config; pass --wiki <name> for per-wiki overrides."
            ),
            schema(
                json!({
                    "action": str_prop("Action: \"get\" (read a key), \"set\" (write a key), or \"list\" (show all keys)"),
                    "key": opt_str("Dot-notation config key, e.g. \"global.default_wiki\", \"defaults.search_top_k\", \"index.auto_rebuild\""),
                    "value": opt_str("New value for the key (required for action=\"set\")"),
                    "global": opt_bool("Write to global config instead of per-wiki wiki.toml"),
                    "wiki": opt_str("Target wiki name (omit to use default wiki)"),
                }),
                &["action"],
            ),
        ),
        Tool::new(
            "wiki_content_read",
            "Read full content of a page by slug or URI.",
            schema(
                json!({
                    "uri": str_prop("Slug (e.g. \"concepts/attention\") or wiki:// URI (e.g. \"wiki://my-wiki/concepts/attention\")"),
                    "no_frontmatter": opt_bool("Strip frontmatter from output"),
                    "list_assets": opt_bool("List co-located assets instead of content"),
                    "backlinks": opt_bool("Include incoming links — pages that link to this page"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_content_write",
            "Write content to a page in the wiki tree.",
            schema(
                json!({
                    "uri": str_prop("Slug (e.g. \"concepts/attention\") or wiki:// URI (e.g. \"wiki://my-wiki/concepts/attention\")"),
                    "content": str_prop("Full file content including required YAML frontmatter (--- title: ... type: ... ---)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri", "content"],
            ),
        ),
        Tool::new(
            "wiki_content_new",
            "Create a page or section with scaffolded frontmatter.",
            schema(
                json!({
                    "uri": str_prop("Slug (e.g. \"concepts/attention\") or wiki:// URI (e.g. \"wiki://my-wiki/concepts/attention\")"),
                    "section": opt_bool("Create a section instead of a page"),
                    "bundle": opt_bool("Create as bundle (folder + index.md)"),
                    "name": opt_str("Page title (default: derived from slug)"),
                    "type": opt_str("Page type (default: page)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_content_commit",
            "Commit pending changes to git.",
            schema(
                json!({
                    "slugs": opt_str("Comma-separated page slugs to commit (omit for all)"),
                    "message": opt_str("Commit message"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_search",
            "Full-text BM25 search, returns ranked results.",
            schema(
                json!({
                    "query": str_prop("Search query. Supports BM25 full-text, phrase quoting (\"exact phrase\"), field filters (title:\"foo\"), and type shorthand (type:concept)."),
                    "type": opt_str("Filter by frontmatter type"),
                    "no_excerpt": opt_bool("Omit excerpts — refs only"),
                    "include_sections": opt_bool("Include section index pages"),
                    "top_k": opt_int("Max results to return (default: 10, max: 100)"),
                    "wiki": opt_str("Target wiki name"),
                    "cross_wiki": opt_bool("Search across all wikis"),
                    "format": opt_str("Output format: json | llms (default: json)"),
                }),
                &["query"],
            ),
        ),
        Tool::new(
            "wiki_list",
            "Paginated page listing with filters.",
            schema(
                json!({
                    "type": opt_str("Filter by frontmatter type"),
                    "status": opt_str("Filter by frontmatter status"),
                    "page": opt_int("Page number, 1-based"),
                    "page_size": opt_int("Results per page (default: 20, max: 200)"),
                    "wiki": opt_str("Target wiki name"),
                    "format": opt_str("Output format: json | llms (default: json)"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_ingest",
            "Validate, commit, and index files in the wiki tree.",
            schema(
                json!({
                    "path": str_prop("File or folder path, relative to wiki root"),
                    "dry_run": opt_bool("Show what would be created without creating"),
                    "redact": opt_bool("Run redaction pass on file bodies before validation (opt-in; lossy — original values are replaced)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["path"],
            ),
        ),
        Tool::new(
            "wiki_index_rebuild",
            "Rebuild the tantivy search index.",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_index_status",
            "Return detailed health for the search index of one wiki — reports openable, queryable, stale, and degraded_reason. Call this when wiki_info shows index_status: \"degraded\" to identify the specific failure.",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_graph",
            "Generate concept graph, returns GraphReport.",
            schema(
                json!({
                    "format": opt_str("Output format: mermaid | dot | llms | json (default: mermaid)"),
                    "root": opt_str("Subgraph from this node (slug)"),
                    "depth": opt_int("Hop limit from root"),
                    "type": opt_str("Comma-separated page types to include"),
                    "relation": opt_str("Filter edges by relation label"),
                    "output": opt_str("File path for output (default: stdout/return)"),
                    "cross_wiki": opt_bool("Merge all mounted wikis into a unified graph"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_export",
            "Export the full wiki to a file (llms.txt, llms-full, or json)",
            schema(
                json!({
                    "wiki": str_prop("Target wiki name"),
                    "path": opt_str("Output path (relative to wiki root or absolute; default: llms.txt)"),
                    "format": opt_str("Export format: llms-txt | llms-full | json (default: llms-txt)"),
                    "status": opt_str("Page status filter: active | all (default: active, excludes archived)"),
                }),
                &["wiki"],
            ),
        ),
        Tool::new(
            "wiki_history",
            "Git commit history for a page.",
            schema(
                json!({
                    "slug": str_prop("Slug or wiki:// URI"),
                    "limit": opt_int("Max entries to return"),
                    "follow": opt_bool("Track renames (default: from config)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["slug"],
            ),
        ),
        Tool::new(
            "wiki_stats",
            "Wiki health dashboard — page counts, graph metrics, staleness, structural topology (diameter, radius, center).",
            schema(
                json!({
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_suggest",
            "Suggest related pages to link. Returns ranked slug candidates for link insertion, scored by shared concept overlap with the target page.",
            schema(
                json!({
                    "slug": str_prop("Slug or wiki:// URI"),
                    "limit": opt_int("Max suggestions"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["slug"],
            ),
        ),
        Tool::new(
            "wiki_lint",
            "Run deterministic lint checks on wiki pages — validates frontmatter fields, detects broken links, and reports structural issues.",
            schema(
                json!({
                    "rules": opt_str("Comma-separated rule names: orphan, broken-link, broken-cross-wiki-link, missing-fields, stale, unknown-type, articulation-point, bridge, periphery (omit for all)"),
                    "severity": opt_str("Filter output: error | warning (omit for all)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &[],
            ),
        ),
        Tool::new(
            "wiki_resolve",
            "Resolve a slug or wiki:// URI to its local filesystem path. Use before writing content directly to disk.",
            schema(
                json!({
                    "uri": str_prop("Slug or wiki:// URI"),
                    "wiki": opt_str("Target wiki name (optional, uses default)"),
                }),
                &["uri"],
            ),
        ),
        Tool::new(
            "wiki_schema",
            "List, show, or validate the YAML type schemas that define frontmatter structure for wiki pages. Actions: list, show, validate.",
            schema(
                json!({
                    "action": str_prop("Action: list, show, add, remove, validate"),
                    "type": opt_str("Type name (for show/add/remove/validate)"),
                    "template": opt_bool("Return frontmatter template instead of schema (for show)"),
                    "schema_path": opt_str("Path to schema file (for add)"),
                    "delete": opt_bool("Also delete schema file (for remove)"),
                    "delete_pages": opt_bool("Also delete page files from disk (for remove)"),
                    "dry_run": opt_bool("Show what would be done (for remove)"),
                    "wiki": opt_str("Target wiki name"),
                }),
                &["action"],
            ),
        ),
        Tool::new(
            "wiki_info",
            "Return server version, config path, registered spaces, and index health.",
            schema(json!({}), &[]),
        ),
    ]
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

/// Dispatch a tool call by name to the appropriate handler, catching panics.
pub fn call(server: &McpServer, name: &str, args: &Map<String, Value>) -> ToolResult {
    let _span = tracing::info_span!("tool_call", tool = name).entered();
    let max_len = server.engine().config.serve.mcp_max_param_len;
    if let Err(e) = check_param_lengths(args, max_len) {
        return ToolResult {
            content: err_text(e),
            is_error: true,
            notify_uris: vec![],
            notify_resources_changed: false,
        };
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match name {
        "wiki_spaces_create" => handlers::handle_spaces_create(server, args),
        "wiki_spaces_register" => handlers::handle_spaces_register(server, args),
        "wiki_spaces_list" => handlers::handle_spaces_list(server, args),
        "wiki_spaces_remove" => handlers::handle_spaces_remove(server, args),
        "wiki_spaces_set_default" => handlers::handle_spaces_set_default(server, args),
        "wiki_config" => handlers::handle_config(server, args),
        "wiki_content_read" => handlers::handle_content_read(server, args),
        "wiki_content_write" => handlers::handle_content_write(server, args),
        "wiki_content_new" => handlers::handle_content_new(server, args),
        "wiki_content_commit" => handlers::handle_content_commit(server, args),
        "wiki_search" => handlers::handle_search(server, args),
        "wiki_list" => handlers::handle_list(server, args),
        "wiki_ingest" => handlers::handle_ingest(server, args),
        "wiki_index_rebuild" => handlers::handle_index_rebuild(server, args),
        "wiki_index_status" => handlers::handle_index_status(server, args),
        "wiki_graph" => handlers::handle_graph(server, args),
        "wiki_history" => handlers::handle_history(server, args),
        "wiki_stats" => handlers::handle_stats(server, args),
        "wiki_lint" => handlers::handle_lint(server, args),
        "wiki_resolve" => handlers::handle_resolve(server, args),
        "wiki_suggest" => handlers::handle_suggest(server, args),
        "wiki_schema" => handlers::handle_schema(server, args),
        "wiki_export" => handlers::handle_export(server, args),
        "wiki_info" => handlers::handle_info(server, args),
        _ => Err(format!("unknown tool: {name}")),
    }));
    match result {
        Ok(Ok((content, notify_uris))) => {
            let notify_resources_changed = matches!(
                name,
                "wiki_spaces_create"
                    | "wiki_spaces_register"
                    | "wiki_spaces_remove"
                    | "wiki_spaces_set_default"
            );
            tracing::debug!(tool = name, "tool call ok");
            ToolResult {
                content,
                is_error: false,
                notify_uris,
                notify_resources_changed,
            }
        }
        Ok(Err(msg)) => {
            tracing::warn!(tool = name, error = %msg, "tool call failed");
            ToolResult {
                content: err_text(msg),
                is_error: true,
                notify_uris: vec![],
                notify_resources_changed: false,
            }
        }
        Err(_) => {
            tracing::error!(tool = name, "tool handler panicked");
            ToolResult {
                content: err_text("internal error: tool panicked".into()),
                is_error: true,
                notify_uris: vec![],
                notify_resources_changed: false,
            }
        }
    }
}
