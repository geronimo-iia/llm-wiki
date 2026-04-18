use std::collections::BTreeMap;

use llm_wiki::type_registry::TypeRegistry;
use serde_yaml::Value;

fn fm(fields: &[(&str, &str)]) -> BTreeMap<String, Value> {
    fields
        .iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        .collect()
}

// ── known types ───────────────────────────────────────────────────────────────

#[test]
fn knows_built_in_types() {
    let reg = TypeRegistry::new();
    for t in &[
        "page", "concept", "query-result", "section", "paper", "article",
        "documentation", "clipping", "transcript", "note", "data",
        "book-chapter", "thread", "skill", "doc",
    ] {
        assert!(reg.is_known(t), "should know type: {t}");
    }
}

#[test]
fn unknown_type() {
    let reg = TypeRegistry::new();
    assert!(!reg.is_known("alien"));
}

// ── validate ──────────────────────────────────────────────────────────────────

#[test]
fn validate_valid_page() {
    let reg = TypeRegistry::new();
    let warnings = reg
        .validate(&fm(&[("title", "Test"), ("type", "concept")]), "loose")
        .unwrap();
    assert!(warnings.is_empty());
}

#[test]
fn validate_missing_title_errors() {
    let reg = TypeRegistry::new();
    let result = reg.validate(&fm(&[("type", "concept")]), "loose");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("title"));
}

#[test]
fn validate_empty_title_errors() {
    let reg = TypeRegistry::new();
    let result = reg.validate(&fm(&[("title", ""), ("type", "concept")]), "loose");
    assert!(result.is_err());
}

#[test]
fn validate_missing_type_warns() {
    let reg = TypeRegistry::new();
    let warnings = reg.validate(&fm(&[("title", "Test")]), "loose").unwrap();
    assert!(warnings.iter().any(|w| w.contains("type")));
}

#[test]
fn validate_unknown_type_loose_warns() {
    let reg = TypeRegistry::new();
    let warnings = reg
        .validate(&fm(&[("title", "Test"), ("type", "alien")]), "loose")
        .unwrap();
    assert!(warnings.iter().any(|w| w.contains("unknown type")));
}

#[test]
fn validate_unknown_type_strict_errors() {
    let reg = TypeRegistry::new();
    let result = reg.validate(&fm(&[("title", "Test"), ("type", "alien")]), "strict");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown type"));
}

// ── Default ───────────────────────────────────────────────────────────────────

#[test]
fn default_impl() {
    let reg = TypeRegistry::default();
    assert!(reg.is_known("concept"));
}
