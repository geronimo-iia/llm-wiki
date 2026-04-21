use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{bail, Result};
use jsonschema::Validator;
use serde_yaml::Value;
use sha2::{Digest, Sha256};

use crate::config;
use crate::default_schemas;

/// A compiled type entry in the registry.
pub struct RegisteredType {
    pub(crate) schema_path: String,
    pub(crate) description: String,
    pub(crate) validator: Validator,
    pub(crate) aliases: HashMap<String, String>,
    pub(crate) required_fields: Vec<String>,
    pub(crate) content_hash: String,
    pub(crate) edges: Vec<EdgeDecl>,
}

/// A graph edge declaration from `x-graph-edges` in a type schema.
#[derive(Debug, Clone)]
pub struct EdgeDecl {
    pub field: String,
    pub relation: String,
    pub direction: String,
    pub target_types: Vec<String>,
}

/// Per-wiki type registry — discovers types from `schemas/*.json` via
/// `x-wiki-types`, with optional `[types.*]` overrides from `wiki.toml`.
pub struct SpaceTypeRegistry {
    types: HashMap<String, RegisteredType>,
    schema_hash: String,
    type_hashes: HashMap<String, String>,
}

impl std::fmt::Debug for SpaceTypeRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpaceTypeRegistry")
            .field("types", &self.types.keys().collect::<Vec<_>>())
            .field("schema_hash", &self.schema_hash)
            .finish()
    }
}

impl SpaceTypeRegistry {
    /// Build from a wiki repository root. Scans `schemas/*.json`, merges
    /// `wiki.toml` overrides.
    pub fn build(repo_root: &Path) -> Result<Self> {
        let schemas_dir = repo_root.join("schemas");
        let mut types = HashMap::new();

        if schemas_dir.is_dir() {
            discover_from_dir(&schemas_dir, &mut types)?;
        } else {
            discover_from_embedded(&mut types)?;
        }

        // Apply wiki.toml overrides
        let wiki_cfg = config::load_wiki(repo_root)?;
        for (type_name, entry) in &wiki_cfg.types {
            let schema_path = repo_root.join(&entry.schema);
            let content = std::fs::read_to_string(&schema_path)?;
            let registered = compile_schema(&entry.schema, &entry.description, &content)?;
            types.insert(type_name.clone(), registered);
        }

        // Enforce base schema invariant
        if !types.contains_key("default") {
            // Inject embedded base.json as fallback
            let schemas = default_schemas::default_schemas();
            let base = schemas["base.json"];
            let registered =
                compile_schema("schemas/base.json", "Fallback for unrecognized types", base)?;
            types.insert("default".to_string(), registered);
        } else {
            validate_base_invariant(&types["default"])?;
        }

        let (schema_hash, type_hashes) = compute_hashes(&types);

        Ok(Self {
            types,
            schema_hash,
            type_hashes,
        })
    }

    /// Build from embedded default schemas only (no disk access).
    /// Used when no wiki is mounted or for backward compatibility.
    pub fn from_embedded() -> Self {
        let mut types = HashMap::new();
        discover_from_embedded(&mut types).expect("embedded schemas are valid");
        let (schema_hash, type_hashes) = compute_hashes(&types);
        Self {
            types,
            schema_hash,
            type_hashes,
        }
    }

    /// Build from pre-constructed parts (used by space_builder).
    pub(crate) fn from_parts(
        types: HashMap<String, RegisteredType>,
        schema_hash: String,
        type_hashes: HashMap<String, String>,
    ) -> Self {
        Self {
            types,
            schema_hash,
            type_hashes,
        }
    }

    pub fn is_known(&self, type_name: &str) -> bool {
        self.types.contains_key(type_name)
    }

    /// List all registered type names with descriptions.
    pub fn list_types(&self) -> Vec<(&str, &str)> {
        let mut out: Vec<_> = self
            .types
            .iter()
            .map(|(name, rt)| (name.as_str(), rt.description.as_str()))
            .collect();
        out.sort_by_key(|(name, _)| *name);
        out
    }

    /// Get the aliases for a type (source field → canonical field).
    pub fn aliases(&self, type_name: &str) -> Option<&HashMap<String, String>> {
        self.types.get(type_name).map(|rt| &rt.aliases)
    }

    /// Get the schema file path for a type (relative to repo root).
    pub fn schema_path(&self, type_name: &str) -> Option<&str> {
        self.types.get(type_name).map(|rt| rt.schema_path.as_str())
    }

    pub fn schema_hash(&self) -> &str {
        &self.schema_hash
    }

    pub fn type_hashes(&self) -> &HashMap<String, String> {
        &self.type_hashes
    }

    /// Get edge declarations for a type.
    pub fn edges(&self, type_name: &str) -> &[EdgeDecl] {
        self.types
            .get(type_name)
            .map(|rt| rt.edges.as_slice())
            .unwrap_or(&[])
    }

    /// Validate frontmatter against the type's JSON Schema.
    ///
    /// - Resolves the page type (falls back to "default")
    /// - Validates against the compiled schema
    /// - In loose mode, unknown types produce warnings
    /// - In strict mode, unknown types produce errors
    ///
    /// Returns a list of warnings. Bails on hard errors.
    pub fn validate(&self, fm: &BTreeMap<String, Value>, strictness: &str) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // title is always required — hard error regardless of strictness
        let has_title = fm
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        // For skill pages, check "name" as alias for title
        let has_name = fm
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        if !has_title && !has_name {
            bail!("title is required");
        }

        let page_type = fm.get("type").and_then(|v| v.as_str()).unwrap_or("");

        // Determine which registered type to use
        let resolved_type = if page_type.is_empty() {
            warnings.push("missing field: type (defaulting to \"page\")".into());
            "default"
        } else if self.types.contains_key(page_type) {
            page_type
        } else {
            if strictness == "strict" {
                bail!("unknown type '{page_type}'");
            }
            warnings.push(format!("unknown type '{page_type}'"));
            "default"
        };

        if let Some(rt) = self.types.get(resolved_type) {
            let json_fm = yaml_fm_to_json(fm)?;
            let errors: Vec<_> = rt.validator.iter_errors(&json_fm).collect();
            if !errors.is_empty() {
                if strictness == "strict" {
                    bail!("schema validation failed: {}", errors[0]);
                }
                for e in &errors {
                    warnings.push(format!("schema validation: {e}"));
                }
            }
        }

        Ok(warnings)
    }
}

impl Default for SpaceTypeRegistry {
    fn default() -> Self {
        Self::from_embedded()
    }
}

// ── Discovery ─────────────────────────────────────────────────────────────────

fn discover_from_dir(
    schemas_dir: &Path,
    types: &mut HashMap<String, RegisteredType>,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(schemas_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy();
        let content = std::fs::read_to_string(&path)?;
        let schema_value: serde_json::Value = serde_json::from_str(&content)?;

        let schema_rel = format!("schemas/{filename}");
        let content_hash = sha256_hex(content.as_bytes());

        if let Some(wiki_types) = schema_value.get("x-wiki-types").and_then(|v| v.as_object()) {
            let aliases = extract_aliases(&schema_value);
            let required_fields = extract_required(&schema_value);
            let edges = extract_edges(&schema_value);

            for (type_name, desc) in wiki_types {
                let description = desc.as_str().unwrap_or("").to_string();
                let validator = Validator::new(&schema_value)
                    .map_err(|e| anyhow::anyhow!("invalid schema {filename}: {e}"))?;
                types.insert(
                    type_name.clone(),
                    RegisteredType {
                        schema_path: schema_rel.clone(),
                        description,
                        validator,
                        aliases: aliases.clone(),
                        required_fields: required_fields.clone(),
                        content_hash: content_hash.clone(),
                        edges: edges.clone(),
                    },
                );
            }
        }
    }

    Ok(())
}

fn discover_from_embedded(types: &mut HashMap<String, RegisteredType>) -> Result<()> {
    for entry in default_schemas::default_type_entries() {
        let filename = entry
            .schema_file
            .strip_prefix("schemas/")
            .unwrap_or(&entry.schema_file);
        let schemas = default_schemas::default_schemas();
        let content = schemas
            .get(filename)
            .ok_or_else(|| anyhow::anyhow!("embedded schema not found: {filename}"))?;
        let registered = compile_schema(&entry.schema_file, &entry.description, content)?;
        types.insert(entry.type_name, registered);
    }
    Ok(())
}

pub(crate) fn compile_schema(
    schema_path: &str,
    description: &str,
    content: &str,
) -> Result<RegisteredType> {
    let content_hash = sha256_hex(content.as_bytes());
    let schema_value: serde_json::Value = serde_json::from_str(content)?;
    compile_schema_from_value(schema_path, description, &schema_value, &content_hash)
}

pub(crate) fn compile_schema_from_value(
    schema_path: &str,
    description: &str,
    schema_value: &serde_json::Value,
    content_hash: &str,
) -> Result<RegisteredType> {
    let validator = Validator::new(schema_value)
        .map_err(|e| anyhow::anyhow!("invalid schema {schema_path}: {e}"))?;
    let aliases = extract_aliases(schema_value);
    let required_fields = extract_required(schema_value);
    let edges = extract_edges(schema_value);

    Ok(RegisteredType {
        schema_path: schema_path.to_string(),
        description: description.to_string(),
        validator,
        aliases,
        required_fields,
        content_hash: content_hash.to_string(),
        edges,
    })
}

pub(crate) fn extract_aliases(schema: &serde_json::Value) -> HashMap<String, String> {
    schema
        .get("x-index-aliases")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_required(schema: &serde_json::Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_edges(schema: &serde_json::Value) -> Vec<EdgeDecl> {
    schema
        .get("x-graph-edges")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(field, decl)| {
                    let relation = decl
                        .get("relation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("links-to")
                        .to_string();
                    let direction = decl
                        .get("direction")
                        .and_then(|v| v.as_str())
                        .unwrap_or("outgoing")
                        .to_string();
                    let target_types = decl
                        .get("target_types")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    EdgeDecl {
                        field: field.clone(),
                        relation,
                        direction,
                        target_types,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Validate that a custom default type requires at least `title` and `type`.
pub(crate) fn validate_base_invariant(rt: &RegisteredType) -> Result<()> {
    if !rt.required_fields.contains(&"title".to_string()) {
        bail!(
            "base schema '{}' must require 'title' — \
             the default type is the fallback for all unknown types",
            rt.schema_path
        );
    }
    if !rt.required_fields.contains(&"type".to_string()) {
        bail!(
            "base schema '{}' must require 'type' — \
             the default type is the fallback for all unknown types",
            rt.schema_path
        );
    }
    Ok(())
}

// ── Hashing ───────────────────────────────────────────────────────────────────

/// SHA-256 of bytes, returned as lowercase hex (64 chars).
pub(crate) fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn compute_hashes(
    types: &HashMap<String, RegisteredType>,
) -> (String, HashMap<String, String>) {
    let entries: HashMap<String, (String, HashMap<String, String>, String)> = types
        .iter()
        .map(|(name, rt)| {
            (
                name.clone(),
                (
                    rt.schema_path.clone(),
                    rt.aliases.clone(),
                    rt.content_hash.clone(),
                ),
            )
        })
        .collect();
    hash_type_entries(&entries)
}

/// Shared hashing core for compute_hashes and compute_disk_hashes.
/// Per-type hash = SHA-256(schema_path + sorted_aliases + content_hash).
/// Global hash = SHA-256(all per-type hashes sorted by name).
fn hash_type_entries(
    entries: &HashMap<String, (String, HashMap<String, String>, String)>,
) -> (String, HashMap<String, String>) {
    let sorted: BTreeMap<_, _> = entries.iter().collect();
    let mut type_hashes = HashMap::new();
    let mut global_hasher = Sha256::new();

    for (name, (schema_path, aliases, content_hash)) in &sorted {
        let mut h = Sha256::new();
        h.update(schema_path.as_bytes());
        let sorted_aliases: BTreeMap<_, _> = aliases.iter().collect();
        for (k, v) in &sorted_aliases {
            h.update(k.as_bytes());
            h.update(v.as_bytes());
        }
        h.update(content_hash.as_bytes());
        let type_hash = format!("{:x}", h.finalize());
        type_hashes.insert(name.to_string(), type_hash.clone());
        global_hasher.update(type_hash.as_bytes());
    }

    (format!("{:x}", global_hasher.finalize()), type_hashes)
}

/// Compute schema hashes directly from disk without building a full registry.
/// Returns (global_hash, per_type_hashes).
///
/// Algorithm:
/// 1. Scan `schemas/*.json` (sorted) — compute content hash per file
/// 2. Read `x-wiki-types` to map type_name → content_hash
/// 3. Apply `wiki.toml` `[types.*]` overrides
/// 4. Per-type hash = SHA-256(schema_path + sorted_aliases + content_hash)
/// 5. Global hash = SHA-256(all per-type hashes sorted by name)
///
/// Falls back to embedded schemas if `schemas/` dir is absent.
pub fn compute_disk_hashes(repo_root: &Path) -> Result<(String, HashMap<String, String>)> {
    let schemas_dir = repo_root.join("schemas");

    // Collect (type_name -> (schema_path, aliases, content_hash))
    let mut type_data: HashMap<String, (String, HashMap<String, String>, String)> = HashMap::new();

    if schemas_dir.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(&schemas_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let content = std::fs::read_to_string(&path)?;
            let content_hash = sha256_hex(content.as_bytes());
            let schema_rel = format!("schemas/{filename}");
            let schema_value: serde_json::Value = serde_json::from_str(&content)?;

            if let Some(wiki_types) = schema_value.get("x-wiki-types").and_then(|v| v.as_object()) {
                let aliases = extract_aliases(&schema_value);
                for (type_name, _) in wiki_types {
                    type_data.insert(
                        type_name.clone(),
                        (schema_rel.clone(), aliases.clone(), content_hash.clone()),
                    );
                }
            }
        }
    } else {
        // Embedded fallback
        for (filename, content) in default_schemas::default_schemas() {
            let content_hash = sha256_hex(content.as_bytes());
            let schema_rel = format!("schemas/{filename}");
            let schema_value: serde_json::Value = serde_json::from_str(content)?;

            if let Some(wiki_types) = schema_value.get("x-wiki-types").and_then(|v| v.as_object()) {
                let aliases = extract_aliases(&schema_value);
                for (type_name, _) in wiki_types {
                    type_data.insert(
                        type_name.clone(),
                        (schema_rel.clone(), aliases.clone(), content_hash.clone()),
                    );
                }
            }
        }
    }

    // Apply wiki.toml overrides
    let wiki_cfg = config::load_wiki(repo_root)?;
    for (type_name, entry) in &wiki_cfg.types {
        let schema_path = repo_root.join(&entry.schema);
        let content = std::fs::read_to_string(&schema_path)?;
        let content_hash = sha256_hex(content.as_bytes());
        let schema_value: serde_json::Value = serde_json::from_str(&content)?;
        let aliases = extract_aliases(&schema_value);
        type_data.insert(
            type_name.clone(),
            (entry.schema.clone(), aliases, content_hash),
        );
    }

    // Ensure default type exists (same logic as build)
    if !type_data.contains_key("default") {
        let schemas = default_schemas::default_schemas();
        let base = schemas["base.json"];
        let content_hash = sha256_hex(base.as_bytes());
        let schema_value: serde_json::Value = serde_json::from_str(base)?;
        let aliases = extract_aliases(&schema_value);
        type_data.insert(
            "default".to_string(),
            ("schemas/base.json".to_string(), aliases, content_hash),
        );
    }

    // Compute per-type and global hashes
    Ok(hash_type_entries(&type_data))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn yaml_fm_to_json(fm: &BTreeMap<String, Value>) -> Result<serde_json::Value> {
    // Round-trip through serde: yaml::Value → String → json::Value
    let yaml_str = serde_yaml::to_string(fm)?;
    let json: serde_json::Value = serde_yaml::from_str(&yaml_str)?;
    Ok(json)
}
