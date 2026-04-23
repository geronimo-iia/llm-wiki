use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;

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
