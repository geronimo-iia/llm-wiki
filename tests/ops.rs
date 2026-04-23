use std::fs;
use std::path::Path;

use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;

fn setup_wiki(dir: &Path, name: &str) -> std::path::PathBuf {
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path).unwrap();

    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"MoE\"\ntype: concept\nstatus: active\ntags: [ml]\n---\n\nMixture of Experts.\n",
    )
    .unwrap();
    fs::write(
        wiki_root.join("concepts/transformer.md"),
        "---\ntitle: \"Transformer\"\ntype: concept\nstatus: active\n---\n\nAttention is all you need. See [[concepts/moe]].\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add pages").unwrap();

    config_path
}

// ── Spaces ────────────────────────────────────────────────────────────────────

#[test]
fn spaces_create_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let entries = ops::spaces_list(&global, None);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "test");
}

#[test]
fn spaces_list_filters_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let beta_path = dir.path().join("beta");
    ops::spaces_create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let filtered = ops::spaces_list(&global, Some("beta"));
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "beta");
}

#[test]
fn spaces_list_unknown_name_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let global = llm_wiki::config::load_global(&config_path).unwrap();
    let filtered = ops::spaces_list(&global, Some("nonexistent"));
    assert!(filtered.is_empty());
}

#[test]
fn spaces_set_default_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    // Create a second wiki
    let beta_path = dir.path().join("beta");
    ops::spaces_create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

    ops::spaces_set_default("beta", &config_path, None).unwrap();
    let global = llm_wiki::config::load_global(&config_path).unwrap();
    assert_eq!(global.global.default_wiki, "beta");

    ops::spaces_remove("alpha", false, &config_path, None).unwrap();
    let global = llm_wiki::config::load_global(&config_path).unwrap();
    assert_eq!(global.wikis.len(), 1);
}

// ── Config ────────────────────────────────────────────────────────────────────

#[test]
fn config_get_returns_value() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let val = ops::config_get(&config_path, "defaults.search_top_k").unwrap();
    assert_eq!(val, "10");
}

#[test]
fn config_set_global() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let msg = ops::config_set(&config_path, "defaults.search_top_k", "20", true, None).unwrap();
    assert!(msg.contains("20"));

    let val = ops::config_get(&config_path, "defaults.search_top_k").unwrap();
    assert_eq!(val, "20");
}

#[test]
fn config_list_global_returns_toml() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let s = ops::config_list_global(&config_path).unwrap();
    assert!(s.contains("[global]"));
}

#[test]
fn config_list_resolved_returns_struct() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let resolved = ops::config_list_resolved(&config_path).unwrap();
    assert_eq!(resolved.defaults.search_top_k, 10);
}

// ── Content ───────────────────────────────────────────────────────────────────

#[test]
fn content_read_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    match ops::content_read(&engine, "concepts/moe", None, false, false).unwrap() {
        ops::ContentReadResult::Page(content) => {
            assert!(content.contains("Mixture of Experts"));
        }
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_read_no_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    match ops::content_read(&engine, "concepts/moe", None, true, false).unwrap() {
        ops::ContentReadResult::Page(content) => {
            assert!(!content.contains("title:"));
            assert!(content.contains("Mixture of Experts"));
        }
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_write_and_read_back() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let body = "---\ntitle: \"New\"\ntype: page\n---\n\nHello.\n";
    let result = ops::content_write(&engine, "new-page", None, body).unwrap();
    assert_eq!(result.bytes_written, body.len());

    match ops::content_read(&engine, "new-page", None, false, false).unwrap() {
        ops::ContentReadResult::Page(content) => assert!(content.contains("Hello.")),
        _ => panic!("expected Page"),
    }
}

#[test]
fn content_new_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let uri = ops::content_new(
        &engine,
        "concepts/new-concept",
        None,
        false,
        false,
        None,
        None,
    )
    .unwrap();
    assert!(uri.starts_with("wiki://test/concepts/new-concept"));
}

#[test]
fn content_new_section() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let uri = ops::content_new(&engine, "topics", None, true, false, None, None).unwrap();
    assert!(uri.contains("topics"));
}

#[test]
fn content_commit_all() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    // Write a new file so there's something to commit
    ops::content_write(
        &engine,
        "scratch",
        None,
        "---\ntitle: \"Scratch\"\ntype: page\n---\n\ntemp\n",
    )
    .unwrap();

    let hash = ops::content_commit(&engine, "test", &[], true, Some("test commit")).unwrap();
    assert!(!hash.is_empty());
}

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

// ── Ingest ────────────────────────────────────────────────────────────────────

#[test]
fn ingest_validates_and_indexes() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Write a new page
    {
        let engine = manager.state.read().unwrap();
        let space = engine.space("test").unwrap();
        fs::write(
            space.wiki_root.join("concepts/rag.md"),
            "---\ntitle: \"RAG\"\ntype: concept\nstatus: active\n---\n\nRetrieval-augmented generation.\n",
        )
        .unwrap();
    }

    let report = {
        let engine = manager.state.read().unwrap();
        ops::ingest(&engine, &manager, "concepts/rag.md", false, "test").unwrap()
    };
    assert_eq!(report.pages_validated, 1);
}

#[test]
fn ingest_dry_run_does_not_commit() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let head_before = {
        let engine = manager.state.read().unwrap();
        let space = engine.space("test").unwrap();
        git::current_head(&space.repo_root)
    };

    let report = {
        let engine = manager.state.read().unwrap();
        ops::ingest(&engine, &manager, "concepts/moe.md", true, "test").unwrap()
    };
    assert_eq!(report.pages_validated, 1);
    assert!(report.commit.is_empty());

    let head_after = {
        let engine = manager.state.read().unwrap();
        let space = engine.space("test").unwrap();
        git::current_head(&space.repo_root)
    };
    assert_eq!(head_before, head_after);
}

// ── Index ─────────────────────────────────────────────────────────────────────

#[test]
fn index_rebuild_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let report = ops::index_rebuild(&manager, "test").unwrap();
    assert!(report.pages_indexed >= 2);

    let engine = manager.state.read().unwrap();
    let status = ops::index_status(&engine, "test").unwrap();
    assert!(status.openable);
    assert!(status.queryable);
    assert!(!status.stale);
}

// ── Graph ─────────────────────────────────────────────────────────────────────

#[test]
fn graph_build_returns_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::graph_build(
        &engine,
        "test",
        &ops::GraphParams {
            format: Some("mermaid"),
            root: None,
            depth: None,
            type_filter: None,
            relation: None,
            output: None,
        },
    )
    .unwrap();
    assert!(result.report.nodes >= 2);
    assert!(result.rendered.contains("graph LR"));
}

#[test]
fn graph_build_dot_format() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::graph_build(
        &engine,
        "test",
        &ops::GraphParams {
            format: Some("dot"),
            root: None,
            depth: None,
            type_filter: None,
            relation: None,
            output: None,
        },
    )
    .unwrap();
    assert!(result.rendered.contains("digraph wiki"));
}

// ── Schema ────────────────────────────────────────────────────────────────────

#[test]
fn schema_list_returns_types() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let types = ops::schema_list(&engine, "test").unwrap();
    assert!(types.len() >= 15);
    assert!(types.iter().any(|t| t.name == "concept"));
    assert!(types.iter().any(|t| t.name == "skill"));
}

#[test]
fn schema_show_returns_json() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let content = ops::schema_show(&engine, "test", "concept").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.get("properties").is_some());
}

#[test]
fn schema_show_unknown_type_errors() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::schema_show(&engine, "test", "nonexistent");
    assert!(result.is_err());
}

#[test]
fn schema_show_template_has_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let template = ops::schema_show_template(&engine, "test", "concept").unwrap();
    assert!(template.starts_with("---"));
    assert!(template.contains("type: concept"));
    assert!(template.contains("title:"));
}

#[test]
fn schema_validate_passes_default_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let issues = ops::schema_validate(&engine, "test", None).unwrap();
    assert!(
        issues.is_empty(),
        "default schemas should validate: {:?}",
        issues
    );
}

// ── Phase 3: edge target type warnings ───────────────────────────────────────

#[test]
fn ingest_warns_on_wrong_edge_target_type() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    // Create a concept page with sources pointing to another concept (wrong type)
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/bad.md"),
        "---\ntitle: \"Bad\"\ntype: concept\nstatus: active\nsources:\n  - concepts/moe\n---\n\nBody.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add bad page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let report = ops::ingest(&engine, &manager, "concepts/bad.md", false, "test").unwrap();

    // Should warn: concepts/moe is type "concept", but sources expects source types
    assert!(
        report
            .warnings
            .iter()
            .any(|w| w.contains("concepts/moe") && w.contains("concept")),
        "expected warning about wrong target type, got: {:?}",
        report.warnings
    );
}

#[test]
fn ingest_no_warning_on_correct_edge_target_type() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    // Create a paper page and a concept that references it correctly
    fs::create_dir_all(wiki_root.join("sources")).unwrap();
    fs::write(
        wiki_root.join("sources/paper-a.md"),
        "---\ntitle: \"Paper A\"\ntype: paper\nstatus: active\n---\n\nBody.\n",
    )
    .unwrap();
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/good.md"),
        "---\ntitle: \"Good\"\ntype: concept\nstatus: active\nsources:\n  - sources/paper-a\n---\n\nBody.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add pages").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let report = ops::ingest(&engine, &manager, "concepts/good.md", false, "test").unwrap();

    // No edge target warnings expected
    let edge_warnings: Vec<&String> = report
        .warnings
        .iter()
        .filter(|w| w.contains("edge"))
        .collect();
    assert!(
        edge_warnings.is_empty(),
        "unexpected edge warnings: {:?}",
        edge_warnings
    );
}

// ── Hot Reload ────────────────────────────────────────────────────────────────

#[test]
fn hot_reload_mount_wiki_makes_it_searchable() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Create beta wiki structure first (before mounting)
    let beta_path = dir.path().join("beta");
    llm_wiki::spaces::create(
        &beta_path,
        "beta",
        Some("second wiki"),
        false,
        false,
        &config_path,
    )
    .unwrap();

    // Write a page into beta before hot-reload mount
    let beta_wiki = beta_path.join("wiki");
    fs::create_dir_all(beta_wiki.join("concepts")).unwrap();
    fs::write(
        beta_wiki.join("concepts/rlhf.md"),
        "---\ntitle: \"RLHF\"\ntype: concept\nstatus: active\n---\n\nReinforcement learning from human feedback.\n",
    )
    .unwrap();
    git::commit(&beta_path, "add page").unwrap();

    // Now hot-reload mount — index builds with the page already present
    let entry = llm_wiki::config::WikiEntry {
        name: "beta".into(),
        path: beta_path.to_string_lossy().into(),
        description: Some("second wiki".into()),
        remote: None,
    };
    manager.mount_wiki(&entry).unwrap();

    // Search beta — should find the page
    let engine = manager.state.read().unwrap();
    let results = ops::search(
        &engine,
        "beta",
        &ops::SearchParams {
            query: "reinforcement",
            type_filter: None,
            no_excerpt: false,
            top_k: None,
            include_sections: false,
            cross_wiki: false,
        },
    )
    .unwrap();
    assert!(
        !results.results.is_empty(),
        "beta wiki should be searchable after hot reload mount"
    );
}

#[test]
fn hot_reload_unmount_wiki_removes_from_search() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    // Create beta
    let beta_path = dir.path().join("beta");
    llm_wiki::spaces::create(&beta_path, "beta", None, false, false, &config_path).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();

    // Verify beta is mounted
    {
        let engine = manager.state.read().unwrap();
        assert!(engine.space("beta").is_ok());
    }

    // Unmount beta via ops
    ops::spaces_remove("beta", false, &config_path, Some(&manager)).unwrap();

    // Verify beta is no longer mounted
    let engine = manager.state.read().unwrap();
    assert!(engine.space("beta").is_err());
}

#[test]
fn hot_reload_refuse_unmount_default_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    // alpha is the default — unmount should fail
    let result = manager.unmount_wiki("alpha");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("default"),
        "error should mention default wiki"
    );
}

#[test]
fn hot_reload_set_default_updates_engine() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    let beta_path = dir.path().join("beta");
    llm_wiki::spaces::create(&beta_path, "beta", None, false, false, &config_path).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();

    // Set beta as default via ops
    ops::spaces_set_default("beta", &config_path, Some(&manager)).unwrap();

    // Verify engine state updated
    let engine = manager.state.read().unwrap();
    assert_eq!(engine.default_wiki_name(), "beta");
}

#[test]
fn hot_reload_cross_wiki_search_reflects_new_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    // Create beta with a page before building the engine
    let beta_path = dir.path().join("beta");
    llm_wiki::spaces::create(&beta_path, "beta", None, false, false, &config_path).unwrap();

    let beta_wiki = beta_path.join("wiki");
    fs::create_dir_all(beta_wiki.join("concepts")).unwrap();
    fs::write(
        beta_wiki.join("concepts/diffusion.md"),
        "---\ntitle: \"Diffusion Models\"\ntype: concept\nstatus: active\n---\n\nScore-based generative models.\n",
    )
    .unwrap();
    git::commit(&beta_path, "add page").unwrap();

    // Build engine with only alpha mounted
    // Remove beta from config so it's not mounted at startup
    llm_wiki::spaces::remove("beta", false, &config_path).unwrap();
    let manager = WikiEngine::build(&config_path).unwrap();

    // Re-register and hot-reload mount beta
    let entry = llm_wiki::config::WikiEntry {
        name: "beta".into(),
        path: beta_path.to_string_lossy().into(),
        description: None,
        remote: None,
    };
    llm_wiki::spaces::register(entry.clone(), false, &config_path).unwrap();
    manager.mount_wiki(&entry).unwrap();

    // Cross-wiki search from alpha should find beta's page
    let engine = manager.state.read().unwrap();
    let results = ops::search(
        &engine,
        "alpha",
        &ops::SearchParams {
            query: "diffusion",
            type_filter: None,
            no_excerpt: false,
            top_k: None,
            include_sections: false,
            cross_wiki: true,
        },
    )
    .unwrap();
    assert!(
        results
            .results
            .iter()
            .any(|r| r.slug == "concepts/diffusion"),
        "cross-wiki search should find beta's page, got: {:?}",
        results
    );
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

// ── History ───────────────────────────────────────────────────────────────────

#[test]
fn history_returns_commits_for_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::history(&engine, "concepts/moe", None, None, None).unwrap();
    assert!(
        !result.entries.is_empty(),
        "history should have at least one commit"
    );
    assert_eq!(result.slug, "concepts/moe");
}

#[test]
fn history_respects_limit() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    // Make a second commit touching the same page
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");
    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"MoE\"\ntype: concept\nstatus: active\ntags: [ml, updated]\n---\n\nUpdated content.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "update moe").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::history(&engine, "concepts/moe", None, Some(1), None).unwrap();
    assert_eq!(result.entries.len(), 1, "limit should cap to 1 entry");
}

#[test]
fn history_excludes_unrelated_commits() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    // Make a commit that only touches transformer, not moe
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");
    fs::write(
        wiki_root.join("concepts/transformer.md"),
        "---\ntitle: \"Transformer\"\ntype: concept\nstatus: active\n---\n\nUpdated transformer.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "update transformer only").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::history(&engine, "concepts/moe", None, None, None).unwrap();
    // The "update transformer only" commit should NOT appear in moe's history
    assert!(
        !result
            .entries
            .iter()
            .any(|e| e.message == "update transformer only"),
        "unrelated commits should not appear in history"
    );
}

#[test]
fn history_empty_for_nonexistent_page() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::history(&engine, "concepts/nonexistent", None, None, None);
    // Should error because slug doesn't resolve to a file
    assert!(result.is_err());
}

#[test]
fn history_via_wiki_uri() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = ops::history(&engine, "wiki://test/concepts/moe", None, None, None).unwrap();
    assert!(!result.entries.is_empty());
}

// ── Git-level history ─────────────────────────────────────────────────────────

#[test]
fn git_page_history_returns_entries() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let _ = config_path; // just to set up the wiki

    let repo_root = dir.path().join("test");
    let rel_path = std::path::Path::new("wiki/concepts/moe.md");

    let entries = git::page_history(&repo_root, rel_path, 10, false).unwrap();
    assert!(
        !entries.is_empty(),
        "git page_history should return at least one entry"
    );
    assert!(!entries[0].hash.is_empty());
    assert!(!entries[0].date.is_empty());
    assert!(!entries[0].author.is_empty());
}

#[test]
fn git_page_history_follow_tracks_rename() {
    let dir = tempfile::tempdir().unwrap();
    let wiki_path = dir.path().join("wiki");
    let config_path = dir.path().join("state").join("config.toml");

    // Create wiki with a flat page
    llm_wiki::spaces::create(&wiki_path, "test", None, false, true, &config_path).unwrap();
    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/old-name.md"),
        "---\ntitle: \"Old\"\ntype: concept\n---\n\nOriginal.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "create old-name").unwrap();

    // Rename the file
    fs::rename(
        wiki_root.join("concepts/old-name.md"),
        wiki_root.join("concepts/new-name.md"),
    )
    .unwrap();
    git::commit(&wiki_path, "rename to new-name").unwrap();

    // With follow, history should include the pre-rename commit
    let entries = git::page_history(
        &wiki_path,
        std::path::Path::new("wiki/concepts/new-name.md"),
        10,
        true,
    )
    .unwrap();
    assert!(
        entries.len() >= 2,
        "follow should track rename, got {} entries: {:?}",
        entries.len(),
        entries.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    assert!(entries.iter().any(|e| e.message == "create old-name"));

    // Without follow, history should only show the rename commit
    let entries_no_follow = git::page_history(
        &wiki_path,
        std::path::Path::new("wiki/concepts/new-name.md"),
        10,
        false,
    )
    .unwrap();
    assert!(
        entries_no_follow.len() < entries.len(),
        "no-follow should have fewer entries than follow"
    );
}

// ── Watch (engine-level) ──────────────────────────────────────────────────────

#[test]
fn schema_rebuild_partial_on_type_change() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Verify schema_rebuild works without error on a clean wiki
    let result = manager.schema_rebuild("test");
    assert!(
        result.is_ok(),
        "schema_rebuild should succeed: {:?}",
        result
    );
}

#[test]
fn schema_rebuild_errors_on_unknown_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();

    let result = manager.schema_rebuild("nonexistent");
    assert!(result.is_err());
}
