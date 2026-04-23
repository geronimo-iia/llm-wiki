use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;
use std::fs;

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
