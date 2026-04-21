use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_manager::SpaceIndexManager;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::search;
use llm_wiki::space_builder;
use llm_wiki::type_registry::SpaceTypeRegistry;
use tantivy::Searcher;

fn schema_and_registry() -> (IndexSchema, SpaceTypeRegistry) {
    let (registry, schema) = space_builder::build_space_from_embedded("en_stem");
    (schema, registry)
}

fn schema() -> IndexSchema {
    schema_and_registry().0
}

fn registry() -> SpaceTypeRegistry {
    schema_and_registry().1
}

fn setup_repo(dir: &Path) -> std::path::PathBuf {
    let wiki_root = dir.join("wiki");
    fs::create_dir_all(&wiki_root).unwrap();
    fs::create_dir_all(dir.join("inbox")).unwrap();
    fs::create_dir_all(dir.join("raw")).unwrap();
    git::init_repo(dir).unwrap();
    fs::write(dir.join("README.md"), "# test\n").unwrap();
    git::commit(dir, "init").unwrap();
    wiki_root
}

fn write_page(wiki_root: &Path, rel_path: &str, content: &str) {
    let path = wiki_root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn concept_page(title: &str, body: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"A concept\"\nstatus: active\ntype: concept\ntags:\n  - scaling\n---\n\n{body}\n"
    )
}

fn make_manager(dir: &Path) -> SpaceIndexManager {
    let index_path = dir.join("index-store");
    SpaceIndexManager::new("test", index_path)
}

fn build_index(dir: &Path, wiki_root: &Path) -> SpaceIndexManager {
    let mut mgr = make_manager(dir);
    git::commit(dir, "index pages").unwrap();
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr.open(&schema(), None).unwrap();
    mgr
}

/// Open a fresh searcher from disk (for tests that mutate then search).
fn open_searcher(mgr: &SpaceIndexManager, is: &IndexSchema) -> Searcher {
    let search_dir = mgr.index_path().join("search-index");
    let dir = tantivy::directory::MmapDirectory::open(&search_dir).unwrap();
    let index = tantivy::Index::open(dir).unwrap();
    let reader = index.reader().unwrap();
    reader.searcher()
}

// ── rebuild ───────────────────────────────────────────────────────────────────

#[test]
fn rebuild_indexes_all_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));

    let mgr = build_index(dir.path(), &wiki_root);

    assert!(mgr.index_path().join("state.toml").exists());
    let state: toml::Value =
        toml::from_str(&fs::read_to_string(mgr.index_path().join("state.toml")).unwrap()).unwrap();
    assert_eq!(state["pages"].as_integer().unwrap(), 2);
}

#[test]
fn rebuild_report_fields() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/a.md", &concept_page("A", "body"));
    write_page(&wiki_root, "concepts/b.md", &concept_page("B", "body"));
    git::commit(dir.path(), "pages").unwrap();

    let mgr = make_manager(dir.path());
    let report = mgr
        .rebuild(&wiki_root, dir.path(), &schema(), &registry())
        .unwrap();

    assert_eq!(report.wiki, "test");
    assert_eq!(report.pages_indexed, 2);
    assert!(report.duration_ms < 10_000);
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn status_not_stale_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let status = mgr.status(dir.path()).unwrap();

    assert!(!status.stale);
    assert!(status.openable);
    assert!(status.queryable);
    assert_eq!(status.pages, 1);
}

#[test]
fn status_stale_after_new_commit() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);

    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));
    git::commit(dir.path(), "add bar").unwrap();

    let status = mgr.status(dir.path()).unwrap();
    assert!(status.stale);
}

#[test]
fn status_when_no_index() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());

    let mgr = SpaceIndexManager::new("test", dir.path().join("nonexistent"));
    let status = mgr.status(dir.path()).unwrap();

    assert!(status.stale);
    assert!(!status.openable);
    assert!(status.built.is_none());
}

// ── last_commit ───────────────────────────────────────────────────────────────

#[test]
fn last_commit_none_before_build() {
    let dir = tempfile::tempdir().unwrap();
    let mgr = make_manager(dir.path());
    assert!(mgr.last_commit().is_none());
}

#[test]
fn last_commit_some_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let commit = mgr.last_commit();

    assert!(commit.is_some());
    assert!(!commit.unwrap().is_empty());
}

// ── update ────────────────────────────────────────────────────────────────────

#[test]
fn update_adds_new_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let is = schema();
    let reg = registry();

    let mgr = make_manager(dir.path());
    mgr.rebuild(&wiki_root, dir.path(), &is, &reg).unwrap();

    write_page(
        &wiki_root,
        "concepts/new.md",
        &concept_page("NewPage", "new body"),
    );

    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.updated, 1);

    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "NewPage",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(results.iter().any(|r| r.title == "NewPage"));
}

#[test]
fn update_noop_when_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.updated, 0);
    assert_eq!(report.deleted, 0);
}

#[test]
fn update_deletes_removed_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/gone.md",
        &concept_page("Gone", "will be deleted"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(!results.is_empty());

    fs::remove_file(wiki_root.join("concepts/gone.md")).unwrap();
    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.deleted, 1);

    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(results.is_empty());
}

#[test]
fn update_modifies_existing_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/evolve.md",
        &concept_page("Evolve", "original body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    write_page(
        &wiki_root,
        "concepts/evolve.md",
        &concept_page("Evolve", "updated body with unicorn"),
    );
    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.updated, 1);

    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "unicorn",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(!results.is_empty());
}

// ── delete_by_type ────────────────────────────────────────────────────────────

#[test]
fn delete_by_type_removes_matching_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    write_page(
        &wiki_root,
        "skills/bar.md",
        "---\nname: \"Bar\"\ntype: skill\nstatus: active\n---\n\nskill body\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    mgr.delete_by_type(&is, "concept").unwrap();

    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "Foo",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(results.is_empty());

    let results = search::search(
        "Bar",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(!results.is_empty());
}

// ── open with recovery ────────────────────────────────────────────────────────

#[test]
fn open_succeeds_on_valid_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    assert!(mgr.searcher().is_ok());
}

#[test]
fn open_recovers_from_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mut mgr = make_manager(dir.path());
    git::commit(dir.path(), "pages").unwrap();
    let is = schema();
    let reg = registry();
    mgr.rebuild(&wiki_root, dir.path(), &is, &reg).unwrap();

    // Corrupt the index files
    let search_dir = mgr.index_path().join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    // open with recovery should rebuild and succeed
    let result = mgr.open(&is, Some((&wiki_root, dir.path(), &reg)));
    assert!(result.is_ok());
    assert!(mgr.searcher().is_ok());
}

#[test]
fn open_fails_without_recovery_on_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mut mgr = make_manager(dir.path());
    git::commit(dir.path(), "pages").unwrap();
    mgr.rebuild(&wiki_root, dir.path(), &schema(), &registry())
        .unwrap();

    // Corrupt the index files
    let search_dir = mgr.index_path().join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    let result = mgr.open(&schema(), None);
    assert!(result.is_err());
}

// ── alias resolution ──────────────────────────────────────────────────────────

fn skill_page(name: &str, description: &str, body: &str) -> String {
    format!(
        "---\nname: \"{name}\"\ndescription: \"{description}\"\nstatus: active\ntype: skill\ntags:\n  - workflow\n---\n\n{body}\n"
    )
}

fn skill_page_with_title(name: &str, title: &str, description: &str, body: &str) -> String {
    format!(
        "---\nname: \"{name}\"\ntitle: \"{title}\"\ndescription: \"{description}\"\nstatus: active\ntype: skill\ntags:\n  - workflow\n---\n\n{body}\n"
    )
}

#[test]
fn alias_name_indexed_as_title() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "skills/ingest.md",
        &skill_page("ingest", "Process source files", "skill body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let results = search::search(
        "ingest",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.title == "ingest"),
        "skill name should be searchable as title"
    );
}

#[test]
fn alias_description_indexed_as_summary() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "skills/ingest.md",
        &skill_page("ingest", "Process source files into wiki", "body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let results = search::search(
        "Process source files",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        !results.is_empty(),
        "skill description should be searchable as summary"
    );
}

#[test]
fn alias_canonical_wins_when_both_exist() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "skills/dual.md",
        &skill_page_with_title("aliased-name", "canonical-title", "desc", "body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let results = search::search(
        "canonical-title",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.title == "canonical-title"),
        "canonical title should win"
    );

    let results = search::search(
        "aliased-name",
        &search::SearchOptions {
            top_k: 10,
            ..Default::default()
        },
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    for r in &results {
        if r.slug == "skills/dual" {
            assert_eq!(
                r.title, "canonical-title",
                "canonical should win over alias"
            );
        }
    }
}

#[test]
fn alias_source_field_not_double_indexed() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "skills/single.md",
        &skill_page("my-skill", "A skill", "body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let result = search::list(
        &search::ListOptions {
            r#type: Some("skill".into()),
            ..Default::default()
        },
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.pages[0].title, "my-skill");
}

#[test]
fn non_aliased_type_indexes_normally() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &concept_page("Mixture of Experts", "MoE body"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let results = search::search(
        "Mixture of Experts",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(results.iter().any(|r| r.title == "Mixture of Experts"));
}

#[test]
fn unrecognized_field_indexed_as_body_text() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/custom.md", "---\ntitle: \"Custom\"\ntype: concept\nmy_custom_field: \"unicorn rainbow\"\n---\n\nBody.\n");

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let searcher = mgr.searcher().unwrap();

    let results = search::search(
        "unicorn rainbow",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/custom"),
        "unrecognized field should be searchable as body text"
    );
}

// ── schema hash staleness ─────────────────────────────────────────────────────

#[test]
fn status_stale_on_schema_hash_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    let status = mgr.status(dir.path()).unwrap();
    assert!(!status.stale);

    let concept_schema = schemas_dir.join("concept.json");
    let mut content = std::fs::read_to_string(&concept_schema).unwrap();
    content = content.replace(
        "\"x-wiki-types\"",
        "\"x-graph-edges\": {\"related\": {}}, \"x-wiki-types\"",
    );
    std::fs::write(&concept_schema, content).unwrap();

    let status = mgr.status(dir.path()).unwrap();
    assert!(status.stale);
}

#[test]
fn round_trip_rebuild_then_not_stale() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    let status = mgr.status(dir.path()).unwrap();
    assert!(!status.stale, "should not be stale right after rebuild");
}

// ── staleness_kind + partial rebuild ──────────────────────────────────────────

use llm_wiki::index_manager::StalenessKind;

#[test]
fn staleness_kind_current_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    let kind = mgr.staleness_kind(dir.path()).unwrap();
    assert_eq!(kind, StalenessKind::Current);
}

#[test]
fn staleness_kind_commit_changed() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));
    git::commit(dir.path(), "add bar").unwrap();

    let kind = mgr.staleness_kind(dir.path()).unwrap();
    assert_eq!(kind, StalenessKind::CommitChanged);
}

#[test]
fn staleness_kind_types_changed() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    // Modify concept schema
    let concept_schema = schemas_dir.join("concept.json");
    let mut content = std::fs::read_to_string(&concept_schema).unwrap();
    content = content.replace(
        "\"x-wiki-types\"",
        "\"x-graph-edges\": {\"related\": {}}, \"x-wiki-types\"",
    );
    std::fs::write(&concept_schema, content).unwrap();

    let kind = mgr.staleness_kind(dir.path()).unwrap();
    match kind {
        StalenessKind::TypesChanged(types) => {
            assert!(types.contains(&"concept".to_string()));
        }
        other => panic!("expected TypesChanged, got {:?}", other),
    }
}

#[test]
fn rebuild_types_reindexes_only_changed_type() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo", "concept body"),
    );
    write_page(
        &wiki_root,
        "skills/bar.md",
        "---\nname: \"Bar\"\ntype: skill\nstatus: active\n---\n\nskill body\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    // Verify both are searchable
    let searcher = mgr.searcher().unwrap();
    let results = search::search(
        "Foo",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(!results.is_empty());
    let results = search::search(
        "Bar",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(!results.is_empty());

    // Partial rebuild only "concept" type
    let report = mgr
        .rebuild_types(&["concept".to_string()], &wiki_root, dir.path(), &is, &reg)
        .unwrap();
    assert_eq!(report.pages_indexed, 1);

    // Both should still be searchable (skill untouched, concept re-indexed)
    let searcher = open_searcher(&mgr, &is);
    let results = search::search(
        "Foo",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        !results.is_empty(),
        "concept should still be searchable after partial rebuild"
    );
    let results = search::search(
        "Bar",
        &search::SearchOptions::default(),
        &searcher,
        "test",
        &is,
    )
    .unwrap();
    assert!(
        !results.is_empty(),
        "skill should be untouched by partial rebuild"
    );
}

#[test]
fn staleness_kind_detects_type_modification() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);

    // No changes yet
    let kind = mgr.staleness_kind(dir.path()).unwrap();
    assert_eq!(kind, StalenessKind::Current);

    // Modify concept schema
    let concept_schema = schemas_dir.join("concept.json");
    let mut content = std::fs::read_to_string(&concept_schema).unwrap();
    content = content.replace(
        "\"x-wiki-types\"",
        "\"x-graph-edges\": {\"related\": {}}, \"x-wiki-types\"",
    );
    std::fs::write(&concept_schema, content).unwrap();

    let kind = mgr.staleness_kind(dir.path()).unwrap();
    match kind {
        StalenessKind::TypesChanged(types) => {
            assert!(types.contains(&"concept".to_string()));
        }
        other => panic!("expected TypesChanged, got {other:?}"),
    }
}
