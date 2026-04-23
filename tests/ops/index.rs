use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

// ── Index ─────────────────────────────────────────────────────────────────────

#[test]
fn index_rebuild_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let report = ops::index_rebuild(&manager, "test").unwrap();
    assert!(report.pages_indexed >= 2);

    let engine = manager.state.read().unwrap();
    let status = ops::index_status(&engine, "test").unwrap();
    assert!(status.openable);
    assert!(status.queryable);
    assert!(!status.stale);
}
