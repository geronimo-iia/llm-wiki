use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::git;
use llm_wiki::ops;
use std::fs;

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
