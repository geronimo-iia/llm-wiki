use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;
use std::fs;

// ── Search ────────────────────────────────────────────────────────────────────

#[test]
fn search_returns_results() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let results = ops::search(
        &engine,
        "test",
        &ops::SearchParams {
            query: "mixture",
            type_filter: None,
            no_excerpt: false,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();
    assert!(!results.results.is_empty());
    assert_eq!(results.results[0].slug, "concepts/moe");
}

#[test]
fn search_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let results = ops::search(
        &engine,
        "test",
        &ops::SearchParams {
            query: "mixture",
            type_filter: Some("paper"),
            no_excerpt: true,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();
    assert!(results.results.is_empty());
}

// ── List ──────────────────────────────────────────────────────────────────────

#[test]
fn list_returns_pages() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::list(&engine, "test", None, None, 1, None).unwrap();
    assert!(result.total >= 2);
}

#[test]
fn list_type_filter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::list(&engine, "test", Some("concept"), None, 1, None).unwrap();
    assert!(result.total >= 2);

    let result = ops::list(&engine, "test", Some("paper"), None, 1, None).unwrap();
    assert_eq!(result.total, 0);
}

// ── Facets ────────────────────────────────────────────────────────────────────

#[test]
fn search_facets_type_distribution() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    // Add a paper page alongside the existing concepts
    fs::create_dir_all(wiki_root.join("sources")).unwrap();
    fs::write(
        wiki_root.join("sources/paper-a.md"),
        "---\ntitle: \"MoE Paper\"\ntype: paper\nstatus: active\ntags: [ml]\n---\n\nMixture of Experts paper.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add paper").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::search(
        &engine,
        "test",
        &ops::SearchParams {
            query: "mixture",
            type_filter: None,
            no_excerpt: true,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();

    // Type facet should show both concept and paper
    assert!(
        result.facets.r#type.contains_key("concept"),
        "type facet should contain concept, got: {:?}",
        result.facets.r#type
    );
    assert!(
        result.facets.r#type.contains_key("paper"),
        "type facet should contain paper, got: {:?}",
        result.facets.r#type
    );
}

#[test]
fn search_facets_type_unfiltered_when_type_filter_active() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    fs::create_dir_all(wiki_root.join("sources")).unwrap();
    fs::write(
        wiki_root.join("sources/paper-b.md"),
        "---\ntitle: \"Experts Paper\"\ntype: paper\nstatus: active\n---\n\nMixture of Experts scaling.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add paper").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    // Search with type filter on concept
    let result = ops::search(
        &engine,
        "test",
        &ops::SearchParams {
            query: "experts",
            type_filter: Some("concept"),
            no_excerpt: true,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();

    // Type facet should still show paper (unfiltered)
    assert!(
        result.facets.r#type.contains_key("paper"),
        "type facet should be unfiltered and show paper, got: {:?}",
        result.facets.r#type
    );
}

#[test]
fn search_facets_empty_when_no_results() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::search(
        &engine,
        "test",
        &ops::SearchParams {
            query: "xyznonexistent",
            type_filter: None,
            no_excerpt: true,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();

    assert!(result.results.is_empty());
    assert!(result.facets.r#type.is_empty());
    assert!(result.facets.status.is_empty());
    assert!(result.facets.tags.is_empty());
}

#[test]
fn list_facets_always_present() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::list(&engine, "test", None, None, 1, None).unwrap();

    // Should have type facet with at least "concept"
    assert!(
        result.facets.r#type.contains_key("concept"),
        "list facets should contain concept, got: {:?}",
        result.facets.r#type
    );
    assert!(
        !result.facets.status.is_empty(),
        "list facets should have status distribution"
    );
}
