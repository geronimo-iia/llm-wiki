use std::path::Path;

use llm_wiki::config::*;
use llm_wiki::spaces;

fn config_path(dir: &Path) -> std::path::PathBuf {
    dir.join("dot-wiki").join("config.toml")
}

fn make_entry(name: &str, path: &str) -> WikiEntry {
    WikiEntry {
        name: name.into(),
        path: path.into(),
        description: None,
        remote: None,
    }
}

// ── create ────────────────────────────────────────────────────────────────────

#[test]
fn create_builds_wiki_structure() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    let report = spaces::create(&wiki_path, "research", Some("test wiki"), false, false, &cfg)
        .unwrap();

    assert!(report.created);
    assert!(report.registered);
    assert!(report.committed);
    assert!(wiki_path.join("wiki").is_dir());
    assert!(wiki_path.join("inbox").is_dir());
    assert!(wiki_path.join("raw").is_dir());
    assert!(wiki_path.join("schemas").is_dir());
    assert!(wiki_path.join("README.md").is_file());
    assert!(wiki_path.join("wiki.toml").is_file());
    assert!(wiki_path.join(".git").is_dir());
    // No schema.md — eliminated
    assert!(!wiki_path.join("schema.md").exists());
}

#[test]
fn create_registers_in_global_config() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", Some("ML wiki"), false, false, &cfg).unwrap();

    let global = load_global(&cfg).unwrap();
    assert_eq!(global.wikis.len(), 1);
    assert_eq!(global.wikis[0].name, "research");
    assert_eq!(global.wikis[0].description.as_deref(), Some("ML wiki"));
}

#[test]
fn create_set_default() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, true, &cfg).unwrap();

    let global = load_global(&cfg).unwrap();
    assert_eq!(global.global.default_wiki, "research");
}

#[test]
fn create_creates_logs_directory() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg).unwrap();

    let logs_dir = cfg.parent().unwrap().join("logs");
    assert!(logs_dir.is_dir());
}

#[test]
fn create_rerun_same_name_skips() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let report = spaces::create(&wiki_path, "research", None, false, false, &cfg).unwrap();

    assert!(!report.created);
    assert!(!report.registered);
    assert!(!report.committed);
}

#[test]
fn create_rerun_different_name_errors_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let result = spaces::create(&wiki_path, "research-v2", None, false, false, &cfg);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--force"));
}

#[test]
fn create_force_allows_rename() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg).unwrap();
    let report = spaces::create(&wiki_path, "research-v2", None, true, false, &cfg).unwrap();

    assert!(report.registered);
    let global = load_global(&cfg).unwrap();
    assert!(global.wikis.iter().any(|w| w.name == "research-v2"));
}

// ── register ──────────────────────────────────────────────────────────────────

#[test]
fn register_appends_entry() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    spaces::register(make_entry("test", "/tmp/test"), false, &cfg).unwrap();

    let config = load_global(&cfg).unwrap();
    assert_eq!(config.wikis.len(), 1);
    assert_eq!(config.wikis[0].name, "test");
}

#[test]
fn register_force_updates_existing() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    spaces::register(make_entry("test", "/tmp/test1"), false, &cfg).unwrap();
    spaces::register(make_entry("test", "/tmp/test2"), true, &cfg).unwrap();

    let config = load_global(&cfg).unwrap();
    assert_eq!(config.wikis.len(), 1);
    assert_eq!(config.wikis[0].path, "/tmp/test2");
}

#[test]
fn register_errors_on_duplicate_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    spaces::register(make_entry("test", "/tmp/test"), false, &cfg).unwrap();
    assert!(spaces::register(make_entry("test", "/tmp/test"), false, &cfg).is_err());
}

// ── remove ────────────────────────────────────────────────────────────────────

#[test]
fn remove_removes_entry() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    spaces::register(make_entry("test", "/tmp/test"), false, &cfg).unwrap();
    spaces::remove("test", false, &cfg).unwrap();

    let config = load_global(&cfg).unwrap();
    assert!(config.wikis.is_empty());
}

#[test]
fn remove_with_delete_removes_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let wiki_dir = dir.path().join("mywiki");
    std::fs::create_dir_all(&wiki_dir).unwrap();

    let entry = WikiEntry {
        name: "test".into(),
        path: wiki_dir.to_string_lossy().into(),
        description: None,
        remote: None,
    };
    spaces::register(entry, false, &cfg).unwrap();
    spaces::remove("test", true, &cfg).unwrap();

    assert!(!wiki_dir.exists());
}

#[test]
fn remove_errors_when_wiki_is_default() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(
        &cfg,
        "[global]\ndefault_wiki = \"test\"\n\n[[wikis]]\nname = \"test\"\npath = \"/tmp/test\"\n",
    )
    .unwrap();

    let result = spaces::remove("test", false, &cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("default wiki"));
}

// ── set_default_wiki ──────────────────────────────────────────────────────────

#[test]
fn set_default_wiki_sets_default() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    spaces::register(make_entry("test", "/tmp/test"), false, &cfg).unwrap();
    spaces::set_default_wiki("test", &cfg).unwrap();

    let config = load_global(&cfg).unwrap();
    assert_eq!(config.global.default_wiki, "test");
}

#[test]
fn set_default_wiki_errors_on_unregistered() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(&cfg, "[global]\ndefault_wiki = \"\"\n").unwrap();

    assert!(spaces::set_default_wiki("nope", &cfg).is_err());
}

// ── load_all ──────────────────────────────────────────────────────────────────

#[test]
fn load_all_returns_all_entries() {
    let global = GlobalConfig {
        wikis: vec![make_entry("a", "/a"), make_entry("b", "/b")],
        ..Default::default()
    };
    let entries = spaces::load_all(&global);
    assert_eq!(entries.len(), 2);
}

// ── resolve_name ──────────────────────────────────────────────────────────────

#[test]
fn resolve_name_finds_entry() {
    let global = GlobalConfig {
        wikis: vec![make_entry("research", "/tmp/research")],
        ..Default::default()
    };
    let entry = spaces::resolve_name("research", &global).unwrap();
    assert_eq!(entry.name, "research");
}

#[test]
fn resolve_name_errors_on_missing() {
    let global = GlobalConfig::default();
    assert!(spaces::resolve_name("nope", &global).is_err());
}
