//! Frontmatter schema for wiki pages.
//!
//! Every wiki page is a valid Markdown file with a YAML frontmatter block.
//! The wiki generates frontmatter from [`crate::analysis::SuggestedPage`] fields —
//! the external LLM never writes frontmatter directly.

use crate::analysis::{Confidence, PageType, SuggestedPage};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Lifecycle status of a wiki page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageStatus {
    Active,
    Deprecated,
    Stub,
}

/// YAML frontmatter block written at the top of every wiki `.md` file.
///
/// Serialised with `serde_yaml` and delimited by `---` markers.
/// The wiki generates this from [`crate::analysis::SuggestedPage`] fields
/// at ingest time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageFrontmatter {
    /// Human-readable page title.
    pub title: String,
    /// One-sentence summary shown in search results.
    pub summary: String,
    /// Conditions under which an LLM should retrieve this page.
    pub read_when: Vec<String>,
    /// Lifecycle status of this page.
    pub status: PageStatus,
    /// ISO 8601 date of the last update (e.g. `"2026-04-13"`).
    pub last_updated: String,
    /// Page category.
    #[serde(rename = "type")]
    pub page_type: PageType,
    /// Tags for full-text search and cross-referencing.
    pub tags: Vec<String>,
    /// Slugs of source pages this page was derived from.
    pub sources: Vec<String>,
    /// Confidence level of this page's content.
    pub confidence: Confidence,
    /// Slugs of contradiction pages that reference this page.
    pub contradictions: Vec<String>,
    /// One-sentence summary (duplicated from `summary` for agent tooling).
    pub tldr: String,
}

/// Today's date as an ISO 8601 string (e.g. `"2026-04-13"`).
pub fn today_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Build a [`PageFrontmatter`] from a [`SuggestedPage`] at creation time.
///
/// Follows the field mapping in `docs/design/page-content.md`:
/// - `status` defaults to `Active`
/// - `confidence` defaults to `Medium`
/// - `sources` and `contradictions` start empty
/// - `last_updated` is set to today
pub fn frontmatter_from_page(page: &SuggestedPage) -> PageFrontmatter {
    PageFrontmatter {
        title: page.title.clone(),
        summary: page.tldr.clone(),
        tldr: page.tldr.clone(),
        read_when: page.read_when.clone(),
        status: PageStatus::Active,
        last_updated: today_iso8601(),
        page_type: page.page_type.clone(),
        tags: page.tags.clone(),
        sources: vec![],
        confidence: Confidence::Medium,
        contradictions: vec![],
    }
}

/// Split a wiki `.md` file into its [`PageFrontmatter`] and body text.
///
/// Expected format:
/// ```text
/// ---
/// <yaml>
/// ---
///
/// <body>
/// ```
///
/// Returns an error if the file has no frontmatter block or if the YAML is
/// malformed (with the missing field named in the error).
pub fn parse_frontmatter(content: &str) -> Result<(PageFrontmatter, &str)> {
    if !content.starts_with("---\n") {
        return Err(anyhow!(
            "file has no frontmatter block (expected first line to be `---`)"
        ));
    }

    let after_open = &content[4..]; // skip the opening "---\n"

    // Find the closing `---` on its own line: "\n---\n"
    let (yaml_end, body_start) = after_open
        .find("\n---\n")
        .map(|p| (p, p + 5))
        .ok_or_else(|| anyhow!("frontmatter block not closed (missing closing `---`)"))?;

    let yaml_block = &after_open[..yaml_end];
    let after_close = &after_open[body_start..]; // starts just after "---\n"

    // Skip the mandatory blank line that separates frontmatter from body.
    let body = after_close.strip_prefix('\n').unwrap_or(after_close);

    let fm: PageFrontmatter = serde_yaml::from_str(yaml_block)
        .map_err(|e| anyhow!("frontmatter parse error: {e}"))?;

    Ok((fm, body))
}

/// Write a wiki page to `path` as `---\n<yaml>\n---\n\n<body>`.
///
/// Creates parent directories if they do not exist.
/// Normalises CRLF to LF in `body`.
pub fn write_page(path: &Path, frontmatter: &PageFrontmatter, body: &str) -> Result<()> {
    let yaml = serde_yaml::to_string(frontmatter).context("failed to serialize frontmatter")?;
    // serde_yaml 0.9 may prefix the output with "---\n"; strip it since we
    // write our own `---` delimiters.
    let yaml_content = yaml.strip_prefix("---\n").unwrap_or(&yaml);

    // Normalise CRLF → LF.
    let body_lf: std::borrow::Cow<str> = if body.contains("\r\n") {
        body.replace("\r\n", "\n").into()
    } else {
        body.into()
    };

    let content = format!("---\n{}---\n\n{}", yaml_content, body_lf);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    std::fs::write(path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}

// ── Bundle / slug helpers ─────────────────────────────────────────────────────

/// Derive the slug for a wiki page file.
///
/// - `index.md` inside a folder → slug = parent dir relative to `wiki_root`
/// - any other `.md` → slug = path without extension relative to `wiki_root`
pub fn slug_for(path: &Path, wiki_root: &Path) -> String {
    let rel = path.strip_prefix(wiki_root).unwrap_or(path);
    if rel.file_name().map(|n| n == "index.md").unwrap_or(false) {
        rel.parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    } else {
        rel.with_extension("")
            .to_string_lossy()
            .replace('\\', "/")
    }
}

/// Resolve a slug to its file path.
///
/// Checks `{slug}.md` first, then `{slug}/index.md`.
/// Returns `None` if neither exists.
pub fn resolve_slug(wiki_root: &Path, slug: &str) -> Option<PathBuf> {
    let flat = wiki_root.join(format!("{slug}.md"));
    if flat.exists() {
        return Some(flat);
    }
    let bundle = wiki_root.join(slug).join("index.md");
    if bundle.exists() {
        return Some(bundle);
    }
    None
}

/// Promote a flat `{slug}.md` to a bundle `{slug}/index.md`.
///
/// No-op if the bundle already exists. Returns an error if neither form exists.
pub fn promote_to_bundle(wiki_root: &Path, slug: &str) -> Result<()> {
    let bundle_dir = wiki_root.join(slug);
    let bundle_index = bundle_dir.join("index.md");
    if bundle_index.exists() {
        return Ok(());
    }
    let flat = wiki_root.join(format!("{slug}.md"));
    if !flat.exists() {
        anyhow::bail!("promote_to_bundle: neither `{slug}.md` nor `{slug}/index.md` exists");
    }
    std::fs::create_dir_all(&bundle_dir)
        .with_context(|| format!("failed to create bundle dir {}", bundle_dir.display()))?;
    std::fs::rename(&flat, &bundle_index)
        .with_context(|| format!("failed to move {} to {}", flat.display(), bundle_index.display()))?;
    Ok(())
}

/// Return `true` if `{slug}/index.md` exists (bundle form).
pub fn is_bundle(wiki_root: &Path, slug: &str) -> bool {
    wiki_root.join(slug).join("index.md").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Confidence, PageType};

    fn sample_frontmatter() -> PageFrontmatter {
        PageFrontmatter {
            title: "Mixture of Experts".into(),
            summary: "Sparse routing of tokens to expert subnetworks.".into(),
            read_when: vec!["Reasoning about MoE architecture".into()],
            status: PageStatus::Active,
            last_updated: "2026-04-13".into(),
            page_type: PageType::Concept,
            tags: vec!["transformers".into(), "scaling".into()],
            sources: vec!["sources/switch-transformer-2021".into()],
            confidence: Confidence::High,
            contradictions: vec![],
            tldr: "Sparse routing of tokens to expert subnetworks.".into(),
        }
    }

    #[test]
    fn page_frontmatter_yaml_round_trip() {
        let fm = sample_frontmatter();
        let yaml = serde_yaml::to_string(&fm).expect("serialise");
        let round_tripped: PageFrontmatter = serde_yaml::from_str(&yaml).expect("deserialise");
        assert_eq!(fm, round_tripped);
    }

    #[test]
    fn write_then_parse_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.md");
        let fm = sample_frontmatter();
        let body = "## Overview\n\nTest content.\n";

        write_page(&path, &fm, body).expect("write");

        let content = std::fs::read_to_string(&path).unwrap();
        let (parsed_fm, parsed_body) = parse_frontmatter(&content).expect("parse");

        assert_eq!(parsed_fm, fm);
        assert_eq!(parsed_body, body);
    }

    #[test]
    fn write_page_starts_with_frontmatter_delimiters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("page.md");
        let fm = sample_frontmatter();

        write_page(&path, &fm, "body").expect("write");
        let content = std::fs::read_to_string(&path).unwrap();

        assert!(content.starts_with("---\n"), "must start with ---");
        assert!(content.contains("title:"), "must contain title field");
        assert!(content.contains("tags:"), "must contain tags field");
    }

    #[test]
    fn parse_frontmatter_no_block_returns_error() {
        let result = parse_frontmatter("# No frontmatter here\n\nbody text");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no frontmatter"), "error: {msg}");
    }

    #[test]
    fn parse_frontmatter_unclosed_returns_error() {
        let result = parse_frontmatter("---\ntitle: foo\n");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not closed"), "error: {msg}");
    }

    #[test]
    fn parse_frontmatter_missing_required_field_returns_error() {
        // PageFrontmatter has many required fields; omitting most should fail.
        let bad = "---\ntitle: only title here\n---\n\nbody\n";
        let result = parse_frontmatter(bad);
        assert!(result.is_err(), "expected error for missing fields");
    }

    #[test]
    fn parse_frontmatter_correct_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.md");
        let fm = sample_frontmatter();
        write_page(&path, &fm, "body").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

        let (parsed, _body) = parse_frontmatter(&content).unwrap();
        assert_eq!(parsed.title, "Mixture of Experts");
        assert_eq!(parsed.tags, vec!["transformers", "scaling"]);
        assert_eq!(parsed.status, PageStatus::Active);
    }

    #[test]
    fn frontmatter_from_page_sets_defaults() {
        use crate::analysis::Action;
        let page = SuggestedPage {
            slug: "concepts/test".into(),
            title: "Test".into(),
            page_type: PageType::Concept,
            action: Action::Create,
            tldr: "A test page.".into(),
            body: "body".into(),
            tags: vec!["test".into()],
            read_when: vec!["Testing".into()],
        };
        let fm = frontmatter_from_page(&page);
        assert_eq!(fm.title, "Test");
        assert_eq!(fm.summary, "A test page.");
        assert_eq!(fm.tldr, "A test page.");
        assert_eq!(fm.status, PageStatus::Active);
        assert_eq!(fm.confidence, Confidence::Medium);
        assert!(fm.sources.is_empty());
        assert!(fm.contradictions.is_empty());
    }
}
