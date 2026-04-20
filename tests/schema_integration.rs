use std::fs;
use std::path::Path;

use llm_wiki::config;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;
use llm_wiki::spaces;

fn setup_wiki(dir: &Path) -> std::path::PathBuf {
    let wiki_path = dir.join("wiki-repo");
    let config_path = dir.join("engine").join("config.toml");
    spaces::create(&wiki_path, "test", None, false, true, &config_path).unwrap();
    config_path
}

fn engine(config_path: &Path) -> WikiEngine {
    WikiEngine::build(config_path).unwrap()
}

// ── schema list ───────────────────────────────────────────────────────────────

#[test]
fn schema_list_returns_all_default_types() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let entries = ops::schema_list(&eng, "test").unwrap();
    assert_eq!(entries.len(), 15);

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"default"));
    assert!(names.contains(&"concept"));
    assert!(names.contains(&"skill"));
    assert!(names.contains(&"paper"));
    assert!(names.contains(&"doc"));
    assert!(names.contains(&"section"));
}

// ── schema show ───────────────────────────────────────────────────────────────

#[test]
fn schema_show_returns_valid_json_schema() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let content = ops::schema_show(&eng, "test", "concept").unwrap();
    let schema: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(schema["required"].as_array().unwrap().contains(&serde_json::json!("read_when")));
}

#[test]
fn schema_show_errors_on_unknown_type() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let result = ops::schema_show(&eng, "test", "nonexistent");
    assert!(result.is_err());
}

// ── schema show --template ────────────────────────────────────────────────────

#[test]
fn schema_template_concept_has_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let tmpl = ops::schema_show_template(&eng, "test", "concept").unwrap();
    assert!(tmpl.contains("title:"));
    assert!(tmpl.contains("type: concept"));
    assert!(tmpl.contains("read_when:"));
}

#[test]
fn schema_template_skill_uses_aliased_fields() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let tmpl = ops::schema_show_template(&eng, "test", "skill").unwrap();
    assert!(tmpl.contains("name:"));
    assert!(tmpl.contains("description:"));
    assert!(tmpl.contains("type: skill"));
    // Should NOT contain "title:" — skill uses "name" instead
    assert!(!tmpl.contains("title:"));
}

#[test]
fn schema_template_passes_own_validation() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    // Templates are scaffolds with empty values. Fill required fields
    // to verify the structure is correct for validation.
    let test_cases: &[(&str, &[(&str, &str)])] = &[
        ("concept", &[("title", "Test"), ("read_when", "- \"test\"")]),
        ("paper", &[("title", "Test")]),
        ("doc", &[("title", "Test")]),
        ("section", &[("title", "Test")]),
    ];

    for (type_name, fills) in test_cases {
        let tmpl = ops::schema_show_template(&eng, "test", type_name).unwrap();
        let mut filled = tmpl.clone();
        for (field, value) in *fills {
            let empty = format!("{field}: \"\"");
            let replacement = format!("{field}: \"{value}\"");
            if filled.contains(&empty) {
                filled = filled.replace(&empty, &replacement);
            }
            // Handle array fields
            let empty_arr = format!("{field}:\n  - \"\"");
            let replacement_arr = format!("{field}:\n  {value}");
            if filled.contains(&empty_arr) {
                filled = filled.replace(&empty_arr, &replacement_arr);
            }
        }

        let yaml_content = filled.trim_start_matches("---").trim_end_matches("---").trim();
        let fm: std::collections::BTreeMap<String, serde_yaml::Value> =
            serde_yaml::from_str(yaml_content).unwrap_or_default();

        let space = eng.space("test").unwrap();
        let result = space.type_registry.validate(&fm, "loose");
        assert!(
            result.is_ok(),
            "template for '{type_name}' failed validation: {:?}",
            result.unwrap_err()
        );
    }
}

// ── round-trip: template → write → ingest ─────────────────────────────────────

#[test]
fn roundtrip_template_write_ingest() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let tmpl = ops::schema_show_template(&eng, "test", "concept").unwrap();
    // Fill in required values
    let filled = tmpl
        .replace("title: \"\"", "title: \"Test Concept\"")
        .replace("read_when:\n  - \"\"", "read_when:\n  - \"Testing round-trip\"");

    let content = format!("{filled}\n\n## Body\n\nSome content.\n");

    let space = eng.space("test").unwrap();
    let page_path = space.wiki_root.join("concepts/test.md");
    fs::create_dir_all(page_path.parent().unwrap()).unwrap();
    fs::write(&page_path, &content).unwrap();

    drop(eng);

    let result = ops::ingest(
        &mgr.state.read().unwrap(),
        &mgr,
        "concepts/test.md",
        false,
        "test",
    );
    assert!(result.is_ok(), "ingest failed: {:?}", result.unwrap_err());
}

// ── schema add ────────────────────────────────────────────────────────────────

#[test]
fn schema_add_registers_custom_type() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    // Write a custom schema file
    let custom_schema = dir.path().join("meeting.json");
    fs::write(
        &custom_schema,
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"meeting": "Meeting notes"},
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "attendees": {"type": "array", "items": {"type": "string"}}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();

    let msg = ops::schema_add(&eng, "test", "meeting", &custom_schema).unwrap();
    assert!(msg.contains("copied to"));

    // Verify the schema file was copied
    let space = eng.space("test").unwrap();
    assert!(space.repo_root.join("schemas/meeting.json").exists());
}

// ── schema validate ───────────────────────────────────────────────────────────

#[test]
fn schema_validate_passes_for_default_schemas() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    let issues = ops::schema_validate(&eng, "test", None).unwrap();
    assert!(issues.is_empty(), "unexpected issues: {issues:?}");
}

#[test]
fn schema_validate_catches_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);
    let eng = mgr.state.read().unwrap();

    // Write an invalid JSON file to schemas/
    let space = eng.space("test").unwrap();
    fs::write(
        space.repo_root.join("schemas/broken.json"),
        "not valid json {{{",
    )
    .unwrap();

    let issues = ops::schema_validate(&eng, "test", None).unwrap();
    assert!(issues.iter().any(|i| i.contains("invalid JSON")));
}

// ── schema remove ─────────────────────────────────────────────────────────────

#[test]
fn schema_remove_dry_run_reports_without_changing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);

    let report = ops::schema_remove(&mgr, "test", "concept", false, false, true).unwrap();
    assert!(report.dry_run);
    assert_eq!(report.pages_removed, 0); // no pages indexed yet
}

#[test]
fn schema_remove_cannot_remove_default() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);

    let result = ops::schema_remove(&mgr, "test", "default", false, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("default"));
}

// ── schema change triggers rebuild ────────────────────────────────────────────

#[test]
fn schema_change_makes_index_stale() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path());
    let mgr = engine(&config_path);

    // Index is current after build
    {
        let eng = mgr.state.read().unwrap();
        let space = eng.space("test").unwrap();
        let status = space.index_manager.status(
            &space.repo_root,
        )
        .unwrap();
        assert!(!status.stale, "index should not be stale after build");
    }

    // Modify a schema file
    {
        let eng = mgr.state.read().unwrap();
        let space = eng.space("test").unwrap();
        let schema_path = space.repo_root.join("schemas/concept.json");
        let mut content = fs::read_to_string(&schema_path).unwrap();
        content = content.replace(
            "\"Synthesized knowledge",
            "\"MODIFIED Synthesized knowledge",
        );
        fs::write(&schema_path, content).unwrap();
    }

    // Rebuild engine — new schema_hash should differ
    let mgr2 = engine(&config_path);
    let eng2 = mgr2.state.read().unwrap();
    let space2 = eng2.space("test").unwrap();

    // The old state.toml has the old hash, new registry has new hash
    // So if we check with the OLD hash, it's not stale
    // But if we check with the NEW hash, it IS stale (hash mismatch)
    let status = space2.index_manager.status(
        &space2.repo_root,
    )
    .unwrap();
    assert!(status.stale, "index should be stale with wrong hash");
}
