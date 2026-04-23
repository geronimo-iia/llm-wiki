use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

// ── Graph ─────────────────────────────────────────────────────────────────────

#[test]
fn graph_build_returns_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::graph_build(
        &engine,
        "test",
        &ops::GraphParams {
            format: Some("mermaid"),
            root: None,
            depth: None,
            type_filter: None,
            relation: None,
            output: None,
        },
    )
    .unwrap();
    assert!(result.report.nodes >= 2);
    assert!(result.rendered.contains("graph LR"));
}

#[test]
fn graph_build_dot_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::graph_build(
        &engine,
        "test",
        &ops::GraphParams {
            format: Some("dot"),
            root: None,
            depth: None,
            type_filter: None,
            relation: None,
            output: None,
        },
    )
    .unwrap();
    assert!(result.rendered.contains("digraph wiki"));
}
