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
    let results = search("MoE scaling", &opts, &index_path, "test").unwrap();

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
    let results = search("Foo", &opts, &index_path, "test").unwrap();
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
    let results = search("Concepts", &opts, &index_path, "test").unwrap();
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
    let results = search("Concepts", &opts, &index_path, "test").unwrap();
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
    let results = search("MoE scaling", &opts, &index_path, "test").unwrap();
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
    let result = list(&opts, &index_path, "test").unwrap();
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
    let result = list(&opts, &index_path, "test").unwrap();
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
    let result = list(&opts, &index_path, "test").unwrap();
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
    let result = list(&opts, &index_path, "test").unwrap();
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
    let result = list(&opts, &index_path, "test").unwrap();
    assert_eq!(result.total, 5);
    assert_eq!(result.page, 3);
    assert_eq!(result.pages.len(), 1);

    // Page 4, size 2 — beyond range
    let opts = ListOptions {
        page: 4,
        page_size: 2,
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test").unwrap();
    assert_eq!(result.total, 5);
    assert!(result.pages.is_empty());
}
