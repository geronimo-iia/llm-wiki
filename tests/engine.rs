use std::fs;
use std::path::Path;

use llm_wiki::engine::WikiEngine;
use llm_wiki::git;

fn setup_wiki(dir: &Path, name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    // config lives at <dir>/state/config.toml → state_dir = <dir>/state/
    // indexes will be at <dir>/state/indexes/<name>/
    let config_path = dir.join("state").join("config.toml");
    let wiki_path = dir.join(name);

    llm_wiki::spaces::create(&wiki_path, name, None, false, true, &config_path, None).unwrap();

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

    assert_eq!(engine.default_wiki_name(), "test");
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

    assert_eq!(engine.resolve_wiki_name(None), "research");
    assert_eq!(engine.resolve_wiki_name(Some("other")), "other");
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

    llm_wiki::spaces::create(
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
    llm_wiki::git::commit(&wiki_path, "add skill page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();
    let space = engine.space("skills").unwrap();

    let expected_wiki_root = wiki_path.canonicalize().unwrap().join("skills");
    assert_eq!(space.wiki_root, expected_wiki_root);
}

#[test]
fn engine_indexes_custom_wiki_root_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let fixture_path = std::path::Path::new("tests/fixtures/wikis/alt-root");

    llm_wiki::spaces::register_existing(
        fixture_path,
        "alt-root",
        None,
        None,
        &config_path,
    )
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

    llm_wiki::spaces::create(
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
    llm_wiki::git::commit(&wiki_path, "add page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let result = llm_wiki::ops::content_read(&engine, "wiki://skills/bootstrap", None, false, false)
        .unwrap();
    match result {
        llm_wiki::ops::ContentReadResult::Page(text) => {
            assert!(text.contains("Bootstrap"));
        }
        _ => panic!("expected Page result"),
    }
}
