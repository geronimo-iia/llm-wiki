use super::helpers::setup_wiki;
use llm_wiki::ops;

// ── Config ────────────────────────────────────────────────────────────────────

#[test]
fn config_get_returns_value() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let val = ops::config_get(&config_path, "defaults.search_top_k").unwrap();
    assert_eq!(val, "10");
}

#[test]
fn config_set_global() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let msg = ops::config_set(&config_path, "defaults.search_top_k", "20", true, None).unwrap();
    assert!(msg.contains("20"));

    let val = ops::config_get(&config_path, "defaults.search_top_k").unwrap();
    assert_eq!(val, "20");
}

#[test]
fn config_list_global_returns_toml() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let s = ops::config_list_global(&config_path).unwrap();
    assert!(s.contains("[global]"));
}

#[test]
fn config_list_resolved_returns_struct() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let resolved = ops::config_list_resolved(&config_path).unwrap();
    assert_eq!(resolved.defaults.search_top_k, 10);
}
