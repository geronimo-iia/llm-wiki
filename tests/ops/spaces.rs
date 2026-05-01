use super::helpers::setup_wiki;
use llm_wiki::ops;

// ── Spaces ────────────────────────────────────────────────────────────────────

#[test]
fn spaces_create_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let entries = ops::spaces_list(&global, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "test");
}

#[test]
fn spaces_list_filters_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let beta_path = dir.path().join("beta");
    ops::spaces_create(
        &beta_path,
        "beta",
        None,
        false,
        false,
        &config_path,
        None,
        None,
    )
    .unwrap();

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let filtered = ops::spaces_list(&global, Some("beta"));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "beta");
}

#[test]
fn spaces_list_unknown_name_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let filtered = ops::spaces_list(&global, Some("nonexistent"));
    assert!(filtered.is_empty());
}

#[test]
fn spaces_set_default_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    // Create a second wiki
    let beta_path = dir.path().join("beta");
    ops::spaces_create(
        &beta_path,
        "beta",
        None,
        false,
        false,
        &config_path,
        None,
        None,
    )
    .unwrap();

    ops::spaces_set_default("beta", &config_path, None).unwrap();
    let global = llm_wiki::config::load_global(&config_path).unwrap();
    assert_eq!(global.global.default_wiki, "beta");

    ops::spaces_remove("alpha", false, &config_path, None).unwrap();
    let global = llm_wiki::config::load_global(&config_path).unwrap();
    assert_eq!(global.wikis.len(), 1);
}
