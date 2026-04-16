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

#[test]
fn save_wiki_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = WikiConfig {
        name: "test".into(),
        description: "A test wiki".into(),
        defaults: Some(Defaults {
            search_top_k: 25,
            ..Default::default()
        }),
        read: None,
        validation: Some(ValidationConfig {
            type_strictness: "strict".into(),
        }),
        lint: None,
    };
    save_wiki(&cfg, dir.path()).unwrap();

    let loaded = load_wiki(dir.path()).unwrap();
    assert_eq!(loaded.name, "test");
    assert_eq!(loaded.description, "A test wiki");
    assert_eq!(loaded.defaults.unwrap().search_top_k, 25);
    assert_eq!(loaded.validation.unwrap().type_strictness, "strict");
}

#[test]
fn set_wiki_config_value_sets_defaults_key() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "defaults.search_top_k", "42").unwrap();
    assert_eq!(cfg.defaults.unwrap().search_top_k, 42);
}

#[test]
fn set_wiki_config_value_sets_validation_key() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "validation.type_strictness", "strict").unwrap();
    assert_eq!(cfg.validation.unwrap().type_strictness, "strict");
}

#[test]
fn set_wiki_config_value_sets_lint_key() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "lint.fix_missing_stubs", "false").unwrap();
    assert_eq!(cfg.lint.unwrap().fix_missing_stubs, false);
}

#[test]
fn set_wiki_config_value_rejects_global_only_key() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "serve.sse", "true");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));
}

#[test]
fn set_wiki_config_value_rejects_unknown_key() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "nonexistent.key", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown key"));
}

#[test]
fn set_global_config_value_sets_key() {
    let mut global = GlobalConfig::default();
    set_global_config_value(&mut global, "defaults.search_top_k", "30").unwrap();
    assert_eq!(global.defaults.search_top_k, 30);
}

#[test]
fn logging_config_defaults() {
    let cfg = LoggingConfig::default();
    assert!(cfg.log_path.ends_with(".llm-wiki/logs"));
    assert_eq!(cfg.log_rotation, "daily");
    assert_eq!(cfg.log_max_files, 7);
    assert_eq!(cfg.log_format, "text");
}

#[test]
fn set_global_config_value_sets_logging_keys() {
    let mut global = GlobalConfig::default();
    set_global_config_value(&mut global, "logging.log_path", "/tmp/logs").unwrap();
    set_global_config_value(&mut global, "logging.log_rotation", "hourly").unwrap();
    set_global_config_value(&mut global, "logging.log_max_files", "14").unwrap();
    set_global_config_value(&mut global, "logging.log_format", "json").unwrap();
    assert_eq!(global.logging.log_path, "/tmp/logs");
    assert_eq!(global.logging.log_rotation, "hourly");
    assert_eq!(global.logging.log_max_files, 14);
    assert_eq!(global.logging.log_format, "json");
}

#[test]
fn set_wiki_config_value_rejects_logging_keys() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "logging.log_path", "/tmp");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));
}

#[test]
fn set_global_config_value_sets_serve_restart_keys() {
    let mut global = GlobalConfig::default();
    set_global_config_value(&mut global, "serve.max_restarts", "5").unwrap();
    set_global_config_value(&mut global, "serve.restart_backoff", "3").unwrap();
    assert_eq!(global.serve.max_restarts, 5);
    assert_eq!(global.serve.restart_backoff, 3);
}

#[test]
fn set_wiki_config_value_rejects_serve_restart_keys() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "serve.max_restarts", "5");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));

    let result = set_wiki_config_value(&mut cfg, "serve.restart_backoff", "3");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));
}

#[test]
fn serve_config_defaults() {
    let cfg = ServeConfig::default();
    assert_eq!(cfg.max_restarts, 10);
    assert_eq!(cfg.restart_backoff, 1);
}

#[test]
fn serve_config_heartbeat_default() {
    let cfg = ServeConfig::default();
    assert_eq!(cfg.heartbeat_secs, 60);
}

#[test]
fn set_global_config_value_sets_heartbeat() {
    let mut global = GlobalConfig::default();
    set_global_config_value(&mut global, "serve.heartbeat_secs", "30").unwrap();
    assert_eq!(global.serve.heartbeat_secs, 30);
}

#[test]
fn set_wiki_config_value_rejects_heartbeat() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "serve.heartbeat_secs", "30");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));
}

#[test]
fn set_wiki_config_value_sets_read_no_frontmatter() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "read.no_frontmatter", "true").unwrap();
    assert_eq!(cfg.read.unwrap().no_frontmatter, true);
}

#[test]
fn resolve_per_wiki_overrides_read_no_frontmatter() {
    let global = GlobalConfig {
        read: ReadConfig {
            no_frontmatter: false,
        },
        ..Default::default()
    };
    let per_wiki = WikiConfig {
        read: Some(ReadConfig {
            no_frontmatter: true,
        }),
        ..Default::default()
    };
    let resolved = resolve(&global, &per_wiki);
    assert_eq!(resolved.read.no_frontmatter, true);
}

#[test]
fn resolve_falls_back_to_global_read_when_per_wiki_absent() {
    let global = GlobalConfig {
        read: ReadConfig {
            no_frontmatter: true,
        },
        ..Default::default()
    };
    let per_wiki = WikiConfig::default();
    let resolved = resolve(&global, &per_wiki);
    assert_eq!(resolved.read.no_frontmatter, true);
}

#[test]
fn index_config_auto_recovery_defaults_true() {
    let cfg = IndexConfig::default();
    assert!(cfg.auto_recovery);
}

#[test]
fn set_global_config_value_sets_auto_recovery() {
    let mut global = GlobalConfig::default();
    set_global_config_value(&mut global, "index.auto_recovery", "false").unwrap();
    assert!(!global.index.auto_recovery);
}

#[test]
fn set_wiki_config_value_rejects_index_keys() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "index.auto_recovery", "false");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));

    let result = set_wiki_config_value(&mut cfg, "index.auto_rebuild", "true");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("global-only"));
}
