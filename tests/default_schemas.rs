use jsonschema::Validator;
use llm_wiki::default_schemas::default_schemas;
use serde_json::{Value, json};

fn compile(name: &str) -> Validator {
    let schemas = default_schemas();
    let content = schemas[name];
    let schema: Value = serde_json::from_str(content).unwrap();
    Validator::new(&schema).unwrap_or_else(|e| panic!("{name} is not a valid JSON Schema: {e}"))
}

// ── Schema validity ──────────────────────────────────────────────────────────

#[test]
fn schema_count() {
    assert_eq!(default_schemas().len(), 6);
}

#[test]
fn all_schemas_compile_as_validators() {
    for (name, content) in default_schemas() {
        let schema: Value = serde_json::from_str(content)
            .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        Validator::new(&schema)
            .unwrap_or_else(|e| panic!("{name} is not a valid JSON Schema: {e}"));
    }
}

// ── base.json ────────────────────────────────────────────────────────────────

#[test]
fn base_accepts_minimal() {
    let v = compile("base.json");
    assert!(v.is_valid(&json!({"title": "Test", "type": "page"})));
}

#[test]
fn base_rejects_missing_title() {
    let v = compile("base.json");
    assert!(!v.is_valid(&json!({"type": "page"})));
}

#[test]
fn base_rejects_missing_type() {
    let v = compile("base.json");
    assert!(!v.is_valid(&json!({"title": "Test"})));
}

#[test]
fn base_allows_additional_properties() {
    let v = compile("base.json");
    assert!(v.is_valid(&json!({"title": "Test", "type": "page", "custom": "ok"})));
}

// ── concept.json ─────────────────────────────────────────────────────────────

#[test]
fn concept_requires_read_when() {
    let v = compile("concept.json");
    assert!(!v.is_valid(&json!({"title": "MoE", "type": "concept"})));
    assert!(v.is_valid(&json!({
        "title": "MoE", "type": "concept",
        "read_when": ["Reasoning about MoE"]
    })));
}

#[test]
fn concept_accepts_full_template() {
    let v = compile("concept.json");
    assert!(v.is_valid(&json!({
        "title": "Mixture of Experts",
        "summary": "Sparse routing of tokens to expert subnetworks.",
        "tldr": "MoE reduces compute 8x at pre-training scale.",
        "read_when": ["Reasoning about MoE architecture tradeoffs"],
        "status": "active",
        "type": "concept",
        "last_updated": "2025-07-17",
        "tags": ["mixture-of-experts", "scaling"],
        "sources": ["sources/switch-transformer-2021"],
        "concepts": ["concepts/scaling-laws"],
        "confidence": 0.9,
        "claims": [{
            "text": "Sparse MoE reduces effective compute 8x",
            "confidence": "high",
            "section": "Results"
        }]
    })));
}

#[test]
fn concept_rejects_invalid_confidence() {
    let v = compile("concept.json");
    assert!(!v.is_valid(&json!({
        "title": "Test", "type": "concept",
        "read_when": ["test"],
        "confidence": "very-high"
    })));
}

// ── paper.json ───────────────────────────────────────────────────────────────

#[test]
fn paper_accepts_source_template() {
    let v = compile("paper.json");
    assert!(v.is_valid(&json!({
        "title": "Switch Transformer (2021)",
        "summary": "Fedus et al. on scaling MoE.",
        "type": "paper",
        "status": "active",
        "read_when": ["Looking for MoE benchmark results"],
        "concepts": ["concepts/mixture-of-experts"],
        "confidence": 0.9,
        "claims": [{"text": "Switch routing achieves 4x speedup", "confidence": "high"}]
    })));
}

#[test]
fn paper_does_not_require_read_when() {
    let v = compile("paper.json");
    assert!(v.is_valid(&json!({"title": "Test Paper", "type": "paper"})));
}

#[test]
fn paper_rejects_invalid_confidence() {
    let v = compile("paper.json");
    assert!(!v.is_valid(&json!({
        "title": "Test", "type": "paper",
        "confidence": "unknown"
    })));
}

// ── skill.json ───────────────────────────────────────────────────────────────

#[test]
fn skill_requires_name_and_description() {
    let v = compile("skill.json");
    assert!(!v.is_valid(&json!({"description": "Does stuff", "type": "skill"})));
    assert!(!v.is_valid(&json!({"name": "test", "type": "skill"})));
    assert!(v.is_valid(&json!({"name": "test", "description": "Does stuff", "type": "skill"})));
}

#[test]
fn skill_accepts_full_template() {
    let v = compile("skill.json");
    assert!(v.is_valid(&json!({
        "name": "ingest",
        "description": "Process source files into synthesized wiki pages.",
        "type": "skill",
        "status": "active",
        "last_updated": "2025-07-17",
        "disable-model-invocation": true,
        "allowed-tools": ["Read", "Write"],
        "tags": ["ingest", "workflow"],
        "owner": "geronimo",
        "document_refs": ["docs/ingest-guide"]
    })));
}

#[test]
fn all_schemas_have_x_wiki_types() {
    for (name, content) in default_schemas() {
        let schema: Value = serde_json::from_str(content).unwrap();
        assert!(
            schema.get("x-wiki-types").is_some(),
            "{name} missing x-wiki-types"
        );
        let types = schema["x-wiki-types"].as_object().unwrap();
        assert!(!types.is_empty(), "{name} has empty x-wiki-types");
    }
}

#[test]
fn default_type_entries_discovers_all_15_types() {
    let entries = llm_wiki::default_schemas::default_type_entries();
    assert_eq!(entries.len(), 15);

    let names: Vec<&str> = entries.iter().map(|e| e.type_name.as_str()).collect();
    assert!(names.contains(&"default"));
    assert!(names.contains(&"concept"));
    assert!(names.contains(&"query-result"));
    assert!(names.contains(&"paper"));
    assert!(names.contains(&"article"));
    assert!(names.contains(&"documentation"));
    assert!(names.contains(&"clipping"));
    assert!(names.contains(&"transcript"));
    assert!(names.contains(&"note"));
    assert!(names.contains(&"data"));
    assert!(names.contains(&"book-chapter"));
    assert!(names.contains(&"thread"));
    assert!(names.contains(&"skill"));
    assert!(names.contains(&"doc"));
    assert!(names.contains(&"section"));
}

#[test]
fn default_type_entries_schema_paths_are_correct() {
    let entries = llm_wiki::default_schemas::default_type_entries();
    for entry in &entries {
        let filename = entry.schema_file.strip_prefix("schemas/").unwrap();
        assert!(
            default_schemas().contains_key(filename),
            "type '{}' references unknown schema '{}'",
            entry.type_name,
            entry.schema_file
        );
    }
}

// ── doc.json ─────────────────────────────────────────────────────────────────

#[test]
fn doc_accepts_template() {
    let v = compile("doc.json");
    assert!(v.is_valid(&json!({
        "title": "Payment API Reference",
        "summary": "Endpoints, auth, error codes.",
        "type": "doc",
        "status": "active",
        "tags": ["api", "payment"],
        "sources": ["sources/payment-rfc-2024"]
    })));
}

#[test]
fn doc_accepts_minimal() {
    let v = compile("doc.json");
    assert!(v.is_valid(&json!({"title": "Test Doc", "type": "doc"})));
}

// ── section.json ─────────────────────────────────────────────────────────────

#[test]
fn section_accepts_template() {
    let v = compile("section.json");
    assert!(v.is_valid(&json!({
        "title": "Scaling Research",
        "summary": "Papers and concepts related to model scaling.",
        "type": "section",
        "status": "active"
    })));
}

#[test]
fn section_rejects_missing_title() {
    let v = compile("section.json");
    assert!(!v.is_valid(&json!({"type": "section"})));
}

#[test]
fn skill_has_index_aliases() {
    let schema: Value = serde_json::from_str(default_schemas()["skill.json"]).unwrap();
    let aliases = schema
        .get("x-index-aliases")
        .expect("missing x-index-aliases");
    assert_eq!(aliases["name"], "title");
    assert_eq!(aliases["description"], "summary");
    assert_eq!(aliases["when_to_use"], "read_when");
}
