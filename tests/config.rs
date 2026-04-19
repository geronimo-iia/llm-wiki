use std::fs;

use llm_wiki::config::*;

// ── load_global ───────────────────────────────────────────────────────────────

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
fn load_global_returns_defaults_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let config = load_global(&path).unwrap();
    assert_eq!(config.defaults.search_top_k, 10);
    assert_eq!(config.defaults.output_format, "text");
    assert_eq!(config.index.memory_budget_mb, 50);
    assert_eq!(config.index.tokenizer, "en_stem");
}

#[test]
fn load_global_returns_error_on_malformed_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "this is not valid toml [[[").unwrap();
    assert!(load_global(&path).is_err());
}

// ── load_wiki ─────────────────────────────────────────────────────────────────

#[test]
fn load_wiki_returns_defaults_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config = load_wiki(dir.path()).unwrap();
    assert_eq!(config.name, "");
    assert!(config.defaults.is_none());
    assert!(config.graph.is_none());
}

#[test]
fn load_wiki_parses_ingest_auto_commit() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("wiki.toml"),
        "name = \"test\"\n\n[ingest]\nauto_commit = false\n",
    )
    .unwrap();
    let config = load_wiki(dir.path()).unwrap();
    assert!(!config.ingest.unwrap().auto_commit);
}

// ── resolve ───────────────────────────────────────────────────────────────────

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
        ..Default::default()
    };
    let resolved = resolve(&global, &WikiConfig::default());
    assert_eq!(resolved.defaults.search_top_k, 10);
}

#[test]
fn resolve_per_wiki_overrides_graph() {
    let global = GlobalConfig {
        graph: GraphConfig {
            depth: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    let per_wiki = WikiConfig {
        graph: Some(GraphConfig {
            depth: 5,
            ..Default::default()
        }),
        ..Default::default()
    };
    let resolved = resolve(&global, &per_wiki);
    assert_eq!(resolved.graph.depth, 5);
}

#[test]
fn resolve_global_only_sections_always_from_global() {
    let global = GlobalConfig {
        index: IndexConfig {
            memory_budget_mb: 100,
            tokenizer: "default".into(),
            ..Default::default()
        },
        serve: ServeConfig {
            sse_port: 9090,
            ..Default::default()
        },
        ..Default::default()
    };
    let resolved = resolve(&global, &WikiConfig::default());
    assert_eq!(resolved.index.memory_budget_mb, 100);
    assert_eq!(resolved.index.tokenizer, "default");
    assert_eq!(resolved.serve.sse_port, 9090);
}

#[test]
fn resolve_per_wiki_overrides_ingest() {
    let global = GlobalConfig {
        ingest: IngestConfig { auto_commit: true },
        ..Default::default()
    };
    let per_wiki = WikiConfig {
        ingest: Some(IngestConfig { auto_commit: false }),
        ..Default::default()
    };
    let resolved = resolve(&global, &per_wiki);
    assert!(!resolved.ingest.auto_commit);
}

// ── save / roundtrip ──────────────────────────────────────────────────────────

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
        graph: Some(GraphConfig {
            depth: 5,
            ..Default::default()
        }),
        ..Default::default()
    };
    save_wiki(&cfg, dir.path()).unwrap();

    let loaded = load_wiki(dir.path()).unwrap();
    assert_eq!(loaded.name, "test");
    assert_eq!(loaded.description, "A test wiki");
    assert_eq!(loaded.defaults.unwrap().search_top_k, 25);
    assert_eq!(loaded.graph.unwrap().depth, 5);
}

#[test]
fn save_global_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("deep/nested/config.toml");
    let config = GlobalConfig::default();
    save_global(&config, &path).unwrap();
    assert!(path.exists());
}

// ── set_global_config_value ───────────────────────────────────────────────────

#[test]
fn set_global_sets_defaults_key() {
    let mut g = GlobalConfig::default();
    set_global_config_value(&mut g, "defaults.search_top_k", "30").unwrap();
    assert_eq!(g.defaults.search_top_k, 30);
}

#[test]
fn set_global_sets_output_format() {
    let mut g = GlobalConfig::default();
    set_global_config_value(&mut g, "defaults.output_format", "json").unwrap();
    assert_eq!(g.defaults.output_format, "json");
}

#[test]
fn set_global_sets_index_keys() {
    let mut g = GlobalConfig::default();
    set_global_config_value(&mut g, "index.memory_budget_mb", "100").unwrap();
    set_global_config_value(&mut g, "index.tokenizer", "default").unwrap();
    assert_eq!(g.index.memory_budget_mb, 100);
    assert_eq!(g.index.tokenizer, "default");
}

#[test]
fn set_global_sets_logging_keys() {
    let mut g = GlobalConfig::default();
    set_global_config_value(&mut g, "logging.log_path", "/tmp/logs").unwrap();
    set_global_config_value(&mut g, "logging.log_rotation", "hourly").unwrap();
    set_global_config_value(&mut g, "logging.log_max_files", "14").unwrap();
    set_global_config_value(&mut g, "logging.log_format", "json").unwrap();
    assert_eq!(g.logging.log_path, "/tmp/logs");
    assert_eq!(g.logging.log_rotation, "hourly");
    assert_eq!(g.logging.log_max_files, 14);
    assert_eq!(g.logging.log_format, "json");
}

#[test]
fn set_global_sets_serve_keys() {
    let mut g = GlobalConfig::default();
    set_global_config_value(&mut g, "serve.max_restarts", "5").unwrap();
    set_global_config_value(&mut g, "serve.restart_backoff", "3").unwrap();
    set_global_config_value(&mut g, "serve.heartbeat_secs", "30").unwrap();
    assert_eq!(g.serve.max_restarts, 5);
    assert_eq!(g.serve.restart_backoff, 3);
    assert_eq!(g.serve.heartbeat_secs, 30);
}

#[test]
fn set_global_rejects_unknown_key() {
    let mut g = GlobalConfig::default();
    assert!(set_global_config_value(&mut g, "nonexistent.key", "v").is_err());
}

// ── set_wiki_config_value ─────────────────────────────────────────────────────

#[test]
fn set_wiki_sets_defaults_key() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "defaults.search_top_k", "42").unwrap();
    assert_eq!(cfg.defaults.unwrap().search_top_k, 42);
}

#[test]
fn set_wiki_sets_output_format() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "defaults.output_format", "json").unwrap();
    assert_eq!(cfg.defaults.unwrap().output_format, "json");
}

#[test]
fn set_wiki_sets_graph_keys() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "graph.format", "dot").unwrap();
    set_wiki_config_value(&mut cfg, "graph.depth", "5").unwrap();
    let g = cfg.graph.unwrap();
    assert_eq!(g.format, "dot");
    assert_eq!(g.depth, 5);
}

#[test]
fn set_wiki_sets_validation_key() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "validation.type_strictness", "strict").unwrap();
    assert_eq!(cfg.validation.unwrap().type_strictness, "strict");
}

#[test]
fn set_wiki_sets_read_no_frontmatter() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "read.no_frontmatter", "true").unwrap();
    assert!(cfg.read.unwrap().no_frontmatter);
}

#[test]
fn set_wiki_sets_ingest_auto_commit() {
    let mut cfg = WikiConfig::default();
    set_wiki_config_value(&mut cfg, "ingest.auto_commit", "false").unwrap();
    assert!(!cfg.ingest.unwrap().auto_commit);
}

#[test]
fn set_wiki_rejects_global_only_keys() {
    let mut cfg = WikiConfig::default();
    for key in &[
        "serve.sse",
        "serve.sse_port",
        "serve.acp",
        "serve.max_restarts",
        "serve.restart_backoff",
        "serve.heartbeat_secs",
        "index.auto_rebuild",
        "index.auto_recovery",
        "index.memory_budget_mb",
        "index.tokenizer",
        "logging.log_path",
        "logging.log_rotation",
        "logging.log_max_files",
        "logging.log_format",
        "global.default_wiki",
    ] {
        let result = set_wiki_config_value(&mut cfg, key, "x");
        assert!(result.is_err(), "expected error for key: {key}");
        assert!(
            result.unwrap_err().to_string().contains("global-only"),
            "expected global-only error for key: {key}"
        );
    }
}

#[test]
fn set_wiki_rejects_unknown_key() {
    let mut cfg = WikiConfig::default();
    let result = set_wiki_config_value(&mut cfg, "nonexistent.key", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("unknown key"));
}

// ── Defaults ──────────────────────────────────────────────────────────────────

#[test]
fn defaults_output_format() {
    assert_eq!(Defaults::default().output_format, "text");
}

#[test]
fn index_config_defaults() {
    let cfg = IndexConfig::default();
    assert!(!cfg.auto_rebuild);
    assert!(cfg.auto_recovery);
    assert_eq!(cfg.memory_budget_mb, 50);
    assert_eq!(cfg.tokenizer, "en_stem");
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
fn serve_config_defaults() {
    let cfg = ServeConfig::default();
    assert_eq!(cfg.max_restarts, 10);
    assert_eq!(cfg.restart_backoff, 1);
    assert_eq!(cfg.heartbeat_secs, 60);
}

#[test]
fn ingest_config_defaults() {
    assert!(IngestConfig::default().auto_commit);
}

// ── get_config_value ──────────────────────────────────────────────────────────

#[test]
fn get_config_value_reads_resolved_keys() {
    let global = GlobalConfig {
        defaults: Defaults {
            search_top_k: 42,
            output_format: "json".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let resolved = resolve(&global, &WikiConfig::default());

    assert_eq!(
        get_config_value(&resolved, &global, "defaults.search_top_k"),
        "42"
    );
    assert_eq!(
        get_config_value(&resolved, &global, "defaults.output_format"),
        "json"
    );
}

#[test]
fn get_config_value_reads_global_only_keys() {
    let global = GlobalConfig {
        index: IndexConfig {
            memory_budget_mb: 100,
            tokenizer: "default".into(),
            ..Default::default()
        },
        serve: ServeConfig {
            sse_port: 9090,
            ..Default::default()
        },
        logging: LoggingConfig {
            log_format: "json".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let resolved = resolve(&global, &WikiConfig::default());

    assert_eq!(
        get_config_value(&resolved, &global, "index.memory_budget_mb"),
        "100"
    );
    assert_eq!(
        get_config_value(&resolved, &global, "index.tokenizer"),
        "default"
    );
    assert_eq!(
        get_config_value(&resolved, &global, "serve.sse_port"),
        "9090"
    );
    assert_eq!(
        get_config_value(&resolved, &global, "logging.log_format"),
        "json"
    );
}

#[test]
fn get_config_value_returns_per_wiki_override() {
    let global = GlobalConfig {
        defaults: Defaults {
            search_top_k: 10,
            ..Default::default()
        },
        ..Default::default()
    };
    let per_wiki = WikiConfig {
        defaults: Some(Defaults {
            search_top_k: 25,
            ..Default::default()
        }),
        ..Default::default()
    };
    let resolved = resolve(&global, &per_wiki);

    assert_eq!(
        get_config_value(&resolved, &global, "defaults.search_top_k"),
        "25"
    );
}

#[test]
fn get_config_value_unknown_key() {
    let global = GlobalConfig::default();
    let resolved = resolve(&global, &WikiConfig::default());

    assert!(get_config_value(&resolved, &global, "nonexistent.key").contains("unknown"));
}

// ── types registry in wiki.toml ──────────────────────────────────────────────

#[test]
fn load_wiki_parses_types() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("wiki.toml"),
        r#"
name = "research"
description = "ML research"

[types.default]
schema = "schemas/base.json"
description = "Fallback for unrecognized types"

[types.concept]
schema = "schemas/concept.json"
description = "Synthesized knowledge"

[types.paper]
schema = "schemas/paper.json"
description = "Academic source"
"#,
    )
    .unwrap();

    let config = load_wiki(dir.path()).unwrap();
    assert_eq!(config.name, "research");
    assert_eq!(config.types.len(), 3);
    assert_eq!(config.types["default"].schema, "schemas/base.json");
    assert_eq!(config.types["concept"].schema, "schemas/concept.json");
    assert_eq!(
        config.types["concept"].description,
        "Synthesized knowledge"
    );
    assert_eq!(config.types["paper"].schema, "schemas/paper.json");
}

#[test]
fn load_wiki_empty_types_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let config = load_wiki(dir.path()).unwrap();
    assert!(config.types.is_empty());
}

#[test]
fn save_wiki_roundtrips_types() {
    let dir = tempfile::tempdir().unwrap();
    let mut types = std::collections::HashMap::new();
    types.insert(
        "concept".into(),
        TypeEntry {
            schema: "schemas/concept.json".into(),
            description: "Synthesized knowledge".into(),
        },
    );
    let cfg = WikiConfig {
        name: "test".into(),
        types,
        ..Default::default()
    };
    save_wiki(&cfg, dir.path()).unwrap();

    let loaded = load_wiki(dir.path()).unwrap();
    assert_eq!(loaded.types.len(), 1);
    assert_eq!(loaded.types["concept"].schema, "schemas/concept.json");
    assert_eq!(
        loaded.types["concept"].description,
        "Synthesized knowledge"
    );
}
