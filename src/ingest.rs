use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::{RedactConfig, ValidationConfig};
use crate::frontmatter;
use crate::git;
use crate::ops::redact::{RedactionMatch, RedactionReport, redact_body};
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
    /// When `Some`, run redaction pass on each file body before validation.
    pub redact: Option<RedactConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IngestReport {
    pub pages_validated: usize,
    pub assets_found: usize,
    pub warnings: Vec<String>,
    pub commit: String,
    #[serde(default)]
    pub unchanged_count: usize,
    #[serde(default)]
    pub redacted: Vec<RedactionReport>,
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
            validate_file(
                &full_path,
                wiki_root,
                registry,
                validation,
                options.redact.as_ref(),
                &mut report,
            )?;
        }
    } else {
        for entry in WalkDir::new(&full_path).into_iter().filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() {
                if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    if should_skip(p, wiki_root, &options.changed_paths) {
                        report.unchanged_count += 1;
                    } else {
                        validate_file(
                            p,
                            wiki_root,
                            registry,
                            validation,
                            options.redact.as_ref(),
                            &mut report,
                        )?;
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

fn slug_from_path(abs_path: &Path, wiki_root: &Path) -> String {
    abs_path
        .strip_prefix(wiki_root)
        .unwrap_or(abs_path)
        .with_extension("")
        .to_string_lossy()
        .into_owned()
}

fn validate_file(
    path: &Path,
    wiki_root: &Path,
    registry: &SpaceTypeRegistry,
    validation: &ValidationConfig,
    redact_cfg: Option<&RedactConfig>,
    report: &mut IngestReport,
) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let mut content = normalize_line_endings(&raw);

    // Redaction pass — body only, before validation
    if let Some(cfg) = redact_cfg {
        let parsed = frontmatter::parse(&content);
        let separator = "---";
        // Find where body starts (after the closing frontmatter delimiter)
        let body_start = if content.starts_with(separator) {
            // skip first "---", find closing "---"
            let after_open = &content[3..];
            after_open
                .find("\n---")
                .map(|pos| 3 + pos + 4 + 1)
                .unwrap_or(0)
        } else {
            0
        };

        if body_start > 0 && body_start <= content.len() {
            let front = &content[..body_start];
            let body = &content[body_start..];
            let (redacted_body, matches) = redact_body(body, cfg);
            if !matches.is_empty() {
                let slug = slug_from_path(path, wiki_root);
                // Adjust line numbers by frontmatter line count
                let fm_lines = front.lines().count();
                let adjusted: Vec<RedactionMatch> = matches
                    .into_iter()
                    .map(|m| RedactionMatch {
                        pattern_name: m.pattern_name,
                        line_number: m.line_number + fm_lines,
                    })
                    .collect();
                report.redacted.push(RedactionReport {
                    slug,
                    matches: adjusted,
                });
                std::fs::write(path, format!("{front}{redacted_body}"))?;
                content = normalize_line_endings(&std::fs::read_to_string(path)?);
            }
        } else {
            // No frontmatter — redact the whole file
            let (redacted, matches) = redact_body(&content, cfg);
            if !matches.is_empty() {
                let slug = slug_from_path(path, wiki_root);
                report.redacted.push(RedactionReport { slug, matches });
                std::fs::write(path, &redacted)?;
                content = normalize_line_endings(&redacted);
            }
        }
        let _ = parsed; // parsed only used to determine frontmatter presence above
    }

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
