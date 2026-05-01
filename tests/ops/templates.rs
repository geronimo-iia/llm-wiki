use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;
use std::fs;

// ── Body templates ────────────────────────────────────────────────────────────

#[test]
fn content_new_concept_has_body_template() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    ops::content_new(
        &engine,
        "concepts/new-one",
        None,
        false,
        false,
        None,
        Some("concept"),
    )
    .unwrap();

    let space = engine.space("test").unwrap();
    let content = fs::read_to_string(space.wiki_root.join("concepts/new-one.md")).unwrap();
    assert!(
        content.contains("## Overview"),
        "concept template should have Overview section"
    );
    assert!(
        content.contains("## Key ideas"),
        "concept template should have Key ideas section"
    );
}

#[test]
fn content_new_section_has_body_template() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    ops::content_new(&engine, "topics", None, true, false, None, None).unwrap();

    let space = engine.space("test").unwrap();
    let content = fs::read_to_string(space.wiki_root.join("topics/index.md")).unwrap();
    assert!(
        content.contains("## Overview"),
        "section template should have Overview section"
    );
}

#[test]
fn custom_template_overrides_embedded() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");

    // Write a custom concept template
    let wiki_path = dir.path().join("test");
    fs::write(
        wiki_path.join("schemas/concept.md"),
        "## Custom Section\n\nThis is custom.\n",
    )
    .unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    ops::content_new(
        &engine,
        "concepts/custom-tmpl",
        None,
        false,
        false,
        None,
        Some("concept"),
    )
    .unwrap();

    let space = engine.space("test").unwrap();
    let content = fs::read_to_string(space.wiki_root.join("concepts/custom-tmpl.md")).unwrap();
    assert!(
        content.contains("## Custom Section"),
        "custom template should override embedded"
    );
    assert!(
        !content.contains("## Key ideas"),
        "embedded template should not appear"
    );
}

#[test]
fn missing_template_falls_back_to_empty_body() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    // "skill" type has no body template
    ops::content_new(
        &engine,
        "skills/test-skill",
        None,
        false,
        false,
        None,
        Some("skill"),
    )
    .unwrap();

    let space = engine.space("test").unwrap();
    let content = fs::read_to_string(space.wiki_root.join("skills/test-skill.md")).unwrap();
    // Should have frontmatter but no body sections
    assert!(content.contains("type: skill"));
    assert!(!content.contains("## "), "no body template for skill type");
}

#[test]
fn spaces_create_writes_template_files() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("state").join("config.toml");
    let wiki_path = dir.path().join("wiki");

    llm_wiki::spaces::create(&wiki_path, "test", None, false, true, &config_path, None).unwrap();

    assert!(wiki_path.join("schemas/concept.md").exists());
    assert!(wiki_path.join("schemas/paper.md").exists());
    assert!(wiki_path.join("schemas/doc.md").exists());
    assert!(wiki_path.join("schemas/section.md").exists());
    assert!(wiki_path.join("schemas/query-result.md").exists());
}
