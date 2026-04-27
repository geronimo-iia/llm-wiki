use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::ValidationConfig;
use crate::frontmatter;
use crate::git;
use crate::type_registry::SpaceTypeRegistry;

/// Normalize line endings: CRLF → LF, lone CR → LF.
pub fn normalize_line_endings(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

#[derive(Debug, Clone, Default)]
pub struct IngestOptions {
    pub dry_run: bool,
    pub auto_commit: bool,
    /// When `Some`, only files in this set are validated; others increment `unchanged_count`.
    /// When `None`, all files are validated.
    pub changed_paths: Option<HashSet<PathBuf>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestReport {
    pub pages_validated: usize,
    pub assets_found: usize,
    pub warnings: Vec<String>,
    pub commit: String,
    #[serde(default)]
    pub unchanged_count: usize,
}

pub fn ingest(
    path: &Path,
    options: &IngestOptions,
    wiki_root: &Path,
    registry: &SpaceTypeRegistry,
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

    if full_path.is_file() {
        let skip = should_skip(&full_path, wiki_root, &options.changed_paths);
        if skip {
            report.unchanged_count += 1;
        } else {
            validate_file(&full_path, registry, validation, &mut report)?;
        }
    } else {
        for entry in WalkDir::new(&full_path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    if should_skip(p, wiki_root, &options.changed_paths) {
                        report.unchanged_count += 1;
                    } else {
                        validate_file(p, registry, validation, &mut report)?;
                    }
                } else {
                    report.assets_found += 1;
                }
            }
        }
    }

    if !options.dry_run && options.auto_commit {
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

fn should_skip(abs_path: &Path, wiki_root: &Path, changed: &Option<HashSet<PathBuf>>) -> bool {
    let Some(set) = changed else { return false };
    if set.is_empty() {
        return false;
    }
    let rel = abs_path.strip_prefix(wiki_root).unwrap_or(abs_path);
    !set.contains(rel)
}

fn validate_file(
    path: &Path,
    registry: &SpaceTypeRegistry,
    validation: &ValidationConfig,
    report: &mut IngestReport,
) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let content = normalize_line_endings(&raw);

    let page = frontmatter::parse(&content);

    // No frontmatter — warn but count as validated
    if page.frontmatter.is_empty() {
        report
            .warnings
            .push(format!("{}: no frontmatter found", path.display()));
        report.pages_validated += 1;
        return Ok(());
    }

    // Validate base fields via type registry
    let warnings = registry.validate(&page.frontmatter, &validation.type_strictness)?;
    for w in warnings {
        report.warnings.push(format!("{}: {}", path.display(), w));
    }

    report.pages_validated += 1;
    Ok(())
}
