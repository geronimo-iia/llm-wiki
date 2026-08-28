use super::helpers::setup_wiki;
use llm_wiki_engine::engine::WikiEngine;
use llm_wiki_engine::graph::{GraphFilter, get_or_build_graph};
use llm_wiki_engine::ops;

// ── Index ─────────────────────────────────────────────────────────────────────

#[test]
fn index_rebuild_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let report = ops::index_rebuild(&manager, "test").unwrap();
    assert!(report.pages_indexed >= 2);

    let engine = manager.state_for_test().read().unwrap();
    let status = ops::index_status(&engine, "test").unwrap();
    assert!(status.openable);
    assert!(status.queryable);
    assert!(!status.stale);
}

#[test]
fn index_rebuild_populates_graph_cache() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    ops::index_rebuild(&manager, "test").unwrap();

    let engine = manager.state_for_test().read().unwrap();
    let space = engine.spaces.get("test").unwrap();
    let searcher = space.index_manager.searcher().unwrap();

    let graph = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter::default(),
    )
    .unwrap();

    assert!(
        graph.node_count() >= 2,
        "graph cache must be populated after ops::index_rebuild: got {} nodes",
        graph.node_count()
    );
}
