use std::path::Path;

use anyhow::{bail, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::{SchemaConfig, ValidationConfig};
use crate::frontmatter::{
    generate_minimal_frontmatter, parse_frontmatter, title_from_body_or_filename,
    validate_frontmatter, write_frontmatter,
};
use crate::git;

#[derive(Debug, Clone, Default)]
pub struct IngestOptions {
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestReport {
    pub pages_validated: usize,
    pub assets_found: usize,
    pub warnings: Vec<String>,
    pub commit: String,
}

pub fn ingest(
    path: &Path,
    options: &IngestOptions,
    wiki_root: &Path,
    schema: &SchemaConfig,
    validation: &ValidationConfig,
) -> Result<IngestReport> {
    let repo_root = wiki_root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("wiki_root has no parent"))?;

    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        wiki_root.join(path)
    };

    if !full_path.exists() {
        bail!("path does not exist: {}", full_path.display());
    }

    // Reject path traversal
    let canonical = full_path.canonicalize()?;
    let canonical_root = wiki_root.canonicalize()?;
    if !canonical.starts_with(&canonical_root) {
        bail!("path is outside wiki root");
    }

    let mut report = IngestReport::default();
    let today = Local::now().format("%Y-%m-%d").to_string();

    if full_path.is_file() {
        process_file(&full_path, &today, schema, validation, &mut report)?;
    } else {
        for entry in WalkDir::new(&full_path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    process_file(p, &today, schema, validation, &mut report)?;
                } else {
                    report.assets_found += 1;
                }
            }
        }
    }

    if !options.dry_run {
        let msg = format!(
            "ingest: {} — +{} pages, +{} assets",
            path.display(),
            report.pages_validated,
            report.assets_found
        );
        let hash = git::commit(repo_root, &msg)?;
        report.commit = hash;
    }

    Ok(report)
}

fn process_file(
    path: &Path,
    today: &str,
    schema: &SchemaConfig,
    validation: &ValidationConfig,
    report: &mut IngestReport,
) -> Result<()> {
    let content = std::fs::read_to_string(path)?;

    if !content.starts_with("---") {
        // No frontmatter — generate minimal
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let title = title_from_body_or_filename(&content, &filename);
        let mut fm = generate_minimal_frontmatter(&title);
        fm.last_updated = today.to_string();
        let output = write_frontmatter(&fm, &content);
        std::fs::write(path, output)?;
        report.pages_validated += 1;
        return Ok(());
    }

    let (mut fm, body) = parse_frontmatter(&content)?;

    if fm.title.is_empty() {
        bail!("title field is missing or empty: {}", path.display());
    }

    // Set defaults for missing fields
    if fm.status.is_empty() {
        fm.status = "active".into();
        report
            .warnings
            .push(format!("{}: status missing, set to active", path.display()));
    }
    if fm.r#type.is_empty() {
        fm.r#type = "page".into();
        report
            .warnings
            .push(format!("{}: type missing, set to page", path.display()));
    }

    // Always set last_updated
    fm.last_updated = today.to_string();

    // Validate frontmatter
    let warnings = validate_frontmatter(&fm, schema, &validation.type_strictness)?;
    for w in warnings {
        report.warnings.push(format!("{}: {}", path.display(), w));
    }

    let output = write_frontmatter(&fm, &body);
    std::fs::write(path, output)?;
    report.pages_validated += 1;
    Ok(())
}
