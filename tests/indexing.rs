use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::indexing::*;
use llm_wiki::search;

fn schema() -> IndexSchema {
    IndexSchema::build("en_stem")
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

fn build_index(dir: &Path, wiki_root: &Path) -> std::path::PathBuf {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    rebuild_index(wiki_root, &index_path, "test", dir, &schema()).unwrap();
    index_path
}

// ── rebuild_index ─────────────────────────────────────────────────────────────

#[test]
fn rebuild_indexes_all_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));

    let index_path = build_index(dir.path(), &wiki_root);

    assert!(index_path.join("state.toml").exists());
    let state: toml::Value =
        toml::from_str(&fs::read_to_string(index_path.join("state.toml")).unwrap()).unwrap();
    assert_eq!(state["pages"].as_integer().unwrap(), 2);
}

// ── index_status ──────────────────────────────────────────────────────────────

#[test]
fn status_not_stale_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    let status = index_status("test", &index_path, dir.path()).unwrap();
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
    let index_path = build_index(dir.path(), &wiki_root);

    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));
    git::commit(dir.path(), "add bar").unwrap();

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
}

#[test]
fn status_when_no_index() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());
    let index_path = dir.path().join("nonexistent");

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
    assert!(!status.openable);
    assert!(status.built.is_none());
}

#[test]
fn status_stale_on_schema_version_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    let state_path = index_path.join("state.toml");
    let content = fs::read_to_string(&state_path).unwrap();
    let tampered = content.replace("schema_version = 2", "schema_version = 999");
    fs::write(&state_path, tampered).unwrap();

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
}

// ── update_index ──────────────────────────────────────────────────────────────

#[test]
fn update_adds_new_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let index_path = dir.path().join("index-store");
    let is = schema();

    rebuild_index(&wiki_root, &index_path, "test", dir.path(), &is).unwrap();

    write_page(
        &wiki_root,
        "concepts/new.md",
        &concept_page("NewPage", "new body"),
    );

    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test").unwrap();
    assert_eq!(report.updated, 1);

    let results = search::search(
        "NewPage",
        &search::SearchOptions::default(),
        &index_path,
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
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test").unwrap();
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
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(!results.is_empty());

    fs::remove_file(wiki_root.join("concepts/gone.md")).unwrap();
    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test").unwrap();
    assert_eq!(report.deleted, 1);

    let results = search::search(
        "Gone",
        &search::SearchOptions::default(),
        &index_path,
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
    write_page(
        &wiki_root,
        "concepts/evolve.md",
        &concept_page("Evolve", "original body"),
    );
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    write_page(
        &wiki_root,
        "concepts/evolve.md",
        &concept_page("Evolve", "updated body with unicorn"),
    );
    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test").unwrap();
    assert_eq!(report.updated, 1);

    let results = search::search(
        "unicorn",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(!results.is_empty());
}


// ── recovery ──────────────────────────────────────────────────────────────────

#[test]
fn recovers_from_corrupt_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let search_dir = index_path.join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    let recovery = RecoveryContext {
        wiki_root: &wiki_root,
        repo_root: dir.path(),
    };
    let results = search::search(
        "Foo",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        Some(&recovery),
    )
    .unwrap();
    assert!(!results.is_empty());
}
