//! Structural lint pass — orphan pages, missing concept stubs, active contradictions.
//!
//! `wiki lint` writes a `LINT.md` report and commits it. The external LLM reads
//! the report and re-ingests enriched analysis as needed.

use crate::analysis::Status;
use crate::contradiction::{list as list_contradictions, ContradictionSummary};
use crate::graph::{build_graph, missing_stubs, orphans};
use crate::markdown::slug_for;
use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use walkdir::WalkDir;

// ── Public types ──────────────────────────────────────────────────────────────

/// Output of a lint pass.
pub struct LintReport {
    /// Pages with no incoming links (petgraph in-degree = 0).
    pub orphan_pages: Vec<String>,
    /// Concept slugs referenced from other pages but not yet created on disk.
    pub missing_stubs: Vec<String>,
    /// Contradiction pages whose status is `active` or `under-analysis`.
    pub active_contradictions: Vec<ContradictionSummary>,
    /// Bundle pages with `./asset` references that don't exist in the bundle folder.
    pub orphan_asset_refs: Vec<String>,
}

// ── orphan asset ref detection ────────────────────────────────────────────────

/// Collect `{slug}/{filename}` entries where a bundle page references `./filename`
/// but the file does not exist beside `index.md`.
fn collect_orphan_asset_refs(wiki_root: &Path) -> Vec<String> {
    let mut results = Vec::new();

    for entry in WalkDir::new(wiki_root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.file_name() != Some(OsStr::new("index.md")) {
            continue;
        }
        let rel = match path.strip_prefix(wiki_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if rel.starts_with(".wiki") {
            continue;
        }

        let bundle_dir = match path.parent() {
            Some(p) => p,
            None => continue,
        };
        let slug = slug_for(path, wiki_root);

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Only scan the body (after frontmatter) for asset references.
        let body = if content.starts_with("---\n") {
            if let Some(after_open) = content.strip_prefix("---\n") {
                if let Some(end) = after_open.find("\n---\n") {
                    let after_close = &after_open[end + 5..];
                    after_close.strip_prefix('\n').unwrap_or(after_close)
                } else {
                    &content
                }
            } else {
                &content
            }
        } else {
            &content
        };

        // Find all `./filename` references in the body.
        for cap in body.split("](./") {
            // Each split after the first starts with `filename)` or `filename "title")`
            let end = cap.find(|c: char| c == ')' || c == ' ' || c == '"');
            if let Some(end) = end {
                let filename = &cap[..end];
                if filename.is_empty() || filename.contains('/') {
                    continue;
                }
                let asset_path = bundle_dir.join(filename);
                if !asset_path.exists() {
                    results.push(format!("{slug}/{filename}"));
                }
            }
        }
    }

    results.sort();
    results.dedup();
    results
}

// ── write_lint_report ─────────────────────────────────────────────────────────

/// Write a `LINT.md` report to `wiki_root/LINT.md`.
pub fn write_lint_report(wiki_root: &Path, report: &LintReport) -> Result<()> {
    let mut md = String::new();

    md.push_str("# Lint Report\n\n");
    md.push_str(&format!(
        "_Generated: {}_\n\n",
        chrono::Utc::now().format("%Y-%m-%d")
    ));

    // ── Orphans ──────────────────────────────────────────────────────────────
    md.push_str("## Orphans\n\n");
    md.push_str("Pages with no inbound links. Consider adding cross-references or removing them.\n\n");
    if report.orphan_pages.is_empty() {
        md.push_str("_None._\n\n");
    } else {
        for slug in &report.orphan_pages {
            md.push_str(&format!("- `{slug}`\n"));
        }
        md.push('\n');
    }

    // ── Missing Stubs ────────────────────────────────────────────────────────
    md.push_str("## Missing Stubs\n\n");
    md.push_str("Slugs referenced by other pages but not yet created on disk.\n\n");
    if report.missing_stubs.is_empty() {
        md.push_str("_None._\n\n");
    } else {
        for slug in &report.missing_stubs {
            md.push_str(&format!("- `{slug}`\n"));
        }
        md.push('\n');
    }

    // ── Active Contradictions ────────────────────────────────────────────────
    md.push_str("## Active Contradictions\n\n");
    md.push_str("Contradiction pages awaiting enrichment (`active` or `under-analysis`).\n\n");
    if report.active_contradictions.is_empty() {
        md.push_str("_None._\n\n");
    } else {
        md.push_str("| Slug | Title | Dimension | Source A | Source B |\n");
        md.push_str("|------|-------|-----------|----------|----------|\n");
        for c in &report.active_contradictions {
            let dim = format!("{:?}", c.dimension).to_lowercase();
            md.push_str(&format!(
                "| `{}` | {} | {} | `{}` | `{}` |\n",
                c.slug, c.title, dim, c.source_a, c.source_b
            ));
        }
        md.push('\n');
    }

    // ── Orphan Asset Refs ────────────────────────────────────────────────────
    md.push_str("## Orphan Asset References\n\n");
    md.push_str("Bundle pages referencing `./asset` files that do not exist.\n\n");
    if report.orphan_asset_refs.is_empty() {
        md.push_str("_None._\n\n");
    } else {
        for r in &report.orphan_asset_refs {
            md.push_str(&format!("- `{r}`\n"));
        }
        md.push('\n');
    }

    // Summary footer.
    md.push_str(&format!(
        "---\n\n_{} orphan(s), {} missing stub(s), {} active contradiction(s), {} orphan asset ref(s)._\n",
        report.orphan_pages.len(),
        report.missing_stubs.len(),
        report.active_contradictions.len(),
        report.orphan_asset_refs.len(),
    ));

    std::fs::write(wiki_root.join("LINT.md"), md)
        .context("failed to write LINT.md")?;

    Ok(())
}

// ── lint ──────────────────────────────────────────────────────────────────────

/// Run a structural lint pass on the wiki at `wiki_root`.
pub fn lint(wiki_root: &Path) -> Result<LintReport> {
    let graph = build_graph(wiki_root).context("failed to build concept graph")?;

    let orphan_pages = orphans(&graph);
    let missing_stubs = missing_stubs(&graph, wiki_root);

    let all = list_contradictions(wiki_root, None)
        .context("failed to list contradiction pages")?;
    let active_contradictions: Vec<ContradictionSummary> = all
        .into_iter()
        .filter(|c| c.status == Status::Active || c.status == Status::UnderAnalysis)
        .collect();

    let orphan_asset_refs = collect_orphan_asset_refs(wiki_root);

    let report = LintReport {
        orphan_pages,
        missing_stubs,
        active_contradictions,
        orphan_asset_refs,
    };

    write_lint_report(wiki_root, &report)?;

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let message = format!(
        "lint: {} \u{2014} {} orphans, {} stubs, {} active contradictions",
        date,
        report.orphan_pages.len(),
        report.missing_stubs.len(),
        report.active_contradictions.len()
    );
    crate::git::commit(wiki_root, &message).context("failed to commit LINT.md")?;

    Ok(report)
}
