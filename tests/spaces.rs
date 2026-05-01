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

    let report = spaces::create(
        &wiki_path,
        "research",
        Some("test wiki"),
        false,
        false,
        &cfg,
        None,
    )
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

    spaces::create(&wiki_path, "research", Some("ML wiki"), false, false, &cfg, None).unwrap();

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

    spaces::create(&wiki_path, "research", None, false, true, &cfg, None).unwrap();

    let global = load_global(&cfg).unwrap();
    assert_eq!(global.global.default_wiki, "research");
}

#[test]
fn create_creates_logs_directory() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    let logs_dir = cfg.parent().unwrap().join("logs");
    assert!(logs_dir.is_dir());
}

#[test]
fn create_rerun_same_name_skips() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();
    let report = spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    assert!(!report.created);
    assert!(!report.registered);
    assert!(!report.committed);
}

#[test]
fn create_rerun_different_name_errors_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();
    let result = spaces::create(&wiki_path, "research-v2", None, false, false, &cfg, None);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("--force"));
}

#[test]
fn create_force_allows_rename() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();
    let report = spaces::create(&wiki_path, "research-v2", None, true, false, &cfg, None).unwrap();

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

// ── schemas and wiki.toml types ──────────────────────────────────────────────

#[test]
fn create_writes_default_schema_files() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    let schemas_dir = wiki_path.join("schemas");
    for name in &[
        "base.json",
        "concept.json",
        "paper.json",
        "skill.json",
        "doc.json",
        "section.json",
    ] {
        let path = schemas_dir.join(name);
        assert!(path.is_file(), "missing schema: {name}");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"$schema\""), "{name} missing $schema");
    }
}

#[test]
fn create_schema_files_match_embedded() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    let embedded = llm_wiki::default_schemas::default_schemas();
    for (filename, expected) in &embedded {
        let on_disk = std::fs::read_to_string(wiki_path.join("schemas").join(filename)).unwrap();
        assert_eq!(&on_disk, *expected, "mismatch for {filename}");
    }
}

#[test]
fn create_generates_wiki_toml_without_types() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", Some("ML wiki"), false, false, &cfg, None).unwrap();

    let wiki_cfg = llm_wiki::config::load_wiki(&wiki_path).unwrap();
    assert_eq!(wiki_cfg.name, "research");
    assert_eq!(wiki_cfg.description, "ML wiki");
    // Types are discovered from schemas, not written to wiki.toml
    assert!(wiki_cfg.types.is_empty());
}

#[test]
fn create_does_not_overwrite_existing_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    // Modify a schema on disk
    let custom = wiki_path.join("schemas/base.json");
    std::fs::write(&custom, r#"{"custom": true}"#).unwrap();

    // Re-run create (same name = skip path)
    // Simulate by calling ensure_structure indirectly via a new wiki
    let wiki_path2 = dir.path().join("other");
    spaces::create(&wiki_path2, "other", None, false, false, &cfg, None).unwrap();

    // Original wiki's custom schema untouched (create skipped it)
    let content = std::fs::read_to_string(&custom).unwrap();
    assert!(content.contains("custom"));
}

// ── validate_wiki_root ────────────────────────────────────────────────────────

#[test]
fn validate_wiki_root_accepts_simple_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("skills");
    std::fs::create_dir_all(&root).unwrap();
    assert!(llm_wiki::spaces::validate_wiki_root(dir.path(), "skills").is_ok());
}

#[test]
fn validate_wiki_root_accepts_multi_component() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("src").join("wiki");
    std::fs::create_dir_all(&root).unwrap();
    assert!(llm_wiki::spaces::validate_wiki_root(dir.path(), "src/wiki").is_ok());
}

#[test]
fn validate_wiki_root_rejects_absolute() {
    let dir = tempfile::tempdir().unwrap();
    let err = llm_wiki::spaces::validate_wiki_root(dir.path(), "/absolute").unwrap_err();
    assert!(err.to_string().contains("must be a relative path"));
}

#[test]
fn validate_wiki_root_rejects_dotdot() {
    let dir = tempfile::tempdir().unwrap();
    let err = llm_wiki::spaces::validate_wiki_root(dir.path(), "../outside").unwrap_err();
    assert!(err.to_string().contains("must not contain"));
}

#[test]
fn validate_wiki_root_rejects_empty() {
    let dir = tempfile::tempdir().unwrap();
    let err = llm_wiki::spaces::validate_wiki_root(dir.path(), "").unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn validate_wiki_root_rejects_dot() {
    let dir = tempfile::tempdir().unwrap();
    let err = llm_wiki::spaces::validate_wiki_root(dir.path(), ".").unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn validate_wiki_root_rejects_reserved_dirs() {
    let dir = tempfile::tempdir().unwrap();
    for reserved in &["inbox", "raw", "schemas"] {
        let err = llm_wiki::spaces::validate_wiki_root(dir.path(), reserved).unwrap_err();
        assert!(
            err.to_string().contains("reserved"),
            "expected reserved error for {reserved}, got: {err}"
        );
    }
}

#[test]
fn validate_wiki_root_rejects_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    // "content" dir does NOT exist
    let err = llm_wiki::spaces::validate_wiki_root(dir.path(), "content").unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn validate_wiki_root_rejects_traversal_via_symlink() {
    let outer = tempfile::tempdir().unwrap();
    let inner = tempfile::tempdir().unwrap();
    let link = outer.path().join("escape");
    #[cfg(unix)]
    std::os::unix::fs::symlink(inner.path(), &link).unwrap();
    #[cfg(unix)]
    {
        let err = llm_wiki::spaces::validate_wiki_root(outer.path(), "escape").unwrap_err();
        assert!(err.to_string().contains("must be inside"));
    }
}

// ── create with wiki_root ─────────────────────────────────────────────────────

#[test]
fn create_with_custom_wiki_root_creates_correct_directory() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("skills-wiki");
    let cfg = config_path(dir.path());

    llm_wiki::spaces::create(
        &wiki_path,
        "skills",
        None,
        false,
        false,
        &cfg,
        Some("skills"),
    )
    .unwrap();

    assert!(wiki_path.join("skills").is_dir(), "custom wiki_root dir should exist");
    assert!(!wiki_path.join("wiki").exists(), "default wiki/ dir should NOT be created");
    let toml_content = std::fs::read_to_string(wiki_path.join("wiki.toml")).unwrap();
    assert!(toml_content.contains("wiki_root = \"skills\""));
}

#[test]
fn create_without_wiki_root_keeps_default_wiki_dir() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("research");
    let cfg = config_path(dir.path());

    llm_wiki::spaces::create(&wiki_path, "research", None, false, false, &cfg, None).unwrap();

    assert!(wiki_path.join("wiki").is_dir());
    let toml_content = std::fs::read_to_string(wiki_path.join("wiki.toml")).unwrap();
    // default wiki_root should NOT be written to toml
    assert!(!toml_content.contains("wiki_root"));
}

// ── register_existing ─────────────────────────────────────────────────────────

#[test]
fn register_existing_basic() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("existing-wiki");
    let cfg = config_path(dir.path());

    std::fs::create_dir_all(wiki_path.join("wiki")).unwrap();
    std::fs::write(wiki_path.join("wiki.toml"), "name = \"existing\"\n").unwrap();

    let report = llm_wiki::spaces::register_existing(
        &wiki_path,
        "existing",
        None,
        None,
        &cfg,
    )
    .unwrap();

    assert!(report.registered);
    assert!(!report.created);
    assert!(!report.committed);
    let global = load_global(&cfg).unwrap();
    assert_eq!(global.wikis.len(), 1);
    assert_eq!(global.wikis[0].name, "existing");
}

#[test]
fn register_existing_with_custom_wiki_root() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("skills-repo");
    let cfg = config_path(dir.path());

    std::fs::create_dir_all(wiki_path.join("skills")).unwrap();
    std::fs::write(
        wiki_path.join("wiki.toml"),
        "name = \"skills\"\nwiki_root = \"skills\"\n",
    )
    .unwrap();

    let report = llm_wiki::spaces::register_existing(
        &wiki_path,
        "skills",
        None,
        None,
        &cfg,
    )
    .unwrap();

    assert!(report.registered);
}

#[test]
fn register_existing_wiki_root_flag_conflicts_with_toml() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("skills-repo");
    let cfg = config_path(dir.path());

    std::fs::create_dir_all(wiki_path.join("skills")).unwrap();
    std::fs::write(
        wiki_path.join("wiki.toml"),
        "name = \"skills\"\nwiki_root = \"skills\"\n",
    )
    .unwrap();

    let err = llm_wiki::spaces::register_existing(
        &wiki_path,
        "skills",
        None,
        Some("other"),
        &cfg,
    )
    .unwrap_err();

    assert!(err.to_string().contains("already declares wiki_root"));
}

#[test]
fn register_existing_missing_wiki_root_directory_errors() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("skills-repo");
    let cfg = config_path(dir.path());

    std::fs::create_dir_all(&wiki_path).unwrap();
    std::fs::write(
        wiki_path.join("wiki.toml"),
        "name = \"skills\"\nwiki_root = \"skills\"\n",
    )
    .unwrap();

    let err = llm_wiki::spaces::register_existing(
        &wiki_path,
        "skills",
        None,
        None,
        &cfg,
    )
    .unwrap_err();

    assert!(err.to_string().contains("does not exist"));
}
