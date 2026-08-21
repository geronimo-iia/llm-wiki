#![allow(unreachable_pub)]
use std::collections::HashMap;

const BASE: &str = include_str!("../schemas/base.json");
const CONCEPT: &str = include_str!("../schemas/concept.json");
const PAPER: &str = include_str!("../schemas/paper.json");
const SKILL: &str = include_str!("../schemas/skill.json");
const DOC: &str = include_str!("../schemas/doc.json");
const SECTION: &str = include_str!("../schemas/section.json");

// ── Archive: pre-1.0.0 ───────────────────────────────────────────────────────
// Add new archive sections here when schemas change in future releases.
// Retrieve old content from the relevant git tag before updating current files.

const ARCHIVE_PRE100_BASE: &str = include_str!("../schemas/archive/pre-1.0.0/base.json");
const ARCHIVE_PRE100_CONCEPT: &str = include_str!("../schemas/archive/pre-1.0.0/concept.json");
const ARCHIVE_PRE100_DOC: &str = include_str!("../schemas/archive/pre-1.0.0/doc.json");
const ARCHIVE_PRE100_PAPER: &str = include_str!("../schemas/archive/pre-1.0.0/paper.json");
const ARCHIVE_PRE100_SECTION: &str = include_str!("../schemas/archive/pre-1.0.0/section.json");

const TMPL_CONCEPT: &str = include_str!("../schemas/concept.md");
const TMPL_PAPER: &str = include_str!("../schemas/paper.md");
const TMPL_DOC: &str = include_str!("../schemas/doc.md");
const TMPL_SECTION: &str = include_str!("../schemas/section.md");
const TMPL_QUERY_RESULT: &str = include_str!("../schemas/query-result.md");

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

/// Returns a map of template filename → embedded Markdown content.
pub fn default_templates() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("concept.md", TMPL_CONCEPT),
        ("paper.md", TMPL_PAPER),
        ("doc.md", TMPL_DOC),
        ("section.md", TMPL_SECTION),
        ("query-result.md", TMPL_QUERY_RESULT),
    ])
}

/// Resolve a body template for a type name. Checks embedded templates.
pub fn embedded_body_template(type_name: &str) -> Option<&'static str> {
    let filename = format!("{type_name}.md");
    default_templates().get(filename.as_str()).copied()
}

/// A default type entry extracted from `x-wiki-types` in a schema.
pub struct DefaultTypeEntry {
    /// The type identifier (e.g. `"concept"`, `"paper"`).
    pub type_name: String,
    /// Relative path to the schema file that defines this type (e.g. `"schemas/concept.json"`).
    pub schema_file: String,
    /// Human-readable description of the type.
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

/// All known stock schema content across all releases.
///
/// Includes current embedded schemas and all archived historical versions.
/// Used by `wiki migrate` to recognise stale stock copies from any release.
///
/// When schemas change in a future release:
/// 1. Copy the current schema files to `schemas/archive/<version>/`.
/// 2. Add `include_str!` constants above.
/// 3. Append them to the Vec returned here.
pub fn all_stock_schema_contents() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = default_schemas().into_values().collect();
    // pre-1.0.0 archive
    all.push(ARCHIVE_PRE100_BASE);
    all.push(ARCHIVE_PRE100_CONCEPT);
    all.push(ARCHIVE_PRE100_DOC);
    all.push(ARCHIVE_PRE100_PAPER);
    all.push(ARCHIVE_PRE100_SECTION);
    all
}

/// Returns true if `on_disk_content` is semantically identical to any
/// known stock schema version (current or archived), using parsed JSON
/// equality. Immune to whitespace, key ordering, and CRLF differences.
pub fn is_stock_schema(on_disk_content: &str) -> bool {
    let Ok(on_disk_val) = serde_json::from_str::<serde_json::Value>(on_disk_content) else {
        return false;
    };
    all_stock_schema_contents().iter().any(|known| {
        serde_json::from_str::<serde_json::Value>(known)
            .map(|v| v == on_disk_val)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_embedded_concept_is_stock() {
        assert!(is_stock_schema(default_schemas()["concept.json"]));
    }

    #[test]
    fn archived_pre100_concept_is_stock() {
        assert!(is_stock_schema(ARCHIVE_PRE100_CONCEPT));
    }

    #[test]
    fn modified_schema_is_not_stock() {
        let mut val: serde_json::Value =
            serde_json::from_str(default_schemas()["concept.json"]).unwrap();
        val["x-custom"] = serde_json::json!(true);
        assert!(!is_stock_schema(&serde_json::to_string(&val).unwrap()));
    }

    #[test]
    fn whitespace_variant_is_still_stock() {
        let val: serde_json::Value =
            serde_json::from_str(default_schemas()["concept.json"]).unwrap();
        assert!(is_stock_schema(
            &serde_json::to_string_pretty(&val).unwrap()
        ));
    }

    #[test]
    fn invalid_json_is_not_stock() {
        assert!(!is_stock_schema("{ not valid json"));
    }

    #[test]
    fn all_current_embedded_are_stock() {
        for (filename, content) in default_schemas() {
            assert!(
                is_stock_schema(content),
                "current embedded schema {filename} not recognised by is_stock_schema"
            );
        }
    }
}
