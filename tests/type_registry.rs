use std::collections::BTreeMap;
use std::fs;

use llm_wiki::type_registry::SpaceTypeRegistry;
use serde_yaml::Value;

fn fm(fields: &[(&str, &str)]) -> BTreeMap<String, Value> {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        .collect()
}

// ── from_embedded ─────────────────────────────────────────────────────────────

#[test]
fn embedded_knows_all_15_types() {
    let reg = SpaceTypeRegistry::from_embedded();
    for t in &[
        "default", "concept", "query-result", "section", "paper", "article",
        "documentation", "clipping", "transcript", "note", "data",
        "book-chapter", "thread", "skill", "doc",
    ] {
        assert!(reg.is_known(t), "should know type: {t}");
    }
}

#[test]
fn embedded_unknown_type() {
    let reg = SpaceTypeRegistry::from_embedded();
    assert!(!reg.is_known("alien"));
}

#[test]
fn embedded_list_types_returns_15() {
    let reg = SpaceTypeRegistry::from_embedded();
    assert_eq!(reg.list_types().len(), 15);
}

#[test]
fn embedded_skill_has_aliases() {
    let reg = SpaceTypeRegistry::from_embedded();
    let aliases = reg.aliases("skill").expect("skill should have aliases");
    assert_eq!(aliases["name"], "title");
    assert_eq!(aliases["description"], "summary");
    assert_eq!(aliases["when_to_use"], "read_when");
}

#[test]
fn embedded_concept_has_no_aliases() {
    let reg = SpaceTypeRegistry::from_embedded();
    let aliases = reg.aliases("concept").expect("concept should exist");
    assert!(aliases.is_empty());
}

#[test]
fn embedded_schema_hash_is_stable() {
    let r1 = SpaceTypeRegistry::from_embedded();
    let r2 = SpaceTypeRegistry::from_embedded();
    assert_eq!(r1.schema_hash(), r2.schema_hash());
}

#[test]
fn default_impl() {
    let reg = SpaceTypeRegistry::default();
    assert!(reg.is_known("concept"));
}

// ── build from disk ───────────────────────────────────────────────────────────

#[test]
fn build_discovers_types_from_schemas_dir() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Write a minimal schema with x-wiki-types
    fs::write(
        schemas_dir.join("test.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {
                "my-type": "A custom type"
            },
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();

    // Minimal wiki.toml
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let reg = SpaceTypeRegistry::build(dir.path()).unwrap();
    assert!(reg.is_known("my-type"));
}

#[test]
fn build_wiki_toml_override_takes_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Schema A declares "paper"
    fs::write(
        schemas_dir.join("a.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"paper": "From schema A"},
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();

    // Schema B is a custom paper schema
    fs::write(
        schemas_dir.join("b.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["title", "type", "custom_field"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "custom_field": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();

    // wiki.toml overrides "paper" to use schema B
    fs::write(
        dir.path().join("wiki.toml"),
        r#"
name = "test"

[types.paper]
schema = "schemas/b.json"
description = "Custom paper"
"#,
    )
    .unwrap();

    let reg = SpaceTypeRegistry::build(dir.path()).unwrap();

    // paper should now require custom_field (from schema B)
    let valid = fm(&[("title", "Test"), ("type", "paper"), ("custom_field", "yes")]);
    assert!(reg.validate(&valid, "strict").is_ok());

    let missing = fm(&[("title", "Test"), ("type", "paper")]);
    assert!(reg.validate(&missing, "strict").is_err());
}

#[test]
fn build_falls_back_to_embedded_when_no_schemas_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let reg = SpaceTypeRegistry::build(dir.path()).unwrap();
    assert!(reg.is_known("concept"));
    assert_eq!(reg.list_types().len(), 15);
}

// ── validate ──────────────────────────────────────────────────────────────────

#[test]
fn validate_valid_concept() {
    let reg = SpaceTypeRegistry::from_embedded();
    let warnings = reg
        .validate(
            &fm(&[("title", "Test"), ("type", "concept"), ("read_when", "test")]),
            "loose",
        )
        .unwrap();
    // read_when as string instead of list will produce a schema warning in loose mode
    // but title+type are present so no hard error
    assert!(warnings.is_empty() || warnings.iter().all(|w| w.contains("schema validation")));
}

#[test]
fn validate_missing_title_in_strict() {
    let reg = SpaceTypeRegistry::from_embedded();
    let result = reg.validate(&fm(&[("type", "concept")]), "strict");
    assert!(result.is_err());
}

#[test]
fn validate_missing_type_warns() {
    let reg = SpaceTypeRegistry::from_embedded();
    let warnings = reg.validate(&fm(&[("title", "Test")]), "loose").unwrap();
    assert!(warnings.iter().any(|w| w.contains("type")));
}

#[test]
fn validate_unknown_type_loose_warns() {
    let reg = SpaceTypeRegistry::from_embedded();
    let warnings = reg
        .validate(&fm(&[("title", "Test"), ("type", "alien")]), "loose")
        .unwrap();
    assert!(warnings.iter().any(|w| w.contains("unknown type")));
}

#[test]
fn validate_unknown_type_strict_errors() {
    let reg = SpaceTypeRegistry::from_embedded();
    let result = reg.validate(&fm(&[("title", "Test"), ("type", "alien")]), "strict");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown type"));
}

#[test]
fn validate_base_type_accepts_minimal() {
    let reg = SpaceTypeRegistry::from_embedded();
    let warnings = reg
        .validate(&fm(&[("title", "Test"), ("type", "page")]), "loose")
        .unwrap();
    // "page" is unknown, falls back to default — warning about unknown type
    assert!(warnings.iter().any(|w| w.contains("unknown type")));
    // But no schema validation error (default/base accepts title+type)
    assert!(!warnings.iter().any(|w| w.contains("schema validation")));
}

// ── base schema invariant ─────────────────────────────────────────────────────

#[test]
fn build_injects_embedded_default_when_no_base_json() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Only a custom type, no base.json
    fs::write(
        schemas_dir.join("custom.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"custom": "A custom type"},
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let reg = SpaceTypeRegistry::build(dir.path()).unwrap();
    assert!(reg.is_known("default"), "default type should be injected from embedded");
    assert!(reg.is_known("custom"));
}

#[test]
fn build_accepts_valid_custom_base() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Custom base.json that requires title + type + an extra field
    fs::write(
        schemas_dir.join("base.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"default": "Custom fallback"},
            "type": "object",
            "required": ["title", "type", "owner"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "owner": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let reg = SpaceTypeRegistry::build(dir.path()).unwrap();
    assert!(reg.is_known("default"));
}

#[test]
fn build_rejects_base_missing_title_requirement() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // base.json that only requires "type" — missing "title"
    fs::write(
        schemas_dir.join("base.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"default": "Bad fallback"},
            "type": "object",
            "required": ["type"],
            "properties": {
                "type": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let result = SpaceTypeRegistry::build(dir.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("title"), "error should mention title: {msg}");
}

#[test]
fn build_rejects_base_missing_type_requirement() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // base.json that only requires "title" — missing "type"
    fs::write(
        schemas_dir.join("base.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"default": "Bad fallback"},
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let result = SpaceTypeRegistry::build(dir.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("type"), "error should mention type: {msg}");
}
