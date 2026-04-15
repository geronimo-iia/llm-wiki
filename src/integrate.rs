//! Integration layer — write pages and contradiction nodes to the wiki.
//!
//! Called by [`crate::ingest::ingest`] after validation. Writes `.md` files
//! according to each [`crate::analysis::SuggestedPage`]'s `action` field, then
//! writes any [`crate::analysis::Contradiction`] pages. Does **not** commit —
//! the caller is responsible for the git commit.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

use crate::analysis::{Action, Analysis, Contradiction, Dimension, Status};
use crate::ingest::IngestReport;
use crate::markdown::{frontmatter_from_page, parse_frontmatter, promote_to_bundle, today_iso8601, write_page};

/// Slug prefixes that the wiki accepts.
const VALID_PREFIXES: &[&str] = &["concepts/", "sources/", "queries/", "contradictions/"];

/// Validate that a slug is safe and has a recognised prefix.
///
/// Rejects path traversal (`../`) and unknown category prefixes.
pub fn validate_slug(slug: &str) -> Result<()> {
    if slug.contains("../") || slug.starts_with('/') {
        bail!("slug `{slug}` contains path traversal (`../`) or is absolute");
    }
    if !VALID_PREFIXES.iter().any(|p| slug.starts_with(p)) {
        bail!(
            "slug `{slug}` has unknown prefix; expected one of: {}",
            VALID_PREFIXES.join(", ")
        );
    }
    Ok(())
}

/// Slugify a title for use in a contradiction file name.
fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// YAML frontmatter for a contradiction page.
///
/// Written by [`write_contradiction_page`]; schema mirrors `docs/design/design.md`.
#[derive(Debug, Serialize, Deserialize)]
struct ContradictionFrontmatter {
    title: String,
    #[serde(rename = "type")]
    page_type: String,
    claim_a: String,
    source_a: String,
    claim_b: String,
    source_b: String,
    dimension: Dimension,
    epistemic_value: String,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,
    tags: Vec<String>,
    created: String,
    updated: String,
}

/// Write a contradiction page to `path`.
fn write_contradiction_page(path: &Path, c: &Contradiction) -> Result<()> {
    let fm = ContradictionFrontmatter {
        title: c.title.clone(),
        page_type: "contradiction".into(),
        claim_a: c.claim_a.clone(),
        source_a: c.source_a.clone(),
        claim_b: c.claim_b.clone(),
        source_b: c.source_b.clone(),
        dimension: c.dimension.clone(),
        epistemic_value: c.epistemic_value.clone(),
        status: c.status.clone(),
        resolution: c.resolution.clone(),
        tags: vec![],
        created: today_iso8601(),
        updated: today_iso8601(),
    };

    let yaml = serde_yaml::to_string(&fm).context("failed to serialise contradiction frontmatter")?;
    let yaml_content = yaml.strip_prefix("---\n").unwrap_or(&yaml);

    let body = format!(
        "## Claim A\n\n{}\n\n## Claim B\n\n{}\n\n## Analysis\n\n{}\n",
        c.claim_a, c.claim_b, c.epistemic_value
    );

    let content = format!("---\n{}---\n\n{}", yaml_content, body);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

/// Map an asset `kind` string to its `assets/` subdirectory.
fn kind_to_subdir(kind: &str) -> &'static str {
    match kind {
        "image" => "diagrams",
        "yaml" | "toml" | "json" => "configs",
        "script" => "scripts",
        "data" => "data",
        _ => "other",
    }
}

/// Infer asset kind from file extension.
fn kind_from_ext(filename: &str) -> &'static str {
    match filename.rsplit('.').next().unwrap_or("") {
        "png" | "jpg" | "svg" | "gif" => "image",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "json" => "json",
        "py" | "sh" | "rs" | "js" => "script",
        "csv" | "tsv" | "jsonl" => "data",
        _ => "other",
    }
}

/// Write an asset co-located with its page bundle.
///
/// Promotes the page to a bundle if it is currently flat.
/// Writes `{wiki_root}/{page_slug}/{filename}`.
pub fn write_asset_colocated(
    wiki_root: &Path,
    page_slug: &str,
    filename: &str,
    content: &[u8],
) -> Result<()> {
    promote_to_bundle(wiki_root, page_slug)
        .with_context(|| format!("failed to promote `{page_slug}` to bundle"))?;
    let dest = wiki_root.join(page_slug).join(filename);
    std::fs::write(&dest, content)
        .with_context(|| format!("failed to write asset {}", dest.display()))?;
    Ok(())
}

/// Write a shared asset to `assets/{subdir}/{filename}` and update `assets/index.md`.
pub fn write_asset_shared(
    wiki_root: &Path,
    kind: &str,
    filename: &str,
    content: &[u8],
) -> Result<()> {
    let subdir = kind_to_subdir(kind);
    let dir = wiki_root.join("assets").join(subdir);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;
    let dest = dir.join(filename);
    std::fs::write(&dest, content)
        .with_context(|| format!("failed to write shared asset {}", dest.display()))?;
    regenerate_assets_index(wiki_root)
}

/// Return the path to `assets/index.md`.
pub fn assets_index_path(wiki_root: &Path) -> std::path::PathBuf {
    wiki_root.join("assets").join("index.md")
}

/// Rebuild `assets/index.md` from all files under `assets/` (excluding `index.md`).
pub fn regenerate_assets_index(wiki_root: &Path) -> Result<()> {
    let assets_dir = wiki_root.join("assets");
    if !assets_dir.exists() {
        return Ok(());
    }

    let mut rows: Vec<String> = Vec::new();
    for entry in WalkDir::new(&assets_dir).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = match path.strip_prefix(wiki_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if rel_str == "assets/index.md" {
            continue;
        }
        let filename = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        let kind = kind_from_ext(&filename);
        let slug = rel.with_extension("").to_string_lossy().replace('\\', "/");
        rows.push(format!("| {slug} | {kind} | | |"));
    }
    rows.sort();

    let mut md = String::from("# Shared Assets\n\n");
    md.push_str("| slug | kind | caption | referenced_by |\n");
    md.push_str("|------|------|---------|---------------|\n");
    for row in &rows {
        md.push_str(row);
        md.push('\n');
    }

    let index_path = assets_index_path(wiki_root);
    std::fs::write(&index_path, md)
        .with_context(|| format!("failed to write {}", index_path.display()))?;
    Ok(())
}

/// Write all pages and contradictions from `analysis` into `wiki_root`.
///
/// Does **not** commit — call [`crate::git::commit`] afterwards.
/// Returns an [`IngestReport`] with counts for each action type and the
/// slugs of all pages written (for incremental index update).
pub fn integrate(analysis: Analysis, wiki_root: &Path) -> Result<IngestReport> {
    let mut report = IngestReport {
        pages_created: 0,
        pages_updated: 0,
        pages_appended: 0,
        contradictions_written: 0,
        bundles_created: 0,
        title: analysis.title.clone(),
        index_updated: false,
        changed_slugs: Vec::new(),
    };

    // Validate all slugs before writing anything — fail fast.
    for page in &analysis.suggested_pages {
        validate_slug(&page.slug)
            .with_context(|| format!("invalid slug in suggested_pages: `{}`", page.slug))?;
    }

    // ── Process suggested pages ──────────────────────────────────────────────
    for page in &analysis.suggested_pages {
        let path = wiki_root.join(format!("{}.md", page.slug));

        // Ensure parent directory exists before writing.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        match &page.action {
            Action::Create => {
                if path.exists() {
                    bail!(
                        "action `create` failed: `{}` already exists",
                        path.display()
                    );
                }
                let fm = frontmatter_from_page(page);
                write_page(&path, &fm, &page.body)?;
                report.pages_created += 1;
                report.changed_slugs.push(page.slug.clone());
            }

            Action::Update => {
                if !path.exists() {
                    bail!(
                        "action `update` failed: `{}` does not exist (use `create` instead)",
                        path.display()
                    );
                }
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let (mut fm, _old_body) = parse_frontmatter(&content)
                    .with_context(|| format!("failed to parse frontmatter of {}", path.display()))?;

                // OVERWRITE: title, summary, tldr, read_when (replaced by latest)
                fm.title = page.title.clone();
                fm.summary = page.tldr.clone();
                fm.tldr = page.tldr.clone();
                // read_when is also overwritten then unioned below.

                // UNION: tags — accumulate, never shrink
                for tag in &page.tags {
                    if !fm.tags.contains(tag) {
                        fm.tags.push(tag.clone());
                    }
                }

                // UNION: read_when — accumulate, never shrink
                for hint in &page.read_when {
                    if !fm.read_when.contains(hint) {
                        fm.read_when.push(hint.clone());
                    }
                }
                // If old read_when was empty, use the new one as base.
                if fm.read_when.is_empty() {
                    fm.read_when = page.read_when.clone();
                }

                // PRESERVE: sources, contradictions, status (not touched above)
                // SET: last_updated
                fm.last_updated = today_iso8601();

                write_page(&path, &fm, &page.body)?;
                report.pages_updated += 1;
                report.changed_slugs.push(page.slug.clone());
            }

            Action::Append => {
                if !path.exists() {
                    bail!(
                        "action `append` failed: `{}` does not exist (use `create` instead)",
                        path.display()
                    );
                }
                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                let (mut fm, old_body) = parse_frontmatter(&content)
                    .with_context(|| format!("failed to parse frontmatter of {}", path.display()))?;

                // UNION: tags
                for tag in &page.tags {
                    if !fm.tags.contains(tag) {
                        fm.tags.push(tag.clone());
                    }
                }

                // UNION: read_when
                for hint in &page.read_when {
                    if !fm.read_when.contains(hint) {
                        fm.read_when.push(hint.clone());
                    }
                }

                // PRESERVE: title, summary, tldr, sources, contradictions, status
                // SET: last_updated
                fm.last_updated = today_iso8601();

                // Append new section after the existing body, separated by `---`.
                let new_body = format!("{}\n\n---\n\n{}", old_body.trim_end(), page.body);
                write_page(&path, &fm, &new_body)?;
                report.pages_appended += 1;
                report.changed_slugs.push(page.slug.clone());
            }
        }
    }

    // ── Write contradiction pages ────────────────────────────────────────────
    for contradiction in &analysis.contradictions {
        let slug = slugify(&contradiction.title);
        let path = wiki_root.join("contradictions").join(format!("{slug}.md"));
        write_contradiction_page(&path, contradiction)?;
        report.contradictions_written += 1;
        report.changed_slugs.push(format!("contradictions/{slug}"));
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Action, Confidence, PageType, SuggestedPage};

    fn make_analysis(pages: Vec<SuggestedPage>) -> Analysis {
        Analysis {
            source: "test".into(),
            doc_type: crate::analysis::DocType::Note,
            title: "Test".into(),
            language: "en".into(),
            claims: vec![],
            concepts: vec![],
            key_quotes: vec![],
            data_gaps: vec![],
            suggested_pages: pages,
            contradictions: vec![],
        }
    }

    fn concept_page(slug: &str, action: Action) -> SuggestedPage {
        SuggestedPage {
            slug: slug.into(),
            title: "Test Concept".into(),
            page_type: PageType::Concept,
            action,
            tldr: "A test concept.".into(),
            body: "## Overview\n\nContent here.\n".into(),
            tags: vec!["test".into()],
            read_when: vec!["Testing".into()],
        }
    }

    #[test]
    fn create_writes_file() {
        let dir = tempfile::tempdir().unwrap();
        let analysis = make_analysis(vec![concept_page("concepts/test", Action::Create)]);
        let report = integrate(analysis, dir.path()).unwrap();

        assert_eq!(report.pages_created, 1);
        assert!(dir.path().join("concepts/test.md").exists());
    }

    #[test]
    fn create_on_existing_slug_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let analysis = make_analysis(vec![concept_page("concepts/test", Action::Create)]);
        integrate(analysis.clone(), dir.path()).unwrap();

        let result = integrate(analysis, dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("already exists"), "error: {msg}");
    }

    #[test]
    fn update_replaces_body_preserves_sources() {
        let dir = tempfile::tempdir().unwrap();

        // Create first.
        integrate(
            make_analysis(vec![concept_page("concepts/test", Action::Create)]),
            dir.path(),
        )
        .unwrap();

        // Manually add a source to the frontmatter to verify preservation.
        let path = dir.path().join("concepts/test.md");
        let content = std::fs::read_to_string(&path).unwrap();
        let (mut fm, _body) = parse_frontmatter(&content).unwrap();
        fm.sources.push("sources/some-source".into());
        write_page(&path, &fm, "old body").unwrap();

        // Now update.
        let mut update_page = concept_page("concepts/test", Action::Update);
        update_page.body = "## New body\n\nReplaced.\n".into();
        integrate(make_analysis(vec![update_page]), dir.path()).unwrap();

        let updated = std::fs::read_to_string(&path).unwrap();
        let (fm2, body2) = parse_frontmatter(&updated).unwrap();

        assert!(body2.contains("Replaced"), "body should be replaced");
        assert!(!body2.contains("old body"), "old body should be gone");
        assert_eq!(fm2.sources, vec!["sources/some-source"], "sources preserved");
    }

    #[test]
    fn append_adds_section_keeps_original_body() {
        let dir = tempfile::tempdir().unwrap();
        integrate(
            make_analysis(vec![concept_page("concepts/test", Action::Create)]),
            dir.path(),
        )
        .unwrap();

        let mut append_page = concept_page("concepts/test", Action::Append);
        append_page.body = "## New findings\n\nExtra content.\n".into();
        let report = integrate(make_analysis(vec![append_page]), dir.path()).unwrap();

        assert_eq!(report.pages_appended, 1);
        let content = std::fs::read_to_string(dir.path().join("concepts/test.md")).unwrap();
        assert!(content.contains("Content here"), "original body present");
        assert!(content.contains("Extra content"), "appended body present");
    }

    #[test]
    fn contradictions_written_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let mut analysis = make_analysis(vec![]);
        analysis.contradictions.push(Contradiction {
            title: "Test Contradiction".into(),
            claim_a: "Claim A".into(),
            source_a: "sources/a".into(),
            claim_b: "Claim B".into(),
            source_b: "sources/b".into(),
            dimension: Dimension::Context,
            epistemic_value: "Important insight.".into(),
            status: Status::Active,
            resolution: None,
        });

        let report = integrate(analysis, dir.path()).unwrap();
        assert_eq!(report.contradictions_written, 1);
        assert!(dir.path().join("contradictions/test-contradiction.md").exists());
    }

    #[test]
    fn empty_contradictions_writes_no_files() {
        let dir = tempfile::tempdir().unwrap();
        integrate(make_analysis(vec![]), dir.path()).unwrap();
        assert!(!dir.path().join("contradictions").exists());
    }

    #[test]
    fn slug_with_path_traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let analysis = make_analysis(vec![concept_page("../evil/path", Action::Create)]);
        let result = integrate(analysis, dir.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("path traversal") || msg.contains("invalid slug"), "error: {msg}");
    }
}
