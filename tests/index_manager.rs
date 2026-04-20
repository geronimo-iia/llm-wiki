use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_manager::{RecoveryContext, SpaceIndexManager};
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::search;
use llm_wiki::space_builder;
use llm_wiki::type_registry::SpaceTypeRegistry;

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
    let mgr = make_manager(dir);
    git::commit(dir, "index pages").unwrap();
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr
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
    let report = mgr.rebuild(&wiki_root, dir.path(), &schema(), &registry()).unwrap();

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

    write_page(&wiki_root, "concepts/new.md", &concept_page("NewPage", "new body"));

    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.updated, 1);

    let results = search::search(
        "NewPage",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
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
    write_page(&wiki_root, "concepts/gone.md", &concept_page("Gone", "will be deleted"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    // Verify it's searchable
    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(!results.is_empty());

    // Delete and update
    fs::remove_file(wiki_root.join("concepts/gone.md")).unwrap();
    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.deleted, 1);

    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(results.is_empty());
}

#[test]
fn update_modifies_existing_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/evolve.md", &concept_page("Evolve", "original body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    write_page(&wiki_root, "concepts/evolve.md", &concept_page("Evolve", "updated body with unicorn"));
    let report = mgr.update(&wiki_root, dir.path(), None, &is, &reg).unwrap();
    assert_eq!(report.updated, 1);

    let results = search::search(
        "unicorn",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
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

    // Concept should be gone
    let results = search::search(
        "Foo",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(results.is_empty());

    // Skill should remain
    let results = search::search(
        "Bar",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(!results.is_empty());
}

// ── open_index with recovery ──────────────────────────────────────────────────

#[test]
fn open_index_succeeds_on_valid_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    let index = mgr.open_index(&is, None);
    assert!(index.is_ok());
}

#[test]
fn open_index_recovers_from_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let reg = registry();

    // Corrupt the index files
    let search_dir = mgr.index_path().join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    let recovery = RecoveryContext {
        wiki_root: &wiki_root,
        repo_root: dir.path(),
        registry: &reg,
    };

    let index = mgr.open_index(&is, Some(&recovery));
    assert!(index.is_ok());
}

#[test]
fn open_index_fails_without_recovery_on_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    // Corrupt the index files
    let search_dir = mgr.index_path().join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    let index = mgr.open_index(&is, None);
    assert!(index.is_err());
}

// ── alias resolution via SpaceIndexManager ────────────────────────────────────

fn skill_page(name: &str, description: &str, body: &str) -> String {
    format!(
        "---\nname: \"{name}\"\ndescription: \"{description}\"\nstatus: active\ntype: skill\ntags:\n  - workflow\n---\n\n{body}\n"
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

    let results = search::search(
        "ingest",
        &search::SearchOptions::default(),
        mgr.index_path(),
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.title == "ingest"),
        "skill name should be searchable as title"
    );
}
