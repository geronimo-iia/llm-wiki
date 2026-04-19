use std::fs;

use llm_wiki::index_schema::IndexSchema;

// ── build (hardcoded, backward compat) ────────────────────────────────────────

#[test]
fn build_has_fixed_fields() {
    let is = IndexSchema::build("en_stem");
    for name in &["slug", "uri", "body", "body_links", "title", "summary", "type", "status", "tags"] {
        assert!(is.try_field(name).is_some(), "missing field: {name}");
    }
}

// ── build_from_schemas with embedded defaults ─────────────────────────────────

#[test]
fn from_embedded_schemas_has_fixed_fields() {
    let dir = tempfile::tempdir().unwrap();
    // No schemas/ dir → falls back to embedded
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    for name in &["slug", "uri", "body", "body_links"] {
        assert!(is.try_field(name).is_some(), "missing fixed field: {name}");
    }
}

#[test]
fn from_embedded_schemas_has_base_fields() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    for name in &["title", "summary", "type", "status", "tags", "owner", "superseded_by"] {
        assert!(is.try_field(name).is_some(), "missing base field: {name}");
    }
}

#[test]
fn from_embedded_schemas_has_concept_fields() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    for name in &["read_when", "tldr", "sources", "concepts", "confidence", "claims"] {
        assert!(is.try_field(name).is_some(), "missing concept field: {name}");
    }
}

#[test]
fn from_embedded_schemas_has_skill_fields() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    // document_refs from skill.json
    assert!(is.try_field("document_refs").is_some());
}

#[test]
fn from_embedded_schemas_skips_aliased_fields() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    // skill.json has name/description/when_to_use aliased to title/summary/read_when
    // The aliased source fields should NOT be in the index
    assert!(is.try_field("name").is_none(), "aliased field 'name' should not be indexed");
    assert!(is.try_field("description").is_none(), "aliased field 'description' should not be indexed");
    assert!(is.try_field("when_to_use").is_none(), "aliased field 'when_to_use' should not be indexed");
    // But their canonical targets should exist
    assert!(is.try_field("title").is_some());
    assert!(is.try_field("summary").is_some());
    assert!(is.try_field("read_when").is_some());
}

// ── build_from_schemas with custom schemas on disk ────────────────────────────

#[test]
fn from_disk_discovers_custom_fields() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    fs::write(
        schemas_dir.join("custom.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"custom": "A custom type"},
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "priority": {"type": "string", "enum": ["low", "medium", "high"]},
                "assignee": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    assert!(is.try_field("priority").is_some(), "custom enum field should exist");
    assert!(is.try_field("assignee").is_some(), "custom string field should exist");
}

#[test]
fn enum_fields_are_keywords() {
    // We can't directly inspect the tantivy field type, but we can verify
    // the field exists. The classification logic is tested via the classify
    // function behavior — enum fields get STRING | STORED.
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    fs::write(
        schemas_dir.join("test.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"test-type": "Test"},
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "level": {"type": "string", "enum": ["a", "b", "c"]}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    assert!(is.try_field("level").is_some());
}

#[test]
fn wiki_toml_override_schema_adds_fields() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Base schema only
    fs::write(
        schemas_dir.join("base.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "x-wiki-types": {"default": "Fallback"},
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

    // Override schema in a different location
    fs::write(
        schemas_dir.join("extended.json"),
        r#"{
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["title", "type"],
            "properties": {
                "title": {"type": "string"},
                "type": {"type": "string"},
                "extra_field": {"type": "string"}
            },
            "additionalProperties": true
        }"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("wiki.toml"),
        r#"
name = "test"

[types.special]
schema = "schemas/extended.json"
description = "Special type"
"#,
    )
    .unwrap();

    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    assert!(is.try_field("extra_field").is_some(), "override schema field should be indexed");
}

// ── deduplication ─────────────────────────────────────────────────────────────

#[test]
fn duplicate_fields_across_schemas_are_deduplicated() {
    let dir = tempfile::tempdir().unwrap();
    let schemas_dir = dir.path().join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();

    // Two schemas both define "title" and "type"
    for name in &["a.json", "b.json"] {
        fs::write(
            schemas_dir.join(name),
            format!(r#"{{
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "x-wiki-types": {{"{name}-type": "Type from {name}"}},
                "type": "object",
                "required": ["title", "type"],
                "properties": {{
                    "title": {{"type": "string"}},
                    "type": {{"type": "string"}}
                }},
                "additionalProperties": true
            }}"#),
        )
        .unwrap();
    }
    fs::write(dir.path().join("wiki.toml"), "name = \"test\"\n").unwrap();

    // Should not panic on duplicate field names
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
    assert!(is.try_field("title").is_some());
}

// ── build from actual repo schemas/ folder ────────────────────────────────────

fn setup_wiki_with_repo_schemas(dir: &std::path::Path) {
    let schemas_dir = dir.join("schemas");
    fs::create_dir_all(&schemas_dir).unwrap();
    for (filename, content) in llm_wiki::default_schemas::default_schemas() {
        fs::write(schemas_dir.join(filename), content).unwrap();
    }
    fs::write(dir.join("wiki.toml"), "name = \"test\"\n").unwrap();
}

#[test]
fn repo_schemas_build_successfully() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();
}

#[test]
fn repo_schemas_have_all_base_fields() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // From base.json
    for name in &["title", "type", "summary", "status", "last_updated", "tags", "owner", "superseded_by"] {
        assert!(is.try_field(name).is_some(), "missing base field from repo schemas: {name}");
    }
}

#[test]
fn repo_schemas_have_all_concept_fields() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // From concept.json
    for name in &["read_when", "tldr", "sources", "concepts", "confidence", "claims"] {
        assert!(is.try_field(name).is_some(), "missing concept field from repo schemas: {name}");
    }
}

#[test]
fn repo_schemas_have_all_doc_fields() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // doc.json adds read_when and sources (already covered by concept),
    // but verify they exist
    assert!(is.try_field("read_when").is_some());
    assert!(is.try_field("sources").is_some());
}

#[test]
fn repo_schemas_have_skill_specific_fields() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // From skill.json — fields that are NOT aliased
    assert!(is.try_field("document_refs").is_some(), "missing skill field: document_refs");
    assert!(is.try_field("disable-model-invocation").is_some(), "missing skill field: disable-model-invocation");
    assert!(is.try_field("user-invocable").is_some(), "missing skill field: user-invocable");
    assert!(is.try_field("allowed-tools").is_some(), "missing skill field: allowed-tools");
    assert!(is.try_field("argument-hint").is_some(), "missing skill field: argument-hint");
}

#[test]
fn repo_schemas_skip_skill_aliased_fields() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // skill.json aliases: name→title, description→summary, when_to_use→read_when
    assert!(is.try_field("name").is_none(), "'name' should be skipped (aliased to title)");
    assert!(is.try_field("description").is_none(), "'description' should be skipped (aliased to summary)");
    assert!(is.try_field("when_to_use").is_none(), "'when_to_use' should be skipped (aliased to read_when)");
}

#[test]
fn repo_schemas_field_count_is_reasonable() {
    let dir = tempfile::tempdir().unwrap();
    setup_wiki_with_repo_schemas(dir.path());
    let is = IndexSchema::build_from_schemas(dir.path(), "en_stem").unwrap();

    // 4 fixed + fields from 6 schemas (deduplicated, aliases skipped)
    // Should be roughly 25-35 fields
    let count = is.fields.len();
    assert!(
        count >= 20 && count <= 50,
        "unexpected field count: {count} (expected 20-50)"
    );
}
