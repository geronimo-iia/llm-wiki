use std::fs;

use llm_wiki::config::*;

#[test]
fn load_global_parses_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        r#"
[global]
default_wiki = "research"

[[wikis]]
name = "research"
path = "/tmp/research"

[defaults]
search_top_k = 15

[validation]
type_strictness = "strict"
"#,
    )
    .unwrap();

    let config = load_global(&path).unwrap();
    assert_eq!(config.global.default_wiki, "research");
    assert_eq!(config.wikis.len(), 1);
    assert_eq!(config.wikis[0].name, "research");
    assert_eq!(config.defaults.search_top_k, 15);
    assert_eq!(config.validation.type_strictness, "strict");
}

#[test]
fn load_global_returns_error_on_malformed_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "this is not valid toml [[[").unwrap();

    let result = load_global(&path);
    assert!(result.is_err());
}

#[test]
fn resolve_per_wiki_overrides_global() {
    let global = GlobalConfig {
        defaults: Defaults {
            search_top_k: 10,
            ..Default::default()
        },
        validation: ValidationConfig {
            type_strictness: "loose".into(),
        },
        ..Default::default()
    };

    let per_wiki = WikiConfig {
        defaults: Some(Defaults {
            search_top_k: 25,
            ..Default::default()
        }),
        validation: Some(ValidationConfig {
            type_strictness: "strict".into(),
        }),
        ..Default::default()
    };

    let resolved = resolve(&global, &per_wiki);
    assert_eq!(resolved.defaults.search_top_k, 25);
    assert_eq!(resolved.validation.type_strictness, "strict");
}

#[test]
fn resolve_falls_back_to_global_when_per_wiki_absent() {
    let global = GlobalConfig {
        defaults: Defaults {
            search_top_k: 10,
            ..Default::default()
        },
        validation: ValidationConfig {
            type_strictness: "loose".into(),
        },
        ..Default::default()
    };

    let per_wiki = WikiConfig::default();

    let resolved = resolve(&global, &per_wiki);
    assert_eq!(resolved.defaults.search_top_k, 10);
    assert_eq!(resolved.validation.type_strictness, "loose");
}

#[test]
fn load_schema_parses_custom_types() {
    let dir = tempfile::tempdir().unwrap();
    let schema_path = dir.path().join("schema.md");
    fs::write(
        &schema_path,
        "# Schema\n\n- type: recipe\n- type: tutorial\n",
    )
    .unwrap();

    let schema = load_schema(dir.path()).unwrap();
    assert_eq!(schema.custom_types, vec!["recipe", "tutorial"]);
}

#[test]
fn load_schema_returns_empty_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let schema = load_schema(dir.path()).unwrap();
    assert!(schema.custom_types.is_empty());
}
