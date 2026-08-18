use std::path::Path;

use rmcp::model::ContentBlock as Content;
use serde_json::{Map, Value};

use crate::engine::EngineState;
use crate::slug::Slug;

// ── ToolResult ────────────────────────────────────────────────────────────────

/// The unified return value from an MCP tool call.
pub struct ToolResult {
    /// MCP content blocks to return to the client.
    pub content: Vec<Content>,
    /// True if the tool call encountered an error.
    pub is_error: bool,
    /// `wiki://` URIs whose resource content has changed (triggers `resources/updated`).
    pub notify_uris: Vec<String>,
    /// True if the resource list has changed (triggers `resources/list_changed`).
    pub notify_resources_changed: bool,
}

// ── Handler result type ───────────────────────────────────────────────────────

/// Return type for individual MCP tool handler functions: `(content, notify_uris)` or an error string.
pub type ToolHandlerResult = Result<(Vec<Content>, Vec<String>), String>;

/// Wrap a plain text string as a successful `ToolHandlerResult` with no URI notifications.
pub fn ok_text(text: String) -> ToolHandlerResult {
    Ok((vec![Content::text(text)], vec![]))
}

/// Wrap an error message as an MCP content block with `"error: "` prefix.
pub fn err_text(msg: String) -> Vec<Content> {
    vec![Content::text(format!("error: {msg}"))]
}

// ── Param length guard ────────────────────────────────────────────────────────

/// Fallback maximum byte length for MCP string parameters (matches `ServeConfig` default).
pub const MAX_PARAM_LEN: usize = 8192;

/// Reject any call whose string arguments exceed `max_len` bytes.
///
/// Called once at the dispatch layer before routing to the individual handler,
/// so all tools are covered without touching per-handler argument parsing.
pub fn check_param_lengths(args: &Map<String, Value>, max_len: usize) -> Result<(), String> {
    for (key, value) in args {
        let len = match value {
            Value::String(s) => s.len(),
            Value::Object(_) | Value::Array(_) => {
                // Serialized length bounds nested strings without a recursive walk.
                serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
            }
            _ => 0,
        };
        if len > max_len {
            return Err(format!(
                "parameter '{key}' exceeds maximum length of {max_len} bytes"
            ));
        }
    }
    Ok(())
}

// ── Argument helpers ──────────────────────────────────────────────────────────

/// Extract an optional string argument by key from tool call arguments.
pub fn arg_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract a required string argument by key, returning an error string if absent.
pub fn arg_str_req(args: &Map<String, Value>, key: &str) -> Result<String, String> {
    arg_str(args, key).ok_or_else(|| format!("missing required parameter: {key}"))
}

/// Extract a boolean argument by key; returns `false` if absent or not a boolean.
pub fn arg_bool(args: &Map<String, Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Extract an optional unsigned integer argument by key.
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
    engine
        .resolve_wiki_name(name.as_deref())
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

// ── Resource notification helper ──────────────────────────────────────────────

/// Collect `wiki://` URIs for all Markdown files under `path` (file or directory).
pub fn collect_page_uris(path: &Path, wiki_root: &Path, wiki_name: &str) -> Vec<String> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("md")
            && let Ok(slug) = Slug::from_path(path, wiki_root)
        {
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
        .filter_map(|e| {
            Slug::from_path(e.path(), wiki_root)
                .ok()
                .map(|slug| format!("wiki://{wiki_name}/{slug}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_args(key: &str, val: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert(key.to_string(), val);
        m
    }

    #[test]
    fn test_check_param_lengths_rejects_oversized() {
        let long = "x".repeat(MAX_PARAM_LEN + 1);
        let args = make_args("query", json!(long));
        let result = check_param_lengths(&args, MAX_PARAM_LEN);
        assert!(result.is_err(), "expected Err for oversized param");
        assert!(result.unwrap_err().contains("exceeds maximum length"));
    }

    #[test]
    fn test_check_param_lengths_accepts_within_limit() {
        let val = "x".repeat(MAX_PARAM_LEN);
        let args = make_args("query", json!(val));
        assert!(check_param_lengths(&args, MAX_PARAM_LEN).is_ok());
    }

    #[test]
    fn test_check_param_lengths_accepts_empty() {
        let args = serde_json::Map::new();
        assert!(check_param_lengths(&args, MAX_PARAM_LEN).is_ok());
    }

    #[test]
    fn test_check_param_lengths_rejects_oversized_nested_object() {
        // Non-string value: nested object with a large string field.
        // as_str() returns None for objects, so the old code silently skipped this.
        let long = "x".repeat(MAX_PARAM_LEN + 1);
        let nested = json!({ "body": long });
        let args = make_args("meta", nested);
        let result = check_param_lengths(&args, MAX_PARAM_LEN);
        assert!(
            result.is_err(),
            "expected Err for oversized nested object param"
        );
    }

    #[test]
    fn test_check_param_lengths_accepts_small_nested_object() {
        let args = make_args("meta", json!({ "type": "concept" }));
        assert!(check_param_lengths(&args, MAX_PARAM_LEN).is_ok());
    }

    #[test]
    fn test_check_param_lengths_skips_number_and_bool() {
        let mut args = serde_json::Map::new();
        args.insert("depth".to_string(), json!(5));
        args.insert("enabled".to_string(), json!(true));
        assert!(check_param_lengths(&args, MAX_PARAM_LEN).is_ok());
    }
}
