use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::engine::{EngineState, WikiEngine};
use crate::git;
use crate::markdown;
use crate::search;
use crate::space_builder;

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaTypeEntry {
    pub name: String,
    pub description: String,
    pub schema_path: String,
}

pub fn schema_list(engine: &EngineState, wiki_name: &str) -> Result<Vec<SchemaTypeEntry>> {
    let space = engine.space(wiki_name)?;
    Ok(space
        .type_registry
        .list_types()
        .into_iter()
        .map(|(name, desc)| SchemaTypeEntry {
            name: name.to_string(),
            description: desc.to_string(),
            schema_path: space
                .type_registry
                .schema_path(name)
                .unwrap_or_default()
                .to_string(),
        })
        .collect())
}

pub fn schema_show(engine: &EngineState, wiki_name: &str, type_name: &str) -> Result<String> {
    let space = engine.space(wiki_name)?;
    let schema_path = space
        .type_registry
        .schema_path(type_name)
        .ok_or_else(|| anyhow::anyhow!("type '{type_name}' is not registered"))?;
    let full_path = space.repo_root.join(schema_path);
    std::fs::read_to_string(&full_path)
        .with_context(|| format!("failed to read schema: {}", full_path.display()))
}

pub fn schema_show_template(
    engine: &EngineState,
    wiki_name: &str,
    type_name: &str,
) -> Result<String> {
    let content = schema_show(engine, wiki_name, type_name)?;
    let schema: serde_json::Value = serde_json::from_str(&content)?;
    Ok(generate_template(&schema, type_name))
}

pub fn schema_add(
    engine: &EngineState,
    wiki_name: &str,
    type_name: &str,
    src_path: &Path,
) -> Result<String> {
    let space = engine.space(wiki_name)?;

    // Validate the schema file
    let content = std::fs::read_to_string(src_path)
        .with_context(|| format!("failed to read: {}", src_path.display()))?;
    let schema_value: serde_json::Value =
        serde_json::from_str(&content).context("file is not valid JSON")?;
    jsonschema::Validator::new(&schema_value)
        .map_err(|e| anyhow::anyhow!("file is not a valid JSON Schema: {e}"))?;

    // Copy to schemas/
    let filename = src_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid path"))?;
    let dest = space.repo_root.join("schemas").join(filename);
    std::fs::copy(src_path, &dest)?;

    // Check if x-wiki-types declares the type
    let has_type = schema_value
        .get("x-wiki-types")
        .and_then(|v| v.as_object())
        .map(|obj| obj.contains_key(type_name))
        .unwrap_or(false);

    let mut msg = format!("copied to {}", dest.display());

    if !has_type {
        // Add wiki.toml override
        let mut wiki_cfg = config::load_wiki(&space.repo_root)?;
        wiki_cfg.types.insert(
            type_name.to_string(),
            config::TypeEntry {
                schema: format!("schemas/{}", filename.to_string_lossy()),
                description: format!("Custom type: {type_name}"),
            },
        );
        config::save_wiki(&wiki_cfg, &space.repo_root)?;
        msg.push_str(&format!(", added [types.{type_name}] to wiki.toml"));
    }

    // Validate index resolution
    if let Err(e) = space_builder::build_space(&space.repo_root, "en_stem") {
        msg.push_str(&format!("\nWARNING: index resolution failed: {e}"));
    }

    Ok(msg)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaRemoveReport {
    pub pages_removed: usize,
    pub pages_deleted_from_disk: usize,
    pub wiki_toml_updated: bool,
    pub schema_file_deleted: bool,
    pub dry_run: bool,
}

pub fn schema_remove(
    manager: &WikiEngine,
    wiki_name: &str,
    type_name: &str,
    delete: bool,
    delete_pages: bool,
    dry_run: bool,
) -> Result<SchemaRemoveReport> {
    if type_name == "default" {
        bail!("cannot remove the 'default' type");
    }

    let engine = manager
        .state
        .read()
        .map_err(|_| anyhow::anyhow!("lock poisoned"))?;
    let space = engine.space(wiki_name)?;

    // Count pages of this type in the index
    let searcher = space.index_manager.searcher()?;
    let list_result = search::list(
        &search::ListOptions {
            r#type: Some(type_name.to_string()),
            ..Default::default()
        },
        &searcher,
        wiki_name,
        &space.index_schema,
    )?;
    let pages_to_remove = list_result.total;

    if dry_run {
        return Ok(SchemaRemoveReport {
            pages_removed: pages_to_remove,
            pages_deleted_from_disk: if delete_pages { pages_to_remove } else { 0 },
            wiki_toml_updated: space
                .type_registry
                .list_types()
                .iter()
                .any(|(n, _)| *n == type_name),
            schema_file_deleted: delete,
            dry_run: true,
        });
    }

    // Remove pages from index
    if pages_to_remove > 0 {
        space
            .index_manager
            .delete_by_type(&space.index_schema, type_name)?;
    }

    // Delete page files from disk if requested
    let mut pages_deleted_from_disk = 0;
    if delete_pages && pages_to_remove > 0 {
        for page in &list_result.pages {
            if markdown::delete_page(&page.slug, &space.wiki_root)? {
                pages_deleted_from_disk += 1;
            }
        }
    }

    // Remove wiki.toml override if present
    let mut wiki_toml_updated = false;
    let mut wiki_cfg = config::load_wiki(&space.repo_root)?;
    if wiki_cfg.types.remove(type_name).is_some() {
        config::save_wiki(&wiki_cfg, &space.repo_root)?;
        wiki_toml_updated = true;
    }

    // Delete schema file if requested
    let mut schema_file_deleted = false;
    if delete {
        if let Some(schema_path) = space.type_registry.schema_path(type_name) {
            let full_path = space.repo_root.join(schema_path);
            if full_path.exists() {
                // Check if other types use this schema
                let content = std::fs::read_to_string(&full_path).unwrap_or_default();
                if let Ok(schema) = serde_json::from_str::<serde_json::Value>(&content) {
                    let wiki_types = schema
                        .get("x-wiki-types")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.len())
                        .unwrap_or(0);
                    if wiki_types <= 1 {
                        std::fs::remove_file(&full_path)?;
                        schema_file_deleted = true;
                    }
                    // If multiple types use this schema, don't delete
                }
            }
        }
    }

    // Auto-commit if configured and changes were made
    let resolved = space.resolved_config(&engine.config);
    let repo_root = space.repo_root.clone();
    if resolved.ingest.auto_commit
        && (pages_deleted_from_disk > 0 || wiki_toml_updated || schema_file_deleted)
    {
        let msg = format!("schema remove: {type_name} — {} pages, wiki.toml={wiki_toml_updated}, schema={schema_file_deleted}",
            pages_deleted_from_disk);
        let _ = git::commit(&repo_root, &msg);
    }

    Ok(SchemaRemoveReport {
        pages_removed: pages_to_remove,
        pages_deleted_from_disk,
        wiki_toml_updated,
        schema_file_deleted,
        dry_run: false,
    })
}

pub fn schema_validate(
    engine: &EngineState,
    wiki_name: &str,
    type_name: Option<&str>,
) -> Result<Vec<String>> {
    let space = engine.space(wiki_name)?;
    let mut issues = Vec::new();

    if let Some(name) = type_name {
        // Validate single type
        if !space.type_registry.is_known(name) {
            bail!("type '{name}' is not registered");
        }
        let schema_path = space
            .type_registry
            .schema_path(name)
            .ok_or_else(|| anyhow::anyhow!("no schema path for type '{name}'"))?;
        let full_path = space.repo_root.join(schema_path);
        validate_schema_file(&full_path, &mut issues);
    } else {
        // Validate all schemas
        let schemas_dir = space.repo_root.join("schemas");
        if schemas_dir.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(&schemas_dir)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
                .collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                validate_schema_file(&entry.path(), &mut issues);
            }
        }
    }

    // Index resolution check
    match space_builder::build_space(&space.repo_root, "en_stem") {
        Ok(_) => {}
        Err(e) => issues.push(format!("index resolution failed: {e}")),
    }

    Ok(issues)
}

fn validate_schema_file(path: &Path, issues: &mut Vec<String>) {
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            issues.push(format!("{filename}: cannot read: {e}"));
            return;
        }
    };

    let schema: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            issues.push(format!("{filename}: invalid JSON: {e}"));
            return;
        }
    };

    if let Err(e) = jsonschema::Validator::new(&schema) {
        issues.push(format!("{filename}: invalid JSON Schema: {e}"));
        return;
    }

    if schema.get("x-wiki-types").is_none() {
        issues.push(format!(
            "{filename}: missing x-wiki-types (types won't be discovered)"
        ));
    }
}

fn generate_template(schema: &serde_json::Value, type_name: &str) -> String {
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let properties = schema
        .get("properties")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut lines = vec!["---".to_string()];

    // Required fields first
    for field in &required {
        if let Some(prop) = properties.get(*field) {
            lines.push(format_template_field(field, prop, type_name));
        }
    }

    // Common optional fields
    for field in &["summary", "status", "last_updated", "tags"] {
        if !required.contains(field) {
            if let Some(prop) = properties.get(*field) {
                lines.push(format_template_field(field, prop, type_name));
            }
        }
    }

    lines.push("---".to_string());
    lines.join("\n")
}

fn format_template_field(name: &str, prop: &serde_json::Value, type_name: &str) -> String {
    let prop_type = prop
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("string");

    match prop_type {
        "array" => {
            if name == "read_when" || name == "tags" {
                format!("{name}:\n  - \"\"")
            } else {
                format!("{name}: []")
            }
        }
        "string" => {
            if name == "type" {
                format!("type: {type_name}")
            } else if name == "status" {
                "status: active".to_string()
            } else if name == "last_updated" {
                format!(
                    "last_updated: \"{}\"",
                    chrono::Utc::now().format("%Y-%m-%d")
                )
            } else {
                format!("{name}: \"\"")
            }
        }
        "boolean" => format!("{name}: false"),
        _ => format!("{name}: \"\""),
    }
}
