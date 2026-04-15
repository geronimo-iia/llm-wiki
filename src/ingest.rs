//! Ingest pipeline — deserialise an [`Analysis`] JSON document, validate it,
//! integrate it into the wiki, and commit via [`crate::git`].

use crate::analysis::Analysis;
use crate::config::WikiConfig;
use crate::{git, integrate, search};
use anyhow::{Context, Result};
use std::io::Read as _;
use std::path::PathBuf;

/// Source of the analysis JSON: a file path or stdin.
pub enum Input {
    /// Read from the file at this path.
    File(PathBuf),
    /// Read from standard input.
    Stdin,
}

/// Summary of what a single ingest session wrote.
#[derive(Debug, Clone)]
pub struct IngestReport {
    /// Number of new pages created.
    pub pages_created: usize,
    /// Number of existing pages replaced.
    pub pages_updated: usize,
    /// Number of existing pages extended.
    pub pages_appended: usize,
    /// Number of contradiction pages written.
    pub contradictions_written: usize,
    /// Number of flat pages promoted to bundles.
    pub bundles_created: usize,
    /// Title of the ingested document.
    pub title: String,
    /// Whether the search index was updated incrementally after this ingest.
    pub index_updated: bool,
    /// Slugs of all pages written during this ingest (used for index update).
    pub changed_slugs: Vec<String>,
}

impl IngestReport {
    /// Total number of page-level changes (created + updated + appended).
    pub fn total_pages(&self) -> usize {
        self.pages_created + self.pages_updated + self.pages_appended
    }
}

impl std::fmt::Display for IngestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Ingested: {}", self.title)?;
        writeln!(f, "  created:        {}", self.pages_created)?;
        writeln!(f, "  updated:        {}", self.pages_updated)?;
        writeln!(f, "  appended:       {}", self.pages_appended)?;
        write!(f, "  contradictions: {}", self.contradictions_written)
    }
}

/// Parse and validate an `analysis.json` string.
///
/// Returns a clear error on malformed JSON (with line/column) or on unknown
/// enum variants (`doc_type`, `action`). Serde provides these automatically.
pub fn parse_analysis(json: &str) -> Result<Analysis> {
    serde_json::from_str(json).map_err(|e| {
        anyhow::anyhow!(
            "invalid analysis JSON at line {}, column {}: {}",
            e.line(),
            e.column(),
            e
        )
    })
}

/// Read JSON from `input`, validate, integrate into the wiki, and commit.
///
/// Exits with an error if:
/// - JSON is malformed
/// - a `create` action targets an existing slug
/// - an `update`/`append` action targets a missing slug
/// - a slug contains path traversal or an unknown prefix
pub async fn ingest(input: Input, config: &WikiConfig) -> Result<IngestReport> {
    let json = match input {
        Input::File(ref path) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?,
        Input::Stdin => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("failed to read from stdin")?;
            buf
        }
    };

    let analysis = parse_analysis(&json)?;

    let wiki_root = &config.root;
    git::init_if_needed(wiki_root)?;

    let report = integrate::integrate(analysis, wiki_root)?;

    let commit_msg = format!("ingest: {} — +{} pages", report.title, report.total_pages());
    git::commit(wiki_root, &commit_msg)?;

    let index_dir = wiki_root.join(".wiki").join("search-index");
    let index_updated = if index_dir.exists() {
        search::update_index(wiki_root, &index_dir, &report.changed_slugs).is_ok()
    } else {
        false
    };

    Ok(IngestReport {
        pages_created: report.pages_created,
        pages_updated: report.pages_updated,
        pages_appended: report.pages_appended,
        contradictions_written: report.contradictions_written,
        bundles_created: report.bundles_created,
        title: report.title,
        index_updated,
        changed_slugs: report.changed_slugs,
    })
}
