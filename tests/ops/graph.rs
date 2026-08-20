use super::helpers::setup_wiki;
use llm_wiki_engine::engine::WikiEngine;
use llm_wiki_engine::ops;

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
            cross_wiki: false,
            limit: None,
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
            cross_wiki: false,
            limit: None,
        },
    )
    .unwrap();
    assert!(result.rendered.contains("digraph wiki"));
}

#[test]
fn graph_build_summary_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::graph_build(
        &engine,
        "test",
        &ops::GraphParams {
            format: Some("summary"),
            root: None,
            depth: None,
            type_filter: None,
            relation: None,
            output: None,
            cross_wiki: false,
            limit: None,
        },
    )
    .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&result.rendered).expect("summary format must produce valid JSON");
    assert!(v["nodes"].as_u64().is_some());
    assert!(v["isolated_count"].as_u64().is_some());
}
