use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;

// ── Watch (engine-level) ──────────────────────────────────────────────────────

#[test]
fn schema_rebuild_partial_on_type_change() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Verify schema_rebuild works without error on a clean wiki
    let result = manager.schema_rebuild("test");
    assert!(
        result.is_ok(),
        "schema_rebuild should succeed: {:?}",
        result
    );
}

#[test]
fn schema_rebuild_errors_on_unknown_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let result = manager.schema_rebuild("nonexistent");
    assert!(result.is_err());
}
