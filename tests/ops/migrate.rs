use llm_wiki_engine::config::WikiEntry;
use llm_wiki_engine::{config, default_schemas, ops};
use tempfile::tempdir;

fn setup_wiki_with_schemas(stock: bool, custom: bool) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let wiki_path = dir.path().join("wiki");
    let schemas_dir = wiki_path.join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    if stock {
        let content = default_schemas::default_schemas()["concept.json"];
        std::fs::write(schemas_dir.join("concept.json"), content).unwrap();
    }
    if custom {
        let custom_schema = serde_json::json!({
            "x-wiki-types": { "mytype": "custom" },
            "properties": {}
        });
        std::fs::write(
            schemas_dir.join("mytype.json"),
            serde_json::to_string(&custom_schema).unwrap(),
        )
        .unwrap();
    }
    (dir, wiki_path)
}

#[test]
fn migrate_deletes_stock_schema() {
    let (dir, wiki_path) = setup_wiki_with_schemas(true, false);
    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    assert_eq!(report.wikis.len(), 1);
    let w = &report.wikis[0];
    assert!(w.deleted.contains(&"concept.json".to_string()));
    assert!(w.kept_custom.is_empty());
    assert!(!wiki_path.join("schemas").join("concept.json").exists());
}

#[test]
fn migrate_keeps_custom_schema() {
    let (dir, wiki_path) = setup_wiki_with_schemas(false, true);
    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    let w = &report.wikis[0];
    assert!(w.deleted.is_empty());
    assert!(w.kept_custom.contains(&"mytype.json".to_string()));
    assert!(wiki_path.join("schemas").join("mytype.json").exists());
    assert!(
        w.already_clean,
        "wiki with only custom schemas must be already_clean"
    );
}

#[test]
fn migrate_dry_run_does_not_delete() {
    let (dir, wiki_path) = setup_wiki_with_schemas(true, false);
    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), true).unwrap();

    let w = &report.wikis[0];
    assert!(!w.deleted.is_empty());
    assert!(wiki_path.join("schemas").join("concept.json").exists());
}

#[test]
fn migrate_already_clean_wiki() {
    let (dir, wiki_path) = setup_wiki_with_schemas(false, false);
    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    assert!(report.wikis[0].already_clean);
}

// ── Cross-version tests ───────────────────────────────────────────────────────
// These tests use the actual archived pre-1.0.0 schema content to verify that
// wiki_migrate correctly recognises stale copies from before the 1.0.0 release.

#[test]
fn migrate_recognises_pre100_concept_as_stock() {
    // Simulate a wiki created before v1.0.0: concept.json on disk is the
    // archived pre-1.0.0 version (missing x-keyword), not the current one.
    let dir = tempdir().unwrap();
    let wiki_path = dir.path().join("wiki");
    let schemas_dir = wiki_path.join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    // Write the archived pre-1.0.0 content — this is what a user who created
    // their wiki before 1.0.0 would have on disk.
    std::fs::write(
        schemas_dir.join("concept.json"),
        default_schemas::ARCHIVE_PRE100_CONCEPT,
    )
    .unwrap();

    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    let w = &report.wikis[0];
    assert!(
        w.deleted.contains(&"concept.json".to_string()),
        "pre-1.0.0 concept.json must be recognised as stock and deleted"
    );
    assert!(w.kept_custom.is_empty());
    assert!(!schemas_dir.join("concept.json").exists());
}

#[test]
fn migrate_recognises_all_pre100_archived_files_as_stock() {
    // All 5 archived files placed on disk must all be deleted by migrate.
    let dir = tempdir().unwrap();
    let wiki_path = dir.path().join("wiki");
    let schemas_dir = wiki_path.join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    std::fs::write(schemas_dir.join("base.json"), default_schemas::ARCHIVE_PRE100_BASE).unwrap();
    std::fs::write(
        schemas_dir.join("concept.json"),
        default_schemas::ARCHIVE_PRE100_CONCEPT,
    )
    .unwrap();
    std::fs::write(schemas_dir.join("doc.json"), default_schemas::ARCHIVE_PRE100_DOC).unwrap();
    std::fs::write(schemas_dir.join("paper.json"), default_schemas::ARCHIVE_PRE100_PAPER).unwrap();
    std::fs::write(
        schemas_dir.join("section.json"),
        default_schemas::ARCHIVE_PRE100_SECTION,
    )
    .unwrap();

    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    let w = &report.wikis[0];
    assert_eq!(w.deleted.len(), 5, "all 5 pre-1.0.0 archived files must be deleted");
    assert!(w.kept_custom.is_empty());
    assert!(!w.already_clean, "wiki had 5 stock files — was not already clean before migration");
}

#[test]
fn migrate_keeps_user_modified_schema_that_resembles_pre100() {
    // A schema that looks like pre-1.0.0 but has an extra user field must NOT
    // be deleted — it is a genuine customisation, not a stock copy.
    let dir = tempdir().unwrap();
    let wiki_path = dir.path().join("wiki");
    let schemas_dir = wiki_path.join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    // Start from the pre-1.0.0 archived content and add a user field
    let mut val: serde_json::Value =
        serde_json::from_str(default_schemas::ARCHIVE_PRE100_CONCEPT).unwrap();
    val["x-user-extension"] = serde_json::json!(true);
    std::fs::write(
        schemas_dir.join("concept.json"),
        serde_json::to_string(&val).unwrap(),
    )
    .unwrap();

    let cfg = dir.path().join("config.toml");
    let entry = WikiEntry {
        name: "mywiki".into(),
        path: wiki_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry, false, &cfg).unwrap();

    let global = config::load_global(&cfg).unwrap();
    let report = ops::wiki_migrate(&global, Some("mywiki"), false).unwrap();

    let w = &report.wikis[0];
    assert!(w.deleted.is_empty());
    assert!(
        w.kept_custom.contains(&"concept.json".to_string()),
        "user-modified schema must be kept as custom override"
    );
    assert!(w.already_clean, "nothing deleted means already_clean");
    assert!(schemas_dir.join("concept.json").exists());
}
