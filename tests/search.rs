use std::fs;
use std::path::Path;

use llm_wiki::config::SearchConfig;
use llm_wiki::git;
use llm_wiki::index_manager::SpaceIndexManager;
use llm_wiki::index_schema::IndexSchema;
use llm_wiki::search::*;
use llm_wiki::space_builder;
use llm_wiki::type_registry::SpaceTypeRegistry;

fn schema() -> IndexSchema {
    let (_registry, schema) = space_builder::build_space_from_embedded("en_stem");
    schema
}

fn registry() -> SpaceTypeRegistry {
    let (registry, _schema) = space_builder::build_space_from_embedded("en_stem");
    registry
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

fn build_index(dir: &Path, wiki_root: &Path) -> SpaceIndexManager {
    let index_path = dir.join("index-store");
    git::commit(dir, "index pages").unwrap();
    let mgr = SpaceIndexManager::new("test", &index_path);
    mgr.rebuild(wiki_root, dir, &schema(), &registry()).unwrap();
    mgr.open(&schema(), None).unwrap();
    mgr
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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "MoE scaling",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert!(!results.results.is_empty());
    for w in results.results.windows(2) {
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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = SearchOptions {
        r#type: Some("paper".into()),
        ..Default::default()
    };
    let results = search("MoE scaling", &opts, &mgr.searcher().unwrap(), "test", &is).unwrap();

    assert!(!results.results.is_empty());
    for r in &results.results {
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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "Concepts",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    for r in &results.results {
        assert_ne!(r.slug, "concepts");
    }
}

#[test]
fn search_no_excerpt() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = SearchOptions {
        no_excerpt: true,
        ..Default::default()
    };
    let results = search("Foo", &opts, &mgr.searcher().unwrap(), "test", &is).unwrap();

    assert!(!results.results.is_empty());
    assert!(results.results[0].excerpt.is_none());
}

// ── list ──────────────────────────────────────────────────────────────────────

#[test]
fn list_returns_sorted_by_slug() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/zebra.md", &concept_page("Zebra", "z"));
    write_page(&wiki_root, "concepts/alpha.md", &concept_page("Alpha", "a"));

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let result = list(
        &ListOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = ListOptions {
        r#type: Some("concept".into()),
        ..Default::default()
    };
    let result = list(&opts, &mgr.searcher().unwrap(), "test", &is).unwrap();

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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let opts = ListOptions {
        status: Some("draft".into()),
        ..Default::default()
    };
    let result = list(&opts, &mgr.searcher().unwrap(), "test", &is).unwrap();

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

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    let result = list(
        &ListOptions {
            page: 1,
            page_size: 2,
            ..Default::default()
        },
        &mgr.searcher().unwrap(),
        "test",
        &is,
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
        &mgr.searcher().unwrap(),
        "test",
        &is,
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
    let mgr_a = build_index(dir_a.path(), &wiki_a);

    let dir_b = tempfile::tempdir().unwrap();
    let wiki_b = setup_repo(dir_b.path());
    write_page(
        &wiki_b,
        "sources/switch.md",
        &paper_page("Switch", "MoE paper"),
    );
    let mgr_b = build_index(dir_b.path(), &wiki_b);

    let is = schema();
    let wikis = vec![
        ("a".into(), mgr_a.searcher().unwrap(), &is),
        ("b".into(), mgr_b.searcher().unwrap(), &is),
    ];
    let results = search_all("MoE", &SearchOptions::default(), &wikis).unwrap();

    assert!(results.results.len() >= 2);
    assert!(
        results
            .results
            .iter()
            .any(|r| r.uri.starts_with("wiki://a/"))
    );
    assert!(
        results
            .results
            .iter()
            .any(|r| r.uri.starts_with("wiki://b/"))
    );
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
    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    let opts = SearchOptions {
        top_k: 2,
        ..Default::default()
    };
    let wikis = vec![("test".into(), mgr.searcher().unwrap(), &is)];
    let results = search_all("keyword", &opts, &wikis).unwrap();
    assert!(results.results.len() <= 2);
}

#[test]
fn search_all_skips_missing_index() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/foo.md", &concept_page("Foo", "body"));
    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();

    // search_all with only the good wiki — bad wiki can't produce a Searcher
    let wikis = vec![("good".into(), mgr.searcher().unwrap(), &is)];
    let results = search_all("Foo", &SearchOptions::default(), &wikis).unwrap();
    assert!(!results.results.is_empty());
}

// ── ranking: status multiplier ────────────────────────────────────────────────

fn status_page(slug_name: &str, status: &str) -> String {
    format!(
        "---\ntitle: \"{slug_name}\"\nstatus: {status}\ntype: concept\n---\n\nidentical ranking body text for testing\n"
    )
}

fn confidence_page(slug_name: &str, conf: f64) -> String {
    format!(
        "---\ntitle: \"{slug_name}\"\nstatus: active\ntype: concept\nconfidence: {conf}\n---\n\nidentical ranking body text for testing\n"
    )
}

#[test]
fn search_ranking_active_above_draft_above_archived() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/archived.md",
        &status_page("Archived", "archived"),
    );
    write_page(
        &wiki_root,
        "concepts/draft.md",
        &status_page("Draft", "draft"),
    );
    write_page(
        &wiki_root,
        "concepts/active.md",
        &status_page("Active", "active"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "identical ranking body",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert_eq!(results.results.len(), 3);
    let slugs: Vec<&str> = results.results.iter().map(|r| r.slug.as_str()).collect();
    let pos_active = slugs.iter().position(|&s| s == "concepts/active").unwrap();
    let pos_draft = slugs.iter().position(|&s| s == "concepts/draft").unwrap();
    let pos_archived = slugs
        .iter()
        .position(|&s| s == "concepts/archived")
        .unwrap();
    assert!(pos_active < pos_draft, "active should rank above draft");
    assert!(pos_draft < pos_archived, "draft should rank above archived");
}

#[test]
fn search_ranking_high_confidence_above_low() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(&wiki_root, "concepts/low.md", &confidence_page("Low", 0.2));
    write_page(
        &wiki_root,
        "concepts/high.md",
        &confidence_page("High", 0.9),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "identical ranking body",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert_eq!(results.results.len(), 2);
    assert_eq!(results.results[0].slug, "concepts/high");
    assert_eq!(results.results[1].slug, "concepts/low");
}

#[test]
fn search_ranking_archived_high_confidence_below_active_medium() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    // archived + 1.0 = 0.3 × 1.0 = 0.3 < active + 0.5 = 1.0 × 0.5 = 0.5
    write_page(
        &wiki_root,
        "concepts/archived-high.md",
        "---\ntitle: \"Archived High\"\nstatus: archived\ntype: concept\nconfidence: 1.0\n---\n\nidentical ranking body text for testing\n",
    );
    write_page(
        &wiki_root,
        "concepts/active-mid.md",
        "---\ntitle: \"Active Mid\"\nstatus: active\ntype: concept\nconfidence: 0.5\n---\n\nidentical ranking body text for testing\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let results = search(
        "identical ranking body",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert_eq!(results.results.len(), 2);
    assert_eq!(results.results[0].slug, "concepts/active-mid");
}

#[test]
fn search_ranking_custom_config_zero_archived() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/archived.md",
        &status_page("Archived", "archived"),
    );
    write_page(
        &wiki_root,
        "concepts/active.md",
        &status_page("Active", "active"),
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let mut custom_status = llm_wiki::config::SearchConfig::default();
    custom_status.status.insert("archived".into(), 0.0);
    let opts = SearchOptions {
        search_config: custom_status,
        ..SearchOptions::default()
    };
    let results = search(
        "identical ranking body",
        &opts,
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    // archived pages score 0 so should not appear or appear last with score 0
    let slugs: Vec<&str> = results.results.iter().map(|r| r.slug.as_str()).collect();
    let pos_active = slugs.iter().position(|&s| s == "concepts/active").unwrap();
    let pos_archived = slugs
        .iter()
        .position(|&s| s == "concepts/archived")
        .unwrap();
    assert!(pos_active < pos_archived);
}

#[test]
fn search_ranking_custom_status_mapped() {
    // active × 1.0 × 0.5 = 0.5  >  stub × 0.6 × 0.5 = 0.3
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/stub-page.md",
        "---\ntitle: \"Stub\"\nstatus: stub\ntype: concept\nconfidence: 0.5\n---\n\nidentical ranking body text for testing\n",
    );
    write_page(
        &wiki_root,
        "concepts/active-page.md",
        "---\ntitle: \"Active\"\nstatus: active\ntype: concept\nconfidence: 0.5\n---\n\nidentical ranking body text for testing\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    let mut custom_sc = SearchConfig::default();
    custom_sc.status.insert("stub".into(), 0.6);
    let opts = SearchOptions {
        search_config: custom_sc,
        ..SearchOptions::default()
    };
    let results = search(
        "identical ranking body",
        &opts,
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert_eq!(results.results.len(), 2);
    let slugs: Vec<&str> = results.results.iter().map(|r| r.slug.as_str()).collect();
    let pos_active = slugs
        .iter()
        .position(|&s| s == "concepts/active-page")
        .unwrap();
    let pos_stub = slugs
        .iter()
        .position(|&s| s == "concepts/stub-page")
        .unwrap();
    assert!(
        pos_active < pos_stub,
        "active (×1.0) should rank above stub (×0.6)"
    );
}

#[test]
fn search_ranking_custom_status_falls_back_to_unknown() {
    // stub has no entry in the map → uses unknown (×0.9)
    // a page with no status also uses unknown (×0.9)
    // both should produce the same multiplier and thus equal scores
    let dir = tempfile::tempdir().unwrap();
    let wiki_root = setup_repo(dir.path());
    write_page(
        &wiki_root,
        "concepts/has-stub.md",
        "---\ntitle: \"Has Stub\"\nstatus: stub\ntype: concept\nconfidence: 0.5\n---\n\nidentical ranking body text for testing\n",
    );
    write_page(
        &wiki_root,
        "concepts/no-status.md",
        "---\ntitle: \"No Status\"\ntype: concept\nconfidence: 0.5\n---\n\nidentical ranking body text for testing\n",
    );

    let mgr = build_index(dir.path(), &wiki_root);
    let is = schema();
    // default config: no "stub" entry — falls back to "unknown" = 0.9
    let results = search(
        "identical ranking body",
        &SearchOptions::default(),
        &mgr.searcher().unwrap(),
        "test",
        &is,
    )
    .unwrap();

    assert_eq!(results.results.len(), 2);
    let score_stub = results
        .results
        .iter()
        .find(|r| r.slug == "concepts/has-stub")
        .unwrap()
        .score;
    let score_no_status = results
        .results
        .iter()
        .find(|r| r.slug == "concepts/no-status")
        .unwrap()
        .score;
    // Both use unknown multiplier; scores should be equal (same body, same confidence, same multiplier)
    assert!(
        (score_stub - score_no_status).abs() < 1e-5,
        "stub (no map entry) and no-status should score the same; got {score_stub} vs {score_no_status}"
    );
}
