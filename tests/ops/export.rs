use super::helpers::setup_wiki;
use llm_wiki::engine::WikiEngine;
use llm_wiki::ops;
use llm_wiki::ops::{ExportFormat, ExportOptions};

// ── llms-txt ──────────────────────────────────────────────────────────────────

#[test]
fn export_llms_txt_writes_file_and_reports() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("llms.txt".into()),
            format: ExportFormat::LlmsTxt,
            include_archived: false,
        },
    )
    .unwrap();

    assert_eq!(report.format, "llms-txt");
    assert!(report.pages_written >= 2);
    assert!(report.bytes > 0);

    let content = std::fs::read_to_string(&report.path).unwrap();
    assert!(content.contains("# test"));
    assert!(content.contains("## concept"));
    assert!(content.contains("wiki://test/concepts/moe"));
}

// ── llms-full ─────────────────────────────────────────────────────────────────

#[test]
fn export_llms_full_includes_page_bodies() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("llms-full.txt".into()),
            format: ExportFormat::LlmsFull,
            include_archived: false,
        },
    )
    .unwrap();

    let content = std::fs::read_to_string(&report.path).unwrap();
    // Each page entry is preceded by ---
    assert!(content.contains("---\n\n# ["));
    // Body text from helpers::setup_wiki
    assert!(content.contains("Mixture of Experts"));
}

// ── json ──────────────────────────────────────────────────────────────────────

#[test]
fn export_json_produces_valid_json_array() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("wiki.json".into()),
            format: ExportFormat::Json,
            include_archived: false,
        },
    )
    .unwrap();

    let content = std::fs::read_to_string(&report.path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed.is_array());
    let arr = parsed.as_array().unwrap();
    assert!(arr.len() >= 2);
    let first = &arr[0];
    assert!(first.get("slug").is_some());
    assert!(first.get("title").is_some());
    assert!(first.get("type").is_some());
    assert!(first.get("body").is_some());
}

// ── path resolution ───────────────────────────────────────────────────────────

#[test]
fn export_default_path_resolves_to_wiki_root() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: None,
            format: ExportFormat::LlmsTxt,
            include_archived: false,
        },
    )
    .unwrap();

    // Default path is llms.txt relative to wiki root
    assert!(report.path.ends_with("llms.txt"));
    assert!(std::path::PathBuf::from(&report.path).exists());
}

// ── status filter ─────────────────────────────────────────────────────────────

#[test]
fn export_excludes_archived_by_default() {
    use llm_wiki::git;
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    // Add an archived page
    fs::write(
        wiki_root.join("concepts/archived-page.md"),
        "---\ntitle: \"Archived\"\ntype: concept\nstatus: archived\n---\n\nOld content.\n",
    )
    .unwrap();
    git::commit(&wiki_path, "add archived").unwrap();

    // Rebuild the index with the new page
    let manager = WikiEngine::build(&config_path).unwrap();
    {
        let wiki_name = "test".to_string();
        ops::index_rebuild(&manager, &wiki_name).unwrap();
    }
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("out.txt".into()),
            format: ExportFormat::LlmsTxt,
            include_archived: false,
        },
    )
    .unwrap();
    let content = std::fs::read_to_string(&report.path).unwrap();
    assert!(!content.contains("archived-page"));

    let report_all = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("out-all.txt".into()),
            format: ExportFormat::LlmsTxt,
            include_archived: true,
        },
    )
    .unwrap();
    let content_all = std::fs::read_to_string(&report_all.path).unwrap();
    assert!(content_all.contains("archived-page"));
}

// ── custom frontmatter fields ─────────────────────────────────────────────────

#[test]
fn export_json_includes_custom_frontmatter_fields() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let wiki_path = dir.path().join("test");
    let wiki_root = wiki_path.join("wiki");

    std::fs::write(
        wiki_path.join("schemas/decision.json"),
        r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "x-wiki-types": { "decision": "A decision record" },
  "type": "object",
  "required": ["title", "type"],
  "properties": {
    "title": { "type": "string" },
    "type": { "type": "string" },
    "created": { "type": "string" },
    "reference": { "type": "string" },
    "deciders": { "type": "array", "items": { "type": "string" } },
    "priority": { "type": "number" }
  },
  "additionalProperties": true
}"#,
    )
    .unwrap();

    std::fs::write(
        wiki_root.join("concepts/adr-1.md"),
        "---\ntitle: \"ADR 1\"\ntype: decision\nstatus: active\ncreated: 2026-07-19\nreference: DEC-001\ndeciders: [alice, bob]\npriority: 3\n---\n\nDecision body.\n",
    )
    .unwrap();
    llm_wiki::git::commit(&wiki_path, "add decision").unwrap();

    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("wiki.json".into()),
            format: ExportFormat::Json,
            include_archived: false,
        },
    )
    .unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report.path).unwrap()).unwrap();
    let pages = json.as_array().unwrap();
    let adr = pages
        .iter()
        .find(|p| p["slug"] == "concepts/adr-1")
        .unwrap();

    let fm = adr["frontmatter"]
        .as_object()
        .expect("frontmatter object must be present");
    assert_eq!(fm["created"], "2026-07-19");
    assert_eq!(fm["reference"], "DEC-001");
    assert_eq!(fm["deciders"], serde_json::json!(["alice", "bob"]));
    assert_eq!(fm["priority"], 3);
    for surfaced in ["title", "type", "status", "summary", "confidence", "id"] {
        assert!(
            fm.get(surfaced).is_none(),
            "{surfaced} must not be repeated inside frontmatter"
        );
    }
    assert_eq!(adr["title"], "ADR 1");
    assert_eq!(adr["type"], "decision");
    assert_eq!(adr["status"], "active");
    assert!(adr["body"].as_str().unwrap().contains("Decision body."));
}

#[test]
fn export_json_omits_frontmatter_when_no_extra_fields() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = setup_wiki(dir.path(), "test");
    let manager = WikiEngine::build(&config_path).unwrap();
    let engine = manager.state.read().unwrap();

    let report = ops::export(
        &engine,
        &ExportOptions {
            wiki: "test".into(),
            path: Some("wiki.json".into()),
            format: ExportFormat::Json,
            include_archived: false,
        },
    )
    .unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report.path).unwrap()).unwrap();
    let pages = json.as_array().unwrap();
    // transformer.md has no extra frontmatter fields — key must be absent
    let transformer = pages
        .iter()
        .find(|p| p["slug"] == "concepts/transformer")
        .unwrap();
    assert!(
        transformer.get("frontmatter").is_none(),
        "frontmatter key must be absent when page has no extra fields"
    );
}
