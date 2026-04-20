use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use jsonschema::Validator;

use crate::config;
use crate::default_schemas;
use crate::index_schema::{classify_field, FieldClass, IndexSchema, SchemaBuilder};
use crate::type_registry::{
    self, compute_hashes, extract_aliases, extract_required, sha256_hex, validate_base_invariant,
    RegisteredType, SpaceTypeRegistry,
};

/// Build both SpaceTypeRegistry and IndexSchema from a wiki's schema
/// files. Reads each schema file once, discards raw JSON after
/// construction.
pub fn build_space(repo_root: &Path, tokenizer: &str) -> Result<(SpaceTypeRegistry, IndexSchema)> {
    let schemas_dir = repo_root.join("schemas");
    let parsed = if schemas_dir.is_dir() {
        parse_from_dir(&schemas_dir, repo_root)?
    } else {
        parse_from_embedded()?
    };

    let (registry, index_schema) = assemble(parsed, repo_root, tokenizer)?;
    Ok((registry, index_schema))
}

/// Build both from embedded defaults (no disk access).
pub fn build_space_from_embedded(tokenizer: &str) -> (SpaceTypeRegistry, IndexSchema) {
    let parsed = parse_from_embedded().expect("embedded schemas are valid");
    // No wiki.toml overrides for embedded
    assemble_without_overrides(parsed, tokenizer).expect("embedded schemas are valid")
}

// ── Intermediate parsed data ──────────────────────────────────────────────────

struct ParsedSchemaFile {
    schema_rel: String,
    schema_json: serde_json::Value,
    wiki_types: Vec<(String, String)>, // (type_name, description)
    aliases: HashMap<String, String>,
    required_fields: Vec<String>,
    properties: Vec<(String, serde_json::Value)>,
    edge_fields: HashSet<String>,
    content_hash: String,
}

// ── Parsing ───────────────────────────────────────────────────────────────────

fn parse_from_dir(schemas_dir: &Path, repo_root: &Path) -> Result<Vec<ParsedSchemaFile>> {
    let mut parsed = Vec::new();
    let mut seen_files: HashSet<String> = HashSet::new();

    let mut entries: Vec<_> = std::fs::read_dir(schemas_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        seen_files.insert(filename.clone());
        let content = std::fs::read_to_string(&path)?;
        let schema_rel = format!("schemas/{filename}");
        parsed.push(parse_schema_file(&schema_rel, &content)?);
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
            parsed.push(parse_schema_file(&type_entry.schema, &content)?);
        }
    }

    Ok(parsed)
}

fn parse_from_embedded() -> Result<Vec<ParsedSchemaFile>> {
    let mut parsed = Vec::new();
    let mut schemas: Vec<_> = default_schemas::default_schemas().into_iter().collect();
    schemas.sort_by_key(|(filename, _)| *filename);
    for (filename, content) in schemas {
        let schema_rel = format!("schemas/{filename}");
        parsed.push(parse_schema_file(&schema_rel, content)?);
    }
    Ok(parsed)
}

fn parse_schema_file(schema_rel: &str, content: &str) -> Result<ParsedSchemaFile> {
    let content_hash = sha256_hex(content.as_bytes());
    let schema_json: serde_json::Value = serde_json::from_str(content)?;

    let wiki_types = schema_json
        .get("x-wiki-types")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect()
        })
        .unwrap_or_default();

    let aliases = extract_aliases(&schema_json);
    let required_fields = extract_required(&schema_json);

    let properties = schema_json
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();

    let edge_fields = schema_json
        .get("x-graph-edges")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    Ok(ParsedSchemaFile {
        schema_rel: schema_rel.to_string(),
        schema_json,
        wiki_types,
        aliases,
        required_fields,
        properties,
        edge_fields,
        content_hash,
    })
}

// ── Assembly ──────────────────────────────────────────────────────────────────

fn assemble(
    parsed: Vec<ParsedSchemaFile>,
    repo_root: &Path,
    tokenizer: &str,
) -> Result<(SpaceTypeRegistry, IndexSchema)> {
    let mut types = HashMap::new();
    let mut schema_builder = SchemaBuilder::new(tokenizer);
    schema_builder.add_fixed_fields();
    let mut seen_fields: HashSet<String> =
        ["slug", "uri", "body", "body_links"].iter().map(|s| s.to_string()).collect();

    for pf in &parsed {
        // Build registry entries
        for (type_name, description) in &pf.wiki_types {
            let validator = Validator::new(&pf.schema_json)
                .map_err(|e| anyhow::anyhow!("invalid schema {}: {e}", pf.schema_rel))?;
            types.insert(
                type_name.clone(),
                RegisteredType {
                    schema_path: pf.schema_rel.clone(),
                    description: description.clone(),
                    validator,
                    aliases: pf.aliases.clone(),
                    required_fields: pf.required_fields.clone(),
                    content_hash: pf.content_hash.clone(),
                },
            );
        }

        // Build index schema fields
        let alias_keys: HashSet<&str> = pf.aliases.keys().map(|k| k.as_str()).collect();
        for (field_name, field_def) in &pf.properties {
            if alias_keys.contains(field_name.as_str()) {
                continue;
            }
            if seen_fields.contains(field_name) {
                continue;
            }
            seen_fields.insert(field_name.clone());

            let is_slug = pf.edge_fields.contains(field_name);
            match classify_field(field_def, is_slug) {
                FieldClass::Text => schema_builder.add_text(field_name),
                FieldClass::Keyword => schema_builder.add_keyword(field_name),
            }
        }
    }

    // Apply wiki.toml overrides
    let wiki_cfg = config::load_wiki(repo_root)?;
    for (type_name, entry) in &wiki_cfg.types {
        let schema_path = repo_root.join(&entry.schema);
        let content = std::fs::read_to_string(&schema_path)?;
        let registered = type_registry::compile_schema(&entry.schema, &entry.description, &content)?;
        types.insert(type_name.clone(), registered);
    }

    // Enforce base schema invariant
    if !types.contains_key("default") {
        let schemas = default_schemas::default_schemas();
        let base = schemas["base.json"];
        let registered =
            type_registry::compile_schema("schemas/base.json", "Fallback for unrecognized types", base)?;
        types.insert("default".to_string(), registered);
    } else {
        validate_base_invariant(&types["default"])?;
    }

    let (schema_hash, type_hashes) = compute_hashes(&types);
    let registry = SpaceTypeRegistry::from_parts(types, schema_hash, type_hashes);
    let index_schema = schema_builder.finish();

    Ok((registry, index_schema))
}

fn assemble_without_overrides(
    parsed: Vec<ParsedSchemaFile>,
    tokenizer: &str,
) -> Result<(SpaceTypeRegistry, IndexSchema)> {
    let mut types = HashMap::new();
    let mut schema_builder = SchemaBuilder::new(tokenizer);
    schema_builder.add_fixed_fields();
    let mut seen_fields: HashSet<String> =
        ["slug", "uri", "body", "body_links"].iter().map(|s| s.to_string()).collect();

    for pf in &parsed {
        for (type_name, description) in &pf.wiki_types {
            let validator = Validator::new(&pf.schema_json)
                .map_err(|e| anyhow::anyhow!("invalid schema {}: {e}", pf.schema_rel))?;
            types.insert(
                type_name.clone(),
                RegisteredType {
                    schema_path: pf.schema_rel.clone(),
                    description: description.clone(),
                    validator,
                    aliases: pf.aliases.clone(),
                    required_fields: pf.required_fields.clone(),
                    content_hash: pf.content_hash.clone(),
                },
            );
        }

        let alias_keys: HashSet<&str> = pf.aliases.keys().map(|k| k.as_str()).collect();
        for (field_name, field_def) in &pf.properties {
            if alias_keys.contains(field_name.as_str()) {
                continue;
            }
            if seen_fields.contains(field_name) {
                continue;
            }
            seen_fields.insert(field_name.clone());

            let is_slug = pf.edge_fields.contains(field_name);
            match classify_field(field_def, is_slug) {
                FieldClass::Text => schema_builder.add_text(field_name),
                FieldClass::Keyword => schema_builder.add_keyword(field_name),
            }
        }
    }

    // Base invariant — embedded always has default
    let (schema_hash, type_hashes) = compute_hashes(&types);
    let registry = SpaceTypeRegistry::from_parts(types, schema_hash, type_hashes);
    let index_schema = schema_builder.finish();

    Ok((registry, index_schema))
}
