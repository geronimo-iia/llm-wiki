use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::indexing::*;
use llm_wiki::search;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn schema() -> IndexSchema {
    IndexSchema::build("en_stem")
}

fn registry() -> SpaceTypeRegistry {
    SpaceTypeRegistry::from_embedded()
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
    rebuild_index(wiki_root, &index_path, "test", dir, &schema(), &registry()).unwrap();
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
fn status_stale_on_schema_hash_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());

    // Copy embedded schemas to disk so we can modify them
    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    // Write minimal wiki.toml
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    // Verify not stale initially
    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(!status.stale);

    // Modify a schema file on disk (add x-graph-edges)
    let concept_schema = schemas_dir.join("concept.json");
    let mut content = std::fs::read_to_string(&concept_schema).unwrap();
    content = content.replace(
        "\"x-wiki-types\"",
        "\"x-graph-edges\": {\"related\": {}}, \"x-wiki-types\"",
    );
    std::fs::write(&concept_schema, content).unwrap();

    // Now index_status should report stale
    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
}

// ── compute_disk_hashes tests ─────────────────────────────────────────────────

use llm_wiki::type_registry::compute_disk_hashes;

#[test]
fn disk_hashes_change_on_schema_file_modification() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    let (hash1, _) = compute_disk_hashes(dir.path()).unwrap();

    // Add a property (no alias change)
    let concept_schema = schemas_dir.join("concept.json");
    let mut content = std::fs::read_to_string(&concept_schema).unwrap();
    content = content.replace(
        "\"x-wiki-types\"",
        "\"x-graph-edges\": {\"related\": {}}, \"x-wiki-types\"",
    );
    std::fs::write(&concept_schema, content).unwrap();

    let (hash2, _) = compute_disk_hashes(dir.path()).unwrap();
    assert_ne!(hash1, hash2, "hash should change when schema file is modified");
}

#[test]
fn disk_hashes_identical_schemas_same_hash() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();

    for dir in [dir1.path(), dir2.path()] {
        setup_repo(dir);
        let schemas_dir = dir.join("schemas");
        std::fs::create_dir_all(&schemas_dir).unwrap();
        for (filename, content) in llm_wiki::default_schemas::default_schemas() {
            std::fs::write(schemas_dir.join(filename), content).unwrap();
        }
        std::fs::write(dir.join("wiki.toml"), "[types]\n").unwrap();
    }

    let (hash1, types1) = compute_disk_hashes(dir1.path()).unwrap();
    let (hash2, types2) = compute_disk_hashes(dir2.path()).unwrap();
    assert_eq!(hash1, hash2);
    assert_eq!(types1, types2);
}

#[test]
fn disk_hashes_embedded_fallback_stable() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());
    // No schemas/ dir — uses embedded fallback
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    let (hash1, _) = compute_disk_hashes(dir.path()).unwrap();
    let (hash2, _) = compute_disk_hashes(dir.path()).unwrap();
    assert_eq!(hash1, hash2, "embedded fallback hash should be stable");
}

#[test]
fn disk_hashes_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());

    let schemas_dir = dir.path().join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        std::fs::write(schemas_dir.join(filename), content).unwrap();
    }
    std::fs::write(dir.path().join("wiki.toml"), "[types]\n").unwrap();

    let (hash1, types1) = compute_disk_hashes(dir.path()).unwrap();
    let (hash2, types2) = compute_disk_hashes(dir.path()).unwrap();
    assert_eq!(hash1, hash2);
    assert_eq!(types1, types2);
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
    let index_path = build_index(dir.path(), &wiki_root);

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(!status.stale, "should not be stale right after rebuild");
}

// ── update_index ──────────────────────────────────────────────────────────────

#[test]
fn update_adds_new_page() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let index_path = dir.path().join("index-store");
    let is = schema();

    rebuild_index(&wiki_root, &index_path, "test", dir.path(), &is, &registry()).unwrap();

    write_page(
        &wiki_root,
        "concepts/new.md",
        &concept_page("NewPage", "new body"),
    );

    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test", &registry()).unwrap();
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

    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test", &registry()).unwrap();
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
    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test", &registry()).unwrap();
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
    let report = update_index(&wiki_root, &index_path, dir.path(), None, &is, "test", &registry()).unwrap();
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

    let reg = registry();
    let recovery = RecoveryContext {
        wiki_root: &wiki_root,
        repo_root: dir.path(),
        registry: &reg,
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

// ── alias resolution edge cases ───────────────────────────────────────────────

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

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // Search by the aliased value — should find it via "title" field
    let results = search::search(
        "ingest",
        &search::SearchOptions::default(),
        &index_path,
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

#[test]
fn alias_description_indexed_as_summary() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "skills/ingest.md",
        &skill_page("ingest", "Process source files into wiki", "body"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // Search by description content — should match via "summary" field
    let results = search::search(
        "Process source files",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
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
    // Page has both "name" (aliased) and "title" (canonical)
    write_page(
        &wiki_root,
        "skills/dual.md",
        &skill_page_with_title("aliased-name", "canonical-title", "desc", "body"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // Search for the canonical value — should find it
    let results = search::search(
        "canonical-title",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.title == "canonical-title"),
        "canonical title should win"
    );

    // Search for the aliased value — should NOT be indexed as title
    let results = search::search(
        "aliased-name",
        &search::SearchOptions {
            top_k: 10,
            ..Default::default()
        },
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    // The aliased name might still match via body text (unrecognized field),
    // but the title field should be "canonical-title", not "aliased-name"
    for r in &results {
        if r.slug == "skills/dual" {
            assert_eq!(r.title, "canonical-title", "canonical should win over alias");
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

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // List should show the skill with title = "my-skill" (from alias)
    let result = search::list(
        &search::ListOptions {
            r#type: Some("skill".into()),
            ..Default::default()
        },
        &index_path,
        "test",
        &is,
        None,
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

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let results = search::search(
        "Mixture of Experts",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(results.iter().any(|r| r.title == "Mixture of Experts"));
}

#[test]
fn unrecognized_field_indexed_as_body_text() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/custom.md",
        "---\ntitle: \"Custom\"\ntype: concept\nmy_custom_field: \"unicorn rainbow\"\n---\n\nBody.\n",
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    // The custom field value should be searchable via body text
    let results = search::search(
        "unicorn rainbow",
        &search::SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert!(
        results.iter().any(|r| r.slug == "concepts/custom"),
        "unrecognized field should be searchable as body text"
    );
}
