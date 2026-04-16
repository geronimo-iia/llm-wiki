use std::fs;
use std::path::Path;

use llm_wiki::config;
use llm_wiki::init::init;

fn config_path(dir: &Path) -> std::path::PathBuf {
    dir.join("dot-wiki").join("config.toml")
}

#[test]
fn init_creates_wiki_structure() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    let report = init(&wiki_path, "research", Some("test wiki"), false, false, &cfg).unwrap();

    assert!(report.created);
    assert!(report.registered);
    assert!(report.committed);
    assert!(wiki_path.join("wiki").is_dir());
    assert!(wiki_path.join("inbox").is_dir());
    assert!(wiki_path.join("raw").is_dir());
    assert!(wiki_path.join("README.md").is_file());
    assert!(wiki_path.join("wiki.toml").is_file());
    assert!(wiki_path.join("schema.md").is_file());
    assert!(wiki_path.join(".git").is_dir());
}

#[test]
fn init_creates_logs_directory() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", None, false, false, &cfg).unwrap();

    let logs_dir = cfg.parent().unwrap().join("logs");
    assert!(logs_dir.is_dir(), "~/.wiki/logs/ should be created by init");
}

#[test]
fn init_registers_in_global_config() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", Some("ML wiki"), false, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    assert_eq!(global.wikis.len(), 1);
    assert_eq!(global.wikis[0].name, "research");
    assert_eq!(global.wikis[0].description.as_deref(), Some("ML wiki"));
}

#[test]
fn init_set_default_sets_default_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", None, false, true, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    assert_eq!(global.global.default_wiki, "research");
}

#[test]
fn init_rerun_same_name_skips() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let report = init(&wiki_path, "research", None, false, false, &cfg).unwrap();

    assert!(!report.created);
    assert!(!report.registered);
    assert!(!report.committed);
}

#[test]
fn init_rerun_different_name_errors_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let result = init(&wiki_path, "research-v2", None, false, false, &cfg);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--force"));
}

#[test]
fn init_force_allows_rename() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    init(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let report = init(&wiki_path, "research-v2", None, true, false, &cfg).unwrap();

    assert!(report.registered);
    let global = config::load_global(&cfg).unwrap();
    assert!(global.wikis.iter().any(|w| w.name == "research-v2"));
}

#[test]
fn init_logs_dir_already_exists_is_fine() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    // Pre-create logs dir
    let logs_dir = cfg.parent().unwrap().join("logs");
    fs::create_dir_all(&logs_dir).unwrap();

    let report = init(&wiki_path, "research", None, false, false, &cfg).unwrap();
    assert!(report.created);
    assert!(logs_dir.is_dir());
}
