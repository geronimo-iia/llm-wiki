use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;
use std::fs;

#[test]
fn suggest_returns_candidates_with_shared_tags() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    // Add a paper with the same tag as moe
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("sources")).unwrap();
    fs::write(
        wiki_root.join("sources/paper-a.md"),
        "---\ntitle: \"Paper A\"\ntype: paper\nstatus: active\ntags: [ml]\n---\n\nA paper about ML.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add paper").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    {
        let engine = manager.state.read().unwrap();
        ops::ingest(&engine, &manager, "sources/paper-a.md", false, "test").unwrap();
    }

    let engine = manager.state.read().unwrap();
    let suggestions = ops::suggest(&engine, "concepts/moe", None, None).unwrap();
    assert!(
        suggestions.iter().any(|s| s.slug == "sources/paper-a"),
        "should suggest paper with shared tag 'ml', got: {:?}",
        suggestions.iter().map(|s| &s.slug).collect::<Vec<_>>()
    );
}

#[test]
fn suggest_excludes_already_linked() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    // transformer has [[concepts/moe]] in body — moe should not be suggested for transformer
    let suggestions = ops::suggest(&engine, "concepts/transformer", None, None).unwrap();
    assert!(
        !suggestions.iter().any(|s| s.slug == "concepts/moe"),
        "already-linked page should not be suggested"
    );
}

#[test]
fn suggest_respects_limit() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let suggestions = ops::suggest(&engine, "concepts/moe", None, Some(1)).unwrap();
    assert!(suggestions.len() <= 1);
}

#[test]
fn suggest_on_empty_wiki() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("empty");
    llm_wiki::spaces::create(&wiki_path, "empty", None, false, true, &config_path).unwrap();

    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("concepts")).unwrap();
    fs::write(
        wiki_root.join("concepts/lonely.md"),
        "---\ntitle: \"Lonely\"\ntype: concept\nstatus: active\n---\n\nAlone.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add page").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    {
        let engine = manager.state.read().unwrap();
        ops::ingest(&engine, &manager, "concepts/lonely.md", false, "empty").unwrap();
    }

    let engine = manager.state.read().unwrap();
    let suggestions = ops::suggest(&engine, "concepts/lonely", None, None).unwrap();
    // Single page wiki — no suggestions possible
    assert!(suggestions.is_empty());
}

#[test]
fn suggest_has_field_suggestion() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");
    fs::create_dir_all(wiki_root.join("sources")).unwrap();
    fs::write(
        wiki_root.join("sources/paper-b.md"),
        "---\ntitle: \"Paper B\"\ntype: paper\nstatus: active\ntags: [ml]\n---\n\nAnother ML paper.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add paper").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    {
        let engine = manager.state.read().unwrap();
        ops::ingest(&engine, &manager, "sources/paper-b.md", false, "test").unwrap();
    }

    let engine = manager.state.read().unwrap();
    let suggestions = ops::suggest(&engine, "concepts/moe", None, None).unwrap();
    // A paper suggested for a concept should have field "sources"
    if let Some(paper_suggestion) = suggestions.iter().find(|s| s.r#type == "paper") {
        assert_eq!(
            paper_suggestion.field, "sources",
            "paper suggested for concept should use 'sources' field"
        );
    }
}
