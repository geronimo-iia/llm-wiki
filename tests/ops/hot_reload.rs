use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;
use std::fs;

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
