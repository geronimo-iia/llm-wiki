use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use tantivy::schema::{
    FAST, Field, IndexRecordOption, NumericOptions, STORED, STRING, Schema, TextFieldIndexing,
    TextOptions,
};

use crate::config;
use crate::default_schemas;

/// Tantivy schema + field handles.
///
/// Built once from schema files (or hardcoded defaults). No raw JSON
/// Schema content is kept — only the compiled tantivy schema and a
/// field name → Field handle map.
pub struct IndexSchema {
    /// Compiled Tantivy schema.
    pub schema: Schema,
    /// Map from field name to Tantivy `Field` handle.
    pub fields: HashMap<String, Field>,
    keyword_fields: HashSet<String>,
    numeric_fields: HashSet<String>,
}

impl IndexSchema {
    /// Build from schema files on disk + wiki.toml overrides.
    ///
    /// Reads each schema file once, extracts properties, classifies
    /// fields, builds the tantivy schema, then discards the raw JSON.
    pub fn build_from_schemas(repo_root: &Path, tokenizer: &str) -> Result<Self> {
        let schemas_dir = repo_root.join("schemas");
        let schema_sources = if schemas_dir.is_dir() {
            collect_schema_sources_from_dir(&schemas_dir, repo_root)?
        } else {
            collect_schema_sources_from_embedded()
        };

        let mut builder = SchemaBuilder::new(tokenizer);
        builder.add_fixed_fields();

        // Collect and classify fields from all schemas
        let mut seen: HashSet<String> = HashSet::new();
        // Fixed fields are already added
        for name in &["slug", "uri", "body", "body_links"] {
            seen.insert(name.to_string());
        }

        for source in &schema_sources {
            let aliases: HashSet<&str> = source.aliases.keys().map(|k| k.as_str()).collect();
            let edge_fields: HashSet<&str> =
                source.edge_fields.iter().map(|s| s.as_str()).collect();

            for (field_name, field_def) in &source.properties {
                // Skip aliased fields — they index under their canonical name
                if aliases.contains(field_name.as_str()) {
                    continue;
                }
                // Skip if already added from another schema
                if seen.contains(field_name) {
                    continue;
                }
                seen.insert(field_name.clone());

                let is_slug = edge_fields.contains(field_name.as_str());
                let classification = classify_field(field_def, is_slug);

                match classification {
                    FieldClass::Text => builder.add_text(field_name),
                    FieldClass::Keyword => builder.add_keyword(field_name),
                    FieldClass::Numeric => builder.add_numeric(field_name),
                }
            }
        }

        Ok(builder.finish())
    }

    /// Return true if `name` is stored as a keyword (STRING | STORED | FAST) field.
    pub fn is_keyword(&self, name: &str) -> bool {
        self.keyword_fields.contains(name)
    }

    /// Return true if `name` is stored as a numeric (f64 FAST) field.
    pub fn is_numeric(&self, name: &str) -> bool {
        self.numeric_fields.contains(name)
    }

    /// Get a field handle by name. Panics if the field doesn't exist.
    pub fn field(&self, name: &str) -> Field {
        self.fields[name]
    }

    /// Try to get a field handle, returning None if the field doesn't exist.
    pub fn try_field(&self, name: &str) -> Option<Field> {
        self.fields.get(name).copied()
    }
}

// ── Field classification ──────────────────────────────────────────────────────

pub(crate) enum FieldClass {
    Text,
    Keyword,
    Numeric,
}

pub(crate) fn classify_field(prop: &serde_json::Value, is_slug_field: bool) -> FieldClass {
    // Slug fields (from x-graph-edges) are always keywords
    if is_slug_field {
        return FieldClass::Keyword;
    }

    let prop_type = prop.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match prop_type {
        "string" => {
            // enum or const → keyword
            if prop.get("enum").is_some() || prop.get("const").is_some() {
                FieldClass::Keyword
            } else {
                FieldClass::Text
            }
        }
        "boolean" => FieldClass::Keyword,
        "array" => {
            // Array of strings with enum items → keyword
            if let Some(items) = prop.get("items")
                && (items.get("enum").is_some() || items.get("const").is_some())
            {
                return FieldClass::Keyword;
            }
            FieldClass::Text
        }
        "number" | "integer" => FieldClass::Numeric,
        // object or unknown → text (serialized)
        _ => FieldClass::Text,
    }
}

// ── Schema sources ────────────────────────────────────────────────────────────

/// Transient data extracted from one schema file — discarded after
/// IndexSchema construction.
struct SchemaSource {
    properties: Vec<(String, serde_json::Value)>,
    aliases: HashMap<String, String>,
    edge_fields: HashSet<String>,
}

fn collect_schema_sources_from_dir(
    schemas_dir: &Path,
    repo_root: &Path,
) -> Result<Vec<SchemaSource>> {
    let mut sources = Vec::new();
    let mut seen_files: HashSet<String> = HashSet::new();

    // Scan schemas/*.json
    let mut entries: Vec<_> = std::fs::read_dir(schemas_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        seen_files.insert(filename);
        let content = std::fs::read_to_string(&path)?;
        sources.push(extract_schema_source(&content)?);
    }

    // Add wiki.toml override schemas not already scanned
    let wiki_cfg = config::load_wiki(repo_root)?;
    for type_entry in wiki_cfg.types.values() {
        let schema_path = repo_root.join(&type_entry.schema);
        let filename = schema_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        if !seen_files.contains(&filename) {
            seen_files.insert(filename);
            let content = std::fs::read_to_string(&schema_path)?;
            sources.push(extract_schema_source(&content)?);
        }
    }

    Ok(sources)
}

fn collect_schema_sources_from_embedded() -> Vec<SchemaSource> {
    let mut sources = Vec::new();
    for (_filename, content) in default_schemas::default_schemas() {
        if let Ok(source) = extract_schema_source(content) {
            sources.push(source);
        }
    }
    sources
}

/// Extract field definitions, aliases, and edge field names from a
/// single schema file. This is the only place raw JSON is read —
/// the result is a lightweight struct that the builder consumes.
fn extract_schema_source(content: &str) -> Result<SchemaSource> {
    let schema: serde_json::Value = serde_json::from_str(content)?;

    let properties = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let aliases = schema
        .get("x-index-aliases")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let edge_fields = schema
        .get("x-graph-edges")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    Ok(SchemaSource {
        properties,
        aliases,
        edge_fields,
    })
}

// ── Schema builder helper ─────────────────────────────────────────────────────

pub(crate) struct SchemaBuilder {
    builder: tantivy::schema::SchemaBuilder,
    fields: HashMap<String, Field>,
    keyword_fields: HashSet<String>,
    numeric_fields: HashSet<String>,
    text_opts: TextOptions,
}

impl SchemaBuilder {
    pub(crate) fn new(tokenizer: &str) -> Self {
        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer(tokenizer)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_opts = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        Self {
            builder: Schema::builder(),
            fields: HashMap::new(),
            keyword_fields: HashSet::new(),
            numeric_fields: HashSet::new(),
            text_opts,
        }
    }

    pub(crate) fn add_fixed_fields(&mut self) {
        // slug needs FAST for sorted pagination via order_by_string_fast_field
        let slug_field = self.builder.add_text_field("slug", STRING | STORED | FAST);
        self.fields.insert("slug".to_string(), slug_field);
        self.keyword_fields.insert("slug".to_string());

        self.add_keyword("uri");
        self.add_text("body");
        self.add_keyword("body_links");
    }

    pub(crate) fn add_text(&mut self, name: &str) {
        if !self.fields.contains_key(name) {
            let field = self.builder.add_text_field(name, self.text_opts.clone());
            self.fields.insert(name.to_string(), field);
        }
    }

    pub(crate) fn add_keyword(&mut self, name: &str) {
        if !self.fields.contains_key(name) {
            let field = self.builder.add_text_field(name, STRING | STORED | FAST);
            self.fields.insert(name.to_string(), field);
            self.keyword_fields.insert(name.to_string());
        }
    }

    pub(crate) fn add_numeric(&mut self, name: &str) {
        if !self.fields.contains_key(name) {
            let opts = NumericOptions::default() | FAST | STORED;
            let field = self.builder.add_f64_field(name, opts);
            self.fields.insert(name.to_string(), field);
            self.numeric_fields.insert(name.to_string());
        }
    }

    pub(crate) fn finish(self) -> IndexSchema {
        IndexSchema {
            schema: self.builder.build(),
            fields: self.fields,
            keyword_fields: self.keyword_fields,
            numeric_fields: self.numeric_fields,
        }
    }
}
