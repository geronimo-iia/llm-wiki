use std::path::Path;

use rmcp::model::Content;
use serde_json::{Map, Value};

use crate::engine::EngineState;
use crate::slug::Slug;

// ── ToolResult ────────────────────────────────────────────────────────────────

pub struct ToolResult {
    pub content: Vec<Content>,
    pub is_error: bool,
    pub notify_uris: Vec<String>,
}

// ── Handler result type ───────────────────────────────────────────────────────

pub type ToolHandlerResult = Result<(Vec<Content>, Vec<String>), String>;

pub fn ok_text(text: String) -> ToolHandlerResult {
    Ok((vec![Content::text(text)], vec![]))
}

pub fn err_text(msg: String) -> Vec<Content> {
    vec![Content::text(format!("error: {msg}"))]
}

// ── Argument helpers ──────────────────────────────────────────────────────────

pub fn arg_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn arg_str_req(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    arg_str(args, key).ok_or_else(|| format!("missing required parameter: {key}"))
}

pub fn arg_bool(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

pub fn arg_usize(args: &Map<String, Value>, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

// ── Wiki resolution ───────────────────────────────────────────────────────────

/// Resolve the target wiki from Engine state + optional `wiki` arg.
/// Resolve the target wiki from Engine state + optional `wiki` arg.
pub fn resolve_wiki_name(
    engine: &EngineState,
    args: &Map<String, Value>,
) -> Result<String, String> {
    let name = arg_str(args, "wiki");
    Ok(engine.resolve_wiki_name(name.as_deref()).to_string())
}

// ── Resource notification helper ──────────────────────────────────────────────

pub fn collect_page_uris(path: &Path, wiki_root: &Path, wiki_name: &str) -> Vec<String> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Ok(slug) = Slug::from_path(path, wiki_root) {
                return vec![format!("wiki://{wiki_name}/{slug}")];
            }
        }
        return vec![];
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file() && e.path().extension().and_then(|x| x.to_str()) == Some("md")
        })
        .filter_map(|e| {
            Slug::from_path(e.path(), wiki_root)
                .ok()
                .map(|slug| format!("wiki://{wiki_name}/{slug}"))
        })
        .collect()
}
