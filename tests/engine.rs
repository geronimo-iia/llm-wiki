use std::fs;
use std::path::Path;

use llm_wiki_engine::config::Tokenizer;
use llm_wiki_engine::engine::WikiEngine;
use llm_wiki_engine::git;

fn setup_wiki(dir: &Path, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    // config lives at <dir>/state/config.toml → state_dir = <dir>/state/
    // indexes will be at <dir>/state/indexes/<name>/
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki_engine::spaces::create(&wiki_path, name, None, false, true, &config_path, None)
        .unwrap();

    // Write a page so the index has something
    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/moe.md"),
        "---\ntitle: \"MoE\"\ntype: concept\nstatus: active\n---\n\nMixture of Experts.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add page").unwrap();

    (config_path, wiki_path)
}

// ── build ─────────────────────────────────────────────────────────────────────

#[test]
fn engine_builds_from_config() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    assert_eq!(engine.default_wiki_name(), Some("test"));
    assert!(engine.spaces.contains_key("test"));
}

#[test]
fn engine_builds_with_no_wikis() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, "[global]\ndefault_wiki = \"\"\n").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    assert!(engine.spaces.is_empty());
}

#[test]
fn engine_builds_with_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("nonexistent").join("config.toml");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    assert!(engine.spaces.is_empty());
}

#[test]
fn engine_mount_fails_loud_on_broken_schema() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, wiki_path) = setup_wiki(dir.path(), "test");

    // Corrupt a schema file — the wiki must fail to mount instead of
    // silently falling back to embedded defaults.
    fs::write(wiki_path.join("schemas/concept.json"), "not valid json {{{").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    assert!(
        engine.space("test").is_err(),
        "wiki with a broken schema must not mount"
    );

    // The error chain names the wiki and the broken schema file.
    let err = match llm_wiki_engine::space_builder::build_space(&wiki_path, &Tokenizer::EnStem) {
        Err(e) => e,
        Ok(_) => panic!("build_space should fail on a broken schema"),
    };
    let chain = format!("{err:#}");
    assert!(
        chain.contains("concept.json"),
        "error should name the broken schema file: {chain}"
    );
}

#[test]
fn engine_mounts_wiki_without_schemas_dir() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, wiki_path) = setup_wiki(dir.path(), "test");

    // A wiki with NO schemas/ directory must still mount, using the
    // embedded defaults inside the schema-source collection.
    fs::remove_dir_all(wiki_path.join("schemas")).unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.space("test").unwrap();
    assert!(space.type_registry.is_known("concept"));
}

// ── space access ──────────────────────────────────────────────────────────────

#[test]
fn engine_space_returns_mounted_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "research");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let space = engine.space("research").unwrap();
    assert_eq!(space.name, "research");
    assert!(space.wiki_root.ends_with("wiki"));
}

#[test]
fn engine_space_errors_on_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    assert!(engine.space("nonexistent").is_err());
}

#[test]
fn resolve_wiki_name_uses_default() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "research");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    assert_eq!(engine.resolve_wiki_name(None).unwrap(), "research");
    assert_eq!(engine.resolve_wiki_name(Some("other")).unwrap(), "other");
}

#[test]
fn index_path_derived_from_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let idx_path = engine.index_path_for("test");
    assert!(idx_path.starts_with(dir.path().join("state")));
    assert!(idx_path.ends_with("indexes/test"));
}

// ── refresh_index ─────────────────────────────────────────────────────────────

#[test]
fn refresh_index_updates_index() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, wiki_path) = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();

    // Write a new page after engine build
    let wiki_root = wiki_path.join("wiki");
    fs::write(
        wiki_root.join("concepts/new.md"),
        "---\ntitle: \"New\"\ntype: concept\nstatus: active\n---\n\nNew.\n",
    )
    .unwrap();

    let report = manager.refresh_index("test").unwrap();
    assert_eq!(report.updated, 1);
}

// ── rebuild_index ─────────────────────────────────────────────────────────────

#[test]
fn rebuild_index_works() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "test");

    let manager = WikiEngine::build(&config_path).unwrap();
    let report = manager.rebuild_index("test").unwrap();

    assert!(report.pages_indexed >= 1);
}

#[test]
fn engine_mounts_wiki_with_custom_wiki_root() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("skills-wiki");

    llm_wiki_engine::spaces::create(
        &wiki_path,
        "skills",
        None,
        false,
        true,
        &config_path,
        Some("skills"),
    )
    .unwrap();

    let wiki_root = wiki_path.join("skills");
    fs::create_dir_all(wiki_root.join("bootstrap")).unwrap();
    fs::write(
        wiki_root.join("bootstrap/SKILL.md"),
        "---\ntitle: \"Bootstrap\"\ntype: page\nstatus: active\n---\n\nBootstrap skill.\n",
    )
    .unwrap();
    llm_wiki_engine::git::commit(&wiki_path, "add skill page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.space("skills").unwrap();

    // canonicalize both sides: macOS resolves /var→/private/var, Windows adds \\?\
    let expected_wiki_root = wiki_path.canonicalize().unwrap().join("skills");
    assert_eq!(space.wiki_root.canonicalize().unwrap(), expected_wiki_root);
}

#[test]
fn engine_indexes_custom_wiki_root_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let fixture_path = std::path::Path::new("tests/fixtures/wikis/alt-root");

    llm_wiki_engine::spaces::register_existing(fixture_path, "alt-root", None, None, &config_path)
        .unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine_guard = manager.state.read().unwrap();
    let space = engine_guard.space("alt-root").unwrap();

    assert!(space.wiki_root.ends_with("content"));
}

#[test]
fn content_read_works_with_custom_wiki_root() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("skills-wiki");

    llm_wiki_engine::spaces::create(
        &wiki_path,
        "skills",
        None,
        false,
        true,
        &config_path,
        Some("skills"),
    )
    .unwrap();

    let wiki_root = wiki_path.join("skills");
    fs::create_dir_all(&wiki_root).unwrap();
    fs::write(
        wiki_root.join("bootstrap.md"),
        "---\ntitle: \"Bootstrap\"\ntype: page\nstatus: active\n---\n\nContent.\n",
    )
    .unwrap();
    llm_wiki_engine::git::commit(&wiki_path, "add page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result =
        llm_wiki_engine::ops::content_read(&engine, "wiki://skills/bootstrap", None, false, false)
            .unwrap();
    match result {
        llm_wiki_engine::ops::ContentReadResult::Page(text) => {
            assert!(text.contains("Bootstrap"));
        }
        _ => panic!("expected Page result"),
    }
}

#[test]
fn auto_recovery_false_leaves_index_unavailable_after_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "test");

    // First build — index is healthy.
    let manager = WikiEngine::build(&config_path).unwrap();
    let idx_path = manager.state.read().unwrap().index_path_for("test");
    drop(manager);

    // Corrupt every file in the search-index directory.
    let search_dir = idx_path.join("search-index");
    for entry in fs::read_dir(&search_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::write(entry.path(), b"corrupted").unwrap();
        }
    }

    // Disable auto-recovery in config.
    let mut cfg = llm_wiki_engine::config::load_global(&config_path).unwrap();
    cfg.index.auto_recovery = false;
    llm_wiki_engine::config::save_global(&cfg, &config_path).unwrap();

    // Remount — open() should NOT rebuild; searcher must be unavailable.
    let manager2 = WikiEngine::build(&config_path).unwrap();
    let engine = manager2.state.read().unwrap();
    let space = engine.space("test").unwrap();
    assert!(
        space.index_manager.searcher().is_err(),
        "searcher must be unavailable when auto_recovery = false and index is corrupt"
    );
}

#[test]
fn set_default_updates_engine_and_disk_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let (config_path, _) = setup_wiki(dir.path(), "alpha");

    // Register a second wiki so switching default is meaningful.
    let beta_path = dir.path().join("beta");
    llm_wiki_engine::spaces::create(&beta_path, "beta", None, false, false, &config_path, None)
        .unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();

    // Precondition: alpha is default.
    assert_eq!(
        manager.state.read().unwrap().default_wiki_name(),
        Some("alpha")
    );

    llm_wiki_engine::ops::spaces_set_default("beta", &config_path, Some(&manager)).unwrap();

    // In-memory engine reflects the change immediately — no restart required.
    assert_eq!(
        manager.state.read().unwrap().default_wiki_name(),
        Some("beta")
    );

    // Disk config also reflects the change.
    let saved = llm_wiki_engine::config::load_global(&config_path).unwrap();
    assert_eq!(saved.global.default_wiki, "beta");
}
