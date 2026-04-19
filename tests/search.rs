use std::fs;
use std::path::Path;

use llm_wiki::git;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::indexing;
use llm_wiki::search::*;
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

fn paper_page(title: &str, body: &str) -> String {
    format!(
        "---\ntitle: \"{title}\"\nsummary: \"A paper\"\nstatus: active\ntype: paper\n---\n\n{body}\n"
    )
}

fn draft_page(title: &str) -> String {
    format!("---\ntitle: \"{title}\"\nstatus: draft\ntype: concept\n---\n\nDraft content.\n")
}

fn section_page(title: &str) -> String {
    format!("---\ntitle: \"{title}\"\nstatus: active\ntype: section\n---\n\n")
}

fn build_index(dir: &Path, wiki_root: &Path) -> std::path::PathBuf {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    indexing::rebuild_index(wiki_root, &index_path, "test", dir, &schema(), &registry()).unwrap();
    index_path
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn search_returns_ranked_results() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &concept_page("Mixture of Experts", "MoE scaling"),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &paper_page("Switch Transformer", "MoE layers"),
    );
    write_page(
        &wiki_root,
        "concepts/attention.md",
        &concept_page("Attention", "Self-attention"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "MoE scaling",
        &SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();

    assert!(!results.is_empty());
    for w in results.windows(2) {
        assert!(w[0].score >= w[1].score);
    }
}

#[test]
fn search_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/moe.md",
        &concept_page("MoE", "MoE scaling"),
    );
    write_page(
        &wiki_root,
        "sources/switch.md",
        &paper_page("Switch", "MoE scaling paper"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = SearchOptions {
        r#type: Some("paper".into()),
        ..Default::default()
    };
    let results = search("MoE scaling", &opts, &index_path, "test", &is, None).unwrap();

    assert!(!results.is_empty());
    for r in &results {
        assert_eq!(r.slug, "sources/switch");
    }
}

#[test]
fn search_excludes_sections_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/index.md", &section_page("Concepts"));
    write_page(
        &wiki_root,
        "concepts/foo.md",
        &concept_page("Foo Concepts", "about concepts"),
    );

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "Concepts",
        &SearchOptions::default(),
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();

    for r in &results {
        assert_ne!(r.slug, "concepts");
    }
}

#[test]
fn search_no_excerpt() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = SearchOptions {
        no_excerpt: true,
        ..Default::default()
    };
    let results = search("Foo", &opts, &index_path, "test", &is, None).unwrap();

    assert!(!results.is_empty());
    assert!(results[0].excerpt.is_none());
}

// ── list ──────────────────────────────────────────────────────────────────────

#[test]
fn list_returns_sorted_by_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/zebra.md", &concept_page("Zebra", "z"));
    write_page(&wiki_root, "concepts/alpha.md", &concept_page("Alpha", "a"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let result = list(&ListOptions::default(), &index_path, "test", &is, None).unwrap();

    assert_eq!(result.total, 2);
    assert_eq!(result.pages[0].slug, "concepts/alpha");
    assert_eq!(result.pages[1].slug, "concepts/zebra");
}

#[test]
fn list_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "f"));
    write_page(&wiki_root, "sources/bar.md", &paper_page("Bar", "b"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = ListOptions {
        r#type: Some("concept".into()),
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", &is, None).unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.pages[0].r#type, "concept");
}

#[test]
fn list_status_filter() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/active.md",
        &concept_page("Active", "a"),
    );
    write_page(&wiki_root, "concepts/wip.md", &draft_page("WIP"));

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = ListOptions {
        status: Some("draft".into()),
        ..Default::default()
    };
    let result = list(&opts, &index_path, "test", &is, None).unwrap();

    assert_eq!(result.total, 1);
    assert_eq!(result.pages[0].status, "draft");
}

#[test]
fn list_pagination() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    for i in 0..5 {
        write_page(
            &wiki_root,
            &format!("concepts/page-{i:02}.md"),
            &concept_page(&format!("Page {i}"), "body"),
        );
    }

    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let result = list(
        &ListOptions {
            page: 1,
            page_size: 2,
            ..Default::default()
        },
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert_eq!(result.total, 5);
    assert_eq!(result.pages.len(), 2);

    let result = list(
        &ListOptions {
            page: 3,
            page_size: 2,
            ..Default::default()
        },
        &index_path,
        "test",
        &is,
        None,
    )
    .unwrap();
    assert_eq!(result.pages.len(), 1);
}

// ── search_all ────────────────────────────────────────────────────────────────

#[test]
fn search_all_merges_wikis() {
    let dir_a = tempfile::tempdir().unwrap();
    let wiki_a = setup_repo(dir_a.path());
    write_page(&wiki_a, "concepts/moe.md", &concept_page("MoE", "MoE body"));
    let idx_a = build_index(dir_a.path(), &wiki_a);

    let dir_b = tempfile::tempdir().unwrap();
    let wiki_b = setup_repo(dir_b.path());
    write_page(
        &wiki_b,
        "sources/switch.md",
        &paper_page("Switch", "MoE paper"),
    );
    let idx_b = build_index(dir_b.path(), &wiki_b);

    let is = schema();
    let wikis = vec![("a".into(), idx_a), ("b".into(), idx_b)];
    let results = search_all("MoE", &SearchOptions::default(), &wikis, &is).unwrap();

    assert!(results.len() >= 2);
    assert!(results.iter().any(|r| r.uri.starts_with("wiki://a/")));
    assert!(results.iter().any(|r| r.uri.starts_with("wiki://b/")));
}

#[test]
fn search_all_respects_top_k() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    for i in 0..5 {
        write_page(
            &wiki_root,
            &format!("concepts/topic-{i}.md"),
            &concept_page(&format!("Topic {i}"), "shared keyword body"),
        );
    }
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let opts = SearchOptions {
        top_k: 2,
        ..Default::default()
    };
    let wikis = vec![("test".into(), index_path)];
    let results = search_all("keyword", &opts, &wikis, &is).unwrap();
    assert!(results.len() <= 2);
}

#[test]
fn search_all_skips_missing_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let index_path = build_index(dir.path(), &wiki_root);
    let is = schema();

    let wikis = vec![
        ("good".into(), index_path),
        ("bad".into(), dir.path().join("nonexistent")),
    ];
    let results = search_all("Foo", &SearchOptions::default(), &wikis, &is).unwrap();
    assert!(!results.is_empty());
}
