use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::search::*;

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
        "---\ntitle: \"{title}\"\nsummary: \"A concept\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: concept\ntags:\n  - scaling\n---\n\n{body}\n"
    )
}

fn paper_page(title: &str, body: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"A paper\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: paper\n---\n\n{body}\n"
    )
}

fn draft_page(title: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"Draft\"\nstatus: draft\nlast_updated: \"2025-01-01\"\ntype: concept\n---\n\nDraft content.\n"
    )
}

fn section_page(title: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"Section\"\nstatus: active\nlast_updated: \"2025-01-01\"\ntype: section\n---\n\n"
    )
}

fn build_index(dir: &Path, wiki_root: &Path) -> std::path::PathBuf {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    rebuild_index(wiki_root, &index_path, "test", dir).unwrap();
    index_path
}

// ── rebuild_index ─────────────────────────────────────────────────────────────

#[test]
fn rebuild_index_indexes_all_pages_and_writes_state_toml() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo", "Foo body"),
    );
    write_page(
        &wiki_root,
        "concepts/bar.md",
        &concept_page("Bar", "Bar body"),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    assert!(index_path.join("state.toml").exists());
    assert!(index_path.join("search-index").exists());

    let state: toml::Value =
        toml::from_str(&fs::read_to_string(index_path.join("state.toml")).unwrap()).unwrap();
    assert_eq!(state["pages"].as_integer().unwrap(), 2);
}

#[test]
fn rebuild_index_stores_commit_hash_in_state_toml() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let index_path = build_index(dir.path(), &wiki_root);

    let state: toml::Value =
        toml::from_str(&fs::read_to_string(index_path.join("state.toml")).unwrap()).unwrap();
    let head = git::current_head(dir.path()).unwrap();
    assert_eq!(state["commit"].as_str().unwrap(), head);
}

// ── index_status ──────────────────────────────────────────────────────────────

#[test]
fn index_status_returns_stale_false_immediately_after_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let index_path = build_index(dir.path(), &wiki_root);

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(!status.stale);
    assert!(status.built.is_some());
    assert_eq!(status.pages, 1);
}

#[test]
fn index_status_returns_stale_true_after_a_new_commit() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let index_path = build_index(dir.path(), &wiki_root);

    // Make a new commit after rebuild
    write_page(&wiki_root, "concepts/bar.md", &concept_page("Bar", "body"));
    git::commit(dir.path(), "add bar").unwrap();

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
}

#[test]
fn index_status_returns_built_none_when_index_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    setup_repo(dir.path());
    let index_path = dir.path().join("nonexistent-index");

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.built.is_none());
    assert!(status.stale);
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn search_returns_results_ranked_by_bm25_score() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &concept_page(
            "Mixture of Experts",
            "MoE routes tokens to sparse expert subnetworks for scaling efficiency.",
        ),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &paper_page(
            "Switch Transformer",
            "Switch Transformer uses sparse MoE layers to scale to trillion parameters.",
        ),
    );
    write_page(
        &wiki_root,
        "concepts/attention.md",
        &concept_page("Attention", "Self-attention mechanism in transformers."),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = SearchOptions::default();
    let results = search("MoE scaling", &opts, &index_path, "test", None).unwrap();

    assert!(!results.is_empty());
    // Results should be sorted by score descending (BM25)
    for w in results.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
    // Top results should mention MoE
    assert!(
        results[0].slug.contains("moe") || results[0].slug.contains("switch"),
        "top result should be MoE-related, got: {}",
        results[0].slug
    );
}

#[test]
fn search_with_no_excerpt_returns_pageref_with_excerpt_none() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo", "Foo body text"),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = SearchOptions {
        no_excerpt: true,
        ..Default::default()
    };
    let results = search("Foo", &opts, &index_path, "test", None).unwrap();
    assert!(!results.is_empty());
    assert!(results[0].excerpt.is_none());
}

#[test]
fn search_with_include_sections_false_excludes_section_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/index.md", &section_page("Concepts"));
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo Concepts", "Concepts about foo."),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = SearchOptions {
        include_sections: false,
        ..Default::default()
    };
    let results = search("Concepts", &opts, &index_path, "test", None).unwrap();
    for r in &results {
        assert_ne!(r.slug, "concepts", "section page should be excluded");
    }
}

#[test]
fn search_with_include_sections_true_includes_section_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/index.md", &section_page("Concepts"));
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo Concepts", "Concepts about foo."),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = SearchOptions {
        include_sections: true,
        ..Default::default()
    };
    let results = search("Concepts", &opts, &index_path, "test", None).unwrap();
    let slugs: Vec<&str> = results.iter().map(|r| r.slug.as_str()).collect();
    assert!(
        slugs.contains(&"concepts"),
        "section page should be included, got: {slugs:?}"
    );
}

#[test]
fn search_type_filter_returns_only_matching_type() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &concept_page("Mixture of Experts", "MoE scaling body."),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &paper_page("Switch Transformer", "MoE scaling paper body."),
    );

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = SearchOptions {
        r#type: Some("paper".into()),
        ..Default::default()
    };
    let results = search("MoE scaling", &opts, &index_path, "test", None).unwrap();
    assert!(!results.is_empty());
    for r in &results {
        // All results should be from the paper page
        assert_eq!(r.slug, "sources/switch");
    }
}

// ── list ──────────────────────────────────────────────────────────────────────

#[test]
fn list_returns_all_pages_ordered_by_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/zebra.md", &concept_page("Zebra", "z"));
    write_page(&wiki_root, "concepts/alpha.md", &concept_page("Alpha", "a"));
    write_page(&wiki_root, "sources/beta.md", &paper_page("Beta", "b"));

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = ListOptions::default();
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 3);
    let slugs: Vec<&str> = result.pages.iter().map(|p| p.slug.as_str()).collect();
    assert_eq!(
        slugs,
        vec!["concepts/alpha", "concepts/zebra", "sources/beta"]
    );
}

#[test]
fn list_with_type_concept_returns_only_concept_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "f"));
    write_page(&wiki_root, "sources/bar.md", &paper_page("Bar", "b"));

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = ListOptions {
        r#type: Some("concept".into()),
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.pages[0].r#type, "concept");
    assert_eq!(result.pages[0].slug, "concepts/foo");
}

#[test]
fn list_with_status_draft_returns_only_draft_pages() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/active.md",
        &concept_page("Active", "a"),
    );
    write_page(&wiki_root, "concepts/wip.md", &draft_page("WIP"));

    let index_path = build_index(dir.path(), &wiki_root);

    let opts = ListOptions {
        status: Some("draft".into()),
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.pages[0].status, "draft");
}

#[test]
fn list_pagination_returns_correct_page_and_total() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    for i in 0..5 {
        write_page(
            &wiki_root,
            &format!("concepts/page-{i:02}.md"),
            &concept_page(&format!("Page {i}"), &format!("body {i}")),
        );
    }

    let index_path = build_index(dir.path(), &wiki_root);

    // Page 1, size 2
    let opts = ListOptions {
        page: 1,
        page_size: 2,
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 5);
    assert_eq!(result.page, 1);
    assert_eq!(result.page_size, 2);
    assert_eq!(result.pages.len(), 2);

    // Page 3, size 2 — should have 1 item
    let opts = ListOptions {
        page: 3,
        page_size: 2,
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 5);
    assert_eq!(result.page, 3);
    assert_eq!(result.pages.len(), 1);

    // Page 4, size 2 — beyond range
    let opts = ListOptions {
        page: 4,
        page_size: 2,
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", None).unwrap();
    assert_eq!(result.total, 5);
    assert!(result.pages.is_empty());
}

// ── search_all ────────────────────────────────────────────────────────────────

#[test]
fn search_all_merges_results_from_multiple_wikis() {
    // Wiki A
    let dir_a = tempfile::tempdir().unwrap();
    let wiki_root_a = setup_repo(dir_a.path());
    write_page(
        &wiki_root_a,
        "concepts/moe.md",
        &concept_page(
            "Mixture of Experts",
            "MoE routes tokens to sparse expert subnetworks.",
        ),
    );
    let index_a = build_index(dir_a.path(), &wiki_root_a);

    // Wiki B
    let dir_b = tempfile::tempdir().unwrap();
    let wiki_root_b = setup_repo(dir_b.path());
    write_page(
        &wiki_root_b,
        "sources/switch.md",
        &paper_page(
            "Switch Transformer",
            "Switch Transformer uses sparse MoE layers.",
        ),
    );
    let index_b = build_index(dir_b.path(), &wiki_root_b);

    let wikis = vec![
        ("alpha".to_string(), index_a),
        ("beta".to_string(), index_b),
    ];
    let opts = SearchOptions::default();
    let results = search_all("MoE", &opts, &wikis).unwrap();

    assert!(
        results.len() >= 2,
        "expected results from both wikis, got {}",
        results.len()
    );

    let uris: Vec<&str> = results.iter().map(|r| r.uri.as_str()).collect();
    assert!(
        uris.iter().any(|u| u.starts_with("wiki://alpha/")),
        "missing alpha wiki result"
    );
    assert!(
        uris.iter().any(|u| u.starts_with("wiki://beta/")),
        "missing beta wiki result"
    );
}

#[test]
fn search_all_sorts_by_score_descending() {
    let dir_a = tempfile::tempdir().unwrap();
    let wiki_root_a = setup_repo(dir_a.path());
    write_page(
        &wiki_root_a,
        "concepts/foo.md",
        &concept_page("Foo", "Foo content about scaling."),
    );
    let index_a = build_index(dir_a.path(), &wiki_root_a);

    let dir_b = tempfile::tempdir().unwrap();
    let wiki_root_b = setup_repo(dir_b.path());
    write_page(
        &wiki_root_b,
        "concepts/bar.md",
        &concept_page("Bar", "Bar content about scaling efficiency."),
    );
    let index_b = build_index(dir_b.path(), &wiki_root_b);

    let wikis = vec![
        ("alpha".to_string(), index_a),
        ("beta".to_string(), index_b),
    ];
    let opts = SearchOptions::default();
    let results = search_all("scaling", &opts, &wikis).unwrap();

    for w in results.windows(2) {
        assert!(
            w[0].score >= w[1].score,
            "results not sorted by score: {} < {}",
            w[0].score,
            w[1].score
        );
    }
}

#[test]
fn search_all_skips_wikis_without_index() {
    let dir_a = tempfile::tempdir().unwrap();
    let wiki_root_a = setup_repo(dir_a.path());
    write_page(
        &wiki_root_a,
        "concepts/foo.md",
        &concept_page("Foo", "Foo body text."),
    );
    let index_a = build_index(dir_a.path(), &wiki_root_a);

    // Wiki B has no index
    let missing_index = tempfile::tempdir().unwrap().path().join("nonexistent");

    let wikis = vec![
        ("alpha".to_string(), index_a),
        ("beta".to_string(), missing_index),
    ];
    let opts = SearchOptions::default();
    let results = search_all("Foo", &opts, &wikis).unwrap();

    assert!(!results.is_empty());
    assert!(results.iter().all(|r| r.uri.starts_with("wiki://alpha/")));
}

#[test]
fn search_all_respects_top_k() {
    let dir_a = tempfile::tempdir().unwrap();
    let wiki_root_a = setup_repo(dir_a.path());
    for i in 0..5 {
        write_page(
            &wiki_root_a,
            &format!("concepts/page-{i}.md"),
            &concept_page(
                &format!("Scaling Page {i}"),
                &format!("scaling content {i}"),
            ),
        );
    }
    let index_a = build_index(dir_a.path(), &wiki_root_a);

    let wikis = vec![("alpha".to_string(), index_a)];
    let opts = SearchOptions {
        top_k: 2,
        ..Default::default()
    };
    let results = search_all("scaling", &opts, &wikis).unwrap();
    assert!(
        results.len() <= 2,
        "expected at most 2 results, got {}",
        results.len()
    );
}

#[test]
fn index_status_returns_stale_on_malformed_state_toml() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    let index_path = dir.path().join("index-store");

    // Build a valid index first
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let _ = build_index(dir.path(), &wiki_root);

    // Corrupt state.toml
    fs::write(index_path.join("state.toml"), "this is not valid toml {{{").unwrap();

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale);
    assert!(status.built.is_none());
    assert_eq!(status.pages, 0);
}

#[test]
fn search_recovers_from_corrupt_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo", "Foo body text"),
    );
    let index_path = build_index(dir.path(), &wiki_root);

    // Corrupt the index by overwriting ALL files
    let search_dir = index_path.join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    // Search with recovery should rebuild and succeed
    let recovery = RecoveryContext {
        wiki_root: &wiki_root,
        repo_root: dir.path(),
    };
    let opts = SearchOptions::default();
    let results = search("Foo", &opts, &index_path, "test", Some(&recovery)).unwrap();
    assert!(!results.is_empty(), "should find results after recovery");
}

#[test]
fn search_errors_on_corrupt_index_without_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo", "Foo body text"),
    );
    let index_path = build_index(dir.path(), &wiki_root);

    // Corrupt the index by overwriting ALL files
    let search_dir = index_path.join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    // Search without recovery should error
    let opts = SearchOptions::default();
    let result = search("Foo", &opts, &index_path, "test", None);
    assert!(result.is_err());
}

#[test]
fn index_status_returns_stale_on_schema_version_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    // Verify fresh index is not stale
    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(!status.stale);

    // Manually set schema_version to 0 (simulating pre-versioning or old version)
    let state_path = index_path.join("state.toml");
    let content = fs::read_to_string(&state_path).unwrap();
    let updated = content.replace("schema_version = 1", "schema_version = 0");
    fs::write(&state_path, updated).unwrap();

    let status = index_status("test", &index_path, dir.path()).unwrap();
    assert!(status.stale, "schema version mismatch should be stale");
}

// ── index_check ───────────────────────────────────────────────────────────────

#[test]
fn index_check_reports_healthy_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    let report = index_check("test", &index_path, dir.path());
    assert!(report.openable);
    assert!(report.queryable);
    assert!(report.schema_current);
    assert!(report.state_valid);
    assert!(!report.stale);
    assert_eq!(report.schema_version, Some(current_schema_version()));
}

#[test]
fn index_check_reports_corrupt_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);

    // Corrupt all index files
    let search_dir = index_path.join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    let report = index_check("test", &index_path, dir.path());
    assert!(!report.openable);
    assert!(!report.queryable);
    // state.toml is still valid (we only corrupted the index files)
    assert!(report.state_valid);
}
