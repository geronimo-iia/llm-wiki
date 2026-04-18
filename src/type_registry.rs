use std::collections::{BTreeMap, HashSet};

use anyhow::{bail, Result};
use serde_yaml::Value;

const BUILT_IN_TYPES: &[&str] = &[
    "page",
    "concept",
    "query-result",
    "section",
    "paper",
    "article",
    "documentation",
    "clipping",
    "transcript",
    "note",
    "data",
    "book-chapter",
    "thread",
    "skill",
    "doc",
];

/// Hardcoded type registry for Phase 1.
///
/// Knows built-in type names and validates base frontmatter fields.
/// Phase 2 replaces this with a dynamic registry driven by JSON Schema.
pub struct TypeRegistry {
    known_types: HashSet<String>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let known_types = BUILT_IN_TYPES.iter().map(|s| (*s).to_string()).collect();
        Self { known_types }
    }

    pub fn is_known(&self, type_name: &str) -> bool {
        self.known_types.contains(type_name)
    }

    /// Validate base frontmatter fields.
    ///
    /// - `title` is required (error if missing)
    /// - `type` defaults to "page" if missing
    /// - Unknown types produce warnings in loose mode, errors in strict mode
    ///
    /// Returns a list of warnings. Bails on hard errors.
    pub fn validate(
        &self,
        fm: &BTreeMap<String, Value>,
        strictness: &str,
    ) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        let title = fm.get("title").and_then(|v| v.as_str()).unwrap_or("");
        if title.is_empty() {
            bail!("title is required");
        }

        let page_type = fm.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if page_type.is_empty() {
            warnings.push("missing field: type (defaulting to \"page\")".into());
        } else if !self.is_known(page_type) {
            if strictness == "strict" {
                bail!("unknown type '{page_type}'");
            }
            warnings.push(format!("unknown type '{page_type}'"));
        }

        Ok(warnings)
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
