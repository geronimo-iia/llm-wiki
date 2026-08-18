use super::helpers::setup_wiki;
use llm_wiki_engine::engine::WikiEngine;
use llm_wiki_engine::git;
use llm_wiki_engine::ops;
use std::fs;

// ── Hot Reload ────────────────────────────────────────────────────────────────

#[test]
fn hot_reload_mount_wiki_makes_it_searchable() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Create beta wiki structure first (before mounting)
    let beta_path = dir.path().join("beta");
    llm_wiki_engine::spaces::create(
        &beta_path,
        "beta",
        Some("second wiki"),
        false,
        false,
        &config_path,
        None,
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
    let entry = llm_wiki_engine::config::WikiEntry {
        name: "beta".into(),
        path: beta_path.clone(),
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
    llm_wiki_engine::spaces::create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

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

/// Invariant 4: `spaces_set_default` must not update disk config when the target
/// wiki is not mounted. The engine validation runs BEFORE the disk write, so a
/// failure leaves the on-disk config unchanged.
#[test]
fn spaces_set_default_fails_and_keeps_disk_config_when_wiki_unmounted() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    let beta_path = dir.path().join("beta");
    llm_wiki_engine::spaces::create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();

    // Unmount beta from the engine (it remains in config, but is not mounted)
    manager.unmount_wiki("beta").unwrap();

    // Attempting set_default("beta") must fail — wiki not mounted in engine
    let result = ops::spaces_set_default("beta", &config_path, Some(&manager));
    assert!(
        result.is_err(),
        "spaces_set_default must fail when wiki is not mounted"
    );

    // Disk config must still show alpha as default — no partial write
    let global = llm_wiki_engine::config::load_global(&config_path).unwrap();
    assert_eq!(
        global.global.default_wiki, "alpha",
        "disk config must not be updated after set_default failure"
    );
}

#[test]
fn hot_reload_set_default_updates_engine() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    let beta_path = dir.path().join("beta");
    llm_wiki_engine::spaces::create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();

    // Set beta as default via ops
    ops::spaces_set_default("beta", &config_path, Some(&manager)).unwrap();

    // Verify engine state updated
    let engine = manager.state.read().unwrap();
    assert_eq!(engine.default_wiki_name(), Some("beta"));
}

#[test]
fn hot_reload_cross_wiki_search_reflects_new_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");

    // Create beta with a page before building the engine
    let beta_path = dir.path().join("beta");
    llm_wiki_engine::spaces::create(&beta_path, "beta", None, false, false, &config_path, None).unwrap();

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
    llm_wiki_engine::spaces::remove("beta", false, &config_path).unwrap();
    let manager = WikiEngine::build(&config_path).unwrap();

    // Re-register and hot-reload mount beta
    let entry = llm_wiki_engine::config::WikiEntry {
        name: "beta".into(),
        path: beta_path.clone(),
        description: None,
        remote: None,
    };
    llm_wiki_engine::spaces::register(entry.clone(), false, &config_path).unwrap();
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

#[test]
fn spaces_create_set_default_updates_engine() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    let beta_path = dir.path().join("beta");
    ops::spaces_create(
        &beta_path,
        "beta",
        None,
        false,
        true, // set_default
        &config_path,
        Some(&manager),
        None,
    )
    .unwrap();

    // Engine default must update in-process without restart.
    let engine = manager.state.read().unwrap();
    assert_eq!(engine.default_wiki_name(), Some("beta"));
}

#[test]
fn spaces_create_mount_failure_rolls_back_config() {
    // build_space reads only .json files from schemas/. Pre-creating bad.json
    // with invalid JSON causes mount_wiki to fail. ensure_structure skips
    // existing files, so bad.json survives into the mount attempt.
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Pre-create beta with a corrupt .json schema. spaces_create will call
    // ensure_structure (which won't overwrite existing files) then register
    // then attempt to mount — which must fail.
    let beta_path = dir.path().join("beta");
    std::fs::create_dir_all(beta_path.join("wiki")).unwrap();
    let schemas = beta_path.join("schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    std::fs::write(schemas.join("bad.json"), "{ this is not valid json").unwrap();

    let result = ops::spaces_create(
        &beta_path,
        "beta",
        None,
        false,
        false,
        &config_path,
        Some(&manager),
        None,
    );
    assert!(
        result.is_err(),
        "spaces_create must return Err when mount_wiki fails"
    );

    // Invariant: config must not contain beta after a failed spaces_create.
    let global = llm_wiki_engine::config::load_global(&config_path).unwrap();
    assert!(
        !global.wikis.iter().any(|w| w.name == "beta"),
        "beta must be absent from config after rollback"
    );

    // Engine must not have beta mounted.
    let engine = manager.state.read().unwrap();
    assert!(engine.space("beta").is_err(), "beta must not be mounted");
}

#[test]
fn spaces_register_mount_failure_rolls_back_config() {
    // Same trigger as spaces_create test: pre-create bad.json with invalid
    // JSON so build_space fails inside mount_wiki.
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "alpha");
    let manager = WikiEngine::build(&config_path).unwrap();

    // Create a wiki dir with git, wiki/ root, and a corrupt .json schema.
    let beta_path = dir.path().join("beta");
    std::fs::create_dir_all(beta_path.join("wiki")).unwrap();
    llm_wiki_engine::git::init_repo(&beta_path).unwrap();
    let schemas = beta_path.join("schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    std::fs::write(schemas.join("bad.json"), "{ this is not valid json").unwrap();

    // spaces_register with a live engine — must fail and roll back.
    let result = ops::spaces_register(&beta_path, "beta", None, None, &config_path, Some(&manager));
    assert!(
        result.is_err(),
        "spaces_register must return Err when mount_wiki fails"
    );

    let global = llm_wiki_engine::config::load_global(&config_path).unwrap();
    assert!(
        !global.wikis.iter().any(|w| w.name == "beta"),
        "beta must be absent from config after rollback"
    );

    let engine = manager.state.read().unwrap();
    assert!(engine.space("beta").is_err(), "beta must not be mounted");
}
