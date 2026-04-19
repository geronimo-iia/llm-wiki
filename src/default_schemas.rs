use std::collections::HashMap;

const BASE: &str = include_str!("../schemas/base.json");
const CONCEPT: &str = include_str!("../schemas/concept.json");
const PAPER: &str = include_str!("../schemas/paper.json");
const SKILL: &str = include_str!("../schemas/skill.json");
const DOC: &str = include_str!("../schemas/doc.json");
const SECTION: &str = include_str!("../schemas/section.json");

/// Returns a map of schema filename → embedded JSON content.
pub fn default_schemas() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("base.json", BASE),
        ("concept.json", CONCEPT),
        ("paper.json", PAPER),
        ("skill.json", SKILL),
        ("doc.json", DOC),
        ("section.json", SECTION),
    ])
}

/// A default type entry extracted from `x-wiki-types` in a schema.
pub struct DefaultTypeEntry {
    pub type_name: String,
    pub schema_file: String,
    pub description: String,
}

/// Extract all default type entries from the embedded schemas.
///
/// Reads `x-wiki-types` from each schema file. Returns entries sorted
/// by type name for deterministic output.
pub fn default_type_entries() -> Vec<DefaultTypeEntry> {
    let mut entries = Vec::new();
    for (filename, content) in default_schemas() {
        let schema: serde_json::Value = serde_json::from_str(content)
            .unwrap_or_else(|e| panic!("{filename} is not valid JSON: {e}"));
        if let Some(types) = schema.get("x-wiki-types").and_then(|v| v.as_object()) {
            for (type_name, desc) in types {
                entries.push(DefaultTypeEntry {
                    type_name: type_name.clone(),
                    schema_file: format!("schemas/{filename}"),
                    description: desc.as_str().unwrap_or("").to_string(),
                });
            }
        }
    }
    entries.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    entries
}
