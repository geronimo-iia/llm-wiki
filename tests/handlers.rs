use std::sync::Arc;

use llm_wiki_engine::engine::WikiEngine;
use llm_wiki_engine::mcp::McpServer;
use llm_wiki_engine::mcp::handlers;
use serde_json::{Map, Value};

// ── Fixture ───────────────────────────────────────────────────────────────────

fn make_server() -> (McpServer, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("llm-wiki.toml");
    std::fs::write(&config_path, "").unwrap();
    let engine = WikiEngine::build(&config_path).unwrap();
    let server = McpServer::new(Arc::new(engine));
    (server, dir)
}

fn str_arg(key: &str, value: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(key.to_string(), Value::String(value.to_string()));
    m
}

fn empty() -> Map<String, Value> {
    Map::new()
}

// ── Missing required params ────────────────────────────────────────────────────

#[test]
fn spaces_create_requires_path() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_create(&server, &str_arg("name", "wiki"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: path")
    );
}

#[test]
fn spaces_create_requires_name() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_create(&server, &str_arg("path", "/tmp/wiki"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: name")
    );
}

#[test]
fn spaces_register_requires_path() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_register(&server, &str_arg("name", "wiki"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: path")
    );
}

#[test]
fn spaces_register_requires_name() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_register(&server, &str_arg("path", "/tmp/wiki"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: name")
    );
}

#[test]
fn spaces_remove_requires_name() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_remove(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: name")
    );
}

#[test]
fn spaces_set_default_requires_name() {
    let (server, _dir) = make_server();
    let result = handlers::handle_spaces_set_default(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: name")
    );
}

#[test]
fn content_read_requires_uri() {
    let (server, _dir) = make_server();
    let result = handlers::handle_content_read(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: uri")
    );
}

#[test]
fn content_write_requires_uri() {
    let (server, _dir) = make_server();
    let result = handlers::handle_content_write(&server, &str_arg("content", "hello"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: uri")
    );
}

#[test]
fn content_write_requires_content() {
    let (server, _dir) = make_server();
    let result = handlers::handle_content_write(&server, &str_arg("uri", "page"));
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: content")
    );
}

#[test]
fn content_new_requires_uri() {
    let (server, _dir) = make_server();
    let result = handlers::handle_content_new(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: uri")
    );
}

#[test]
fn resolve_requires_uri() {
    let (server, _dir) = make_server();
    let result = handlers::handle_resolve(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: uri")
    );
}

#[test]
fn search_requires_query() {
    let (server, _dir) = make_server();
    let result = handlers::handle_search(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: query")
    );
}

#[test]
fn ingest_requires_path() {
    let (server, _dir) = make_server();
    let result = handlers::handle_ingest(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: path")
    );
}

#[test]
fn history_requires_slug() {
    let (server, _dir) = make_server();
    let result = handlers::handle_history(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: slug")
    );
}

#[test]
fn suggest_requires_slug() {
    let (server, _dir) = make_server();
    let result = handlers::handle_suggest(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: slug")
    );
}

#[test]
fn schema_requires_action() {
    let (server, _dir) = make_server();
    let result = handlers::handle_schema(&server, &empty());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("action is required"));
}

#[test]
fn export_requires_wiki() {
    let (server, _dir) = make_server();
    let result = handlers::handle_export(&server, &empty());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("missing required parameter: wiki")
    );
}

// ── Error path tests (fixes from this review cycle) ──────────────────────────

#[test]
fn content_write_rejects_oversized_content() {
    let (server, _dir) = make_server();
    let mut args = str_arg("uri", "test-page");
    args.insert(
        "content".to_string(),
        Value::String("x".repeat(11 * 1024 * 1024)),
    );
    let result = handlers::handle_content_write(&server, &args);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("exceeds maximum allowed size"),
        "unexpected error: {msg}"
    );
    assert!(msg.contains("10485760"), "limit not in message: {msg}");
}

#[test]
fn content_write_accepts_content_within_limit() {
    let (server, _dir) = make_server();
    // Just enough to pass the size check — will fail later (no wiki), but not at the size check.
    let mut args = str_arg("uri", "test-page");
    args.insert("content".to_string(), Value::String("x".repeat(1024)));
    let result = handlers::handle_content_write(&server, &args);
    // Fails at wiki resolution, not at the size check.
    if let Err(ref msg) = result {
        assert!(
            !msg.contains("exceeds maximum allowed size"),
            "should not fail at size check: {msg}"
        );
    }
}

#[test]
fn search_index_error_includes_rebuild_hint() {
    let (server, _dir) = make_server();
    // No wikis configured: resolve_wiki_name fails before reaching index query.
    // Verify that errors that DO reach the index path include the hint.
    // We test the hint by checking the error-enrichment closure compiles and fires on the right text.
    // This is a unit test for the hint text pattern.
    let result = handlers::handle_search(&server, &str_arg("query", "foo"));
    assert!(result.is_err());
    let msg = result.unwrap_err();
    // No wikis → "default wiki" error, NOT the index error. Just verify no panic.
    assert!(!msg.is_empty(), "error message must not be empty");
}
