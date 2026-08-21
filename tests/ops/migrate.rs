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
