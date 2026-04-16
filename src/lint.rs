use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::config::ResolvedConfig;
use crate::frontmatter::parse_frontmatter;
use crate::git;
use crate::graph::{build_graph, in_degree, GraphFilter};
use crate::links::extract_links;
use crate::markdown::{create_page, create_section, resolve_slug, slug_for};
use crate::search::PageRef;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingConnection {
    pub slug_a: String,
    pub slug_b: String,
    pub overlapping_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintReport {
    pub orphans: Vec<PageRef>,
    pub missing_stubs: Vec<String>,
    pub empty_sections: Vec<String>,
    pub missing_connections: Vec<MissingConnection>,
    pub untyped_sources: Vec<String>,
    pub date: String,
}

// ── lint ───────────────────────────────────────────────────────────────────────

pub fn lint(wiki_root: &Path, _config: &ResolvedConfig, wiki_name: &str) -> Result<LintReport> {
    let date = Local::now().format("%Y-%m-%d").to_string();

    // Collect all pages: slug -> (frontmatter type, title, content)
    let mut pages: HashMap<String, (String, String, String)> = HashMap::new();
    for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let slug = slug_for(path, wiki_root);
        if let Ok((fm, _body)) = parse_frontmatter(&content) {
            pages.insert(slug, (fm.r#type.clone(), fm.title.clone(), content));
        }
    }

    // Build graph for orphan detection
    let filter = GraphFilter {
        root: None,
        depth: None,
        types: Vec::new(),
    };
    let graph = build_graph(wiki_root, &filter);

    // Orphans: in-degree 0, excluding sections
    let mut orphans = Vec::new();
    for (slug, (page_type, title, _content)) in &pages {
        if page_type == "section" {
            continue;
        }
        if in_degree(&graph, slug) == 0 {
            orphans.push(PageRef {
                slug: slug.clone(),
                uri: format!("wiki://{wiki_name}/{slug}"),
                title: title.clone(),
                score: 0.0,
                excerpt: None,
            });
        }
    }
    orphans.sort_by(|a, b| a.slug.cmp(&b.slug));

    // Missing stubs: referenced slugs that don't exist
    let mut missing_stubs_set: HashSet<String> = HashSet::new();
    for (_ptype, _title, content) in pages.values() {
        for link in extract_links(content) {
            if resolve_slug(&link, wiki_root).is_err() && !missing_stubs_set.contains(&link) {
                missing_stubs_set.insert(link);
            }
        }
    }
    let mut missing_stubs: Vec<String> = missing_stubs_set.into_iter().collect();
    missing_stubs.sort();

    // Empty sections: directories without index.md
    let mut empty_sections = Vec::new();
    for entry in WalkDir::new(wiki_root)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() && !path.join("index.md").exists() {
            // Check it has at least one .md file (it's a real section dir)
            let has_md = std::fs::read_dir(path)
                .map(|rd| {
                    rd.filter_map(|e| e.ok()).any(|e| {
                        e.path().extension().and_then(|x| x.to_str()) == Some("md")
                            || e.path().is_dir()
                    })
                })
                .unwrap_or(false);
            if has_md {
                let slug = path
                    .strip_prefix(wiki_root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                empty_sections.push(slug);
            }
        }
    }
    empty_sections.sort();

    // Missing connections: term overlap heuristic
    let missing_connections = detect_missing_connections(&pages);

    // Untyped sources: missing type or source-summary
    let mut untyped_sources = Vec::new();
    for (slug, (page_type, _title, _content)) in &pages {
        if page_type == "source-summary" {
            untyped_sources.push(slug.clone());
        } else if page_type.is_empty() || page_type == "page" {
            // Check if it looks like a source page (in sources/ dir)
            if slug.starts_with("sources/") {
                untyped_sources.push(slug.clone());
            }
        }
    }
    untyped_sources.sort();

    Ok(LintReport {
        orphans,
        missing_stubs,
        empty_sections,
        missing_connections,
        untyped_sources,
        date,
    })
}

fn detect_missing_connections(
    pages: &HashMap<String, (String, String, String)>,
) -> Vec<MissingConnection> {
    // Extract significant terms per page
    let mut page_terms: HashMap<String, HashSet<String>> = HashMap::new();
    let mut page_links: HashMap<String, HashSet<String>> = HashMap::new();

    for (slug, (_ptype, _title, content)) in pages {
        let links: HashSet<String> = extract_links(content).into_iter().collect();
        page_links.insert(slug.clone(), links);

        // Extract terms from title and body (words >= 4 chars, lowercased)
        let terms: HashSet<String> = if let Ok((fm, body)) = parse_frontmatter(content) {
            fm.title
                .split_whitespace()
                .chain(body.split_whitespace())
                .map(|w| {
                    w.to_lowercase()
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_string()
                })
                .filter(|w| w.len() >= 4)
                .collect()
        } else {
            HashSet::new()
        };
        page_terms.insert(slug.clone(), terms);
    }

    let slugs: Vec<&String> = pages.keys().collect();
    let mut connections = Vec::new();

    for i in 0..slugs.len() {
        for j in (i + 1)..slugs.len() {
            let a = slugs[i];
            let b = slugs[j];

            // Skip if already linked in either direction
            let a_links = page_links.get(a).cloned().unwrap_or_default();
            let b_links = page_links.get(b).cloned().unwrap_or_default();
            if a_links.contains(b) || b_links.contains(a) {
                continue;
            }

            let a_terms = page_terms.get(a).cloned().unwrap_or_default();
            let b_terms = page_terms.get(b).cloned().unwrap_or_default();
            let overlap: Vec<String> = a_terms.intersection(&b_terms).cloned().collect();

            if overlap.len() >= 3 {
                let mut sorted_overlap = overlap;
                sorted_overlap.sort();
                connections.push(MissingConnection {
                    slug_a: a.clone(),
                    slug_b: b.clone(),
                    overlapping_terms: sorted_overlap,
                });
            }
        }
    }

    connections.sort_by(|a, b| a.slug_a.cmp(&b.slug_a).then(a.slug_b.cmp(&b.slug_b)));
    connections
}

// ── write_lint_md ─────────────────────────────────────────────────────────────

pub fn write_lint_md(report: &LintReport, repo_root: &Path) -> Result<()> {
    let mut out = String::new();

    out.push_str(&format!("# Lint Report — {}\n\n", report.date));

    // Orphans
    out.push_str(&format!("## Orphans ({})\n\n", report.orphans.len()));
    if report.orphans.is_empty() {
        out.push_str("_No orphans found._\n");
    } else {
        out.push_str("| slug | title | uri | path |\n");
        out.push_str("|------|-------|-----|------|\n");
        for o in &report.orphans {
            out.push_str(&format!("| {} | {} | {} | |\n", o.slug, o.title, o.uri));
        }
    }

    out.push('\n');

    // Missing Stubs
    out.push_str(&format!(
        "## Missing Stubs ({})\n\n",
        report.missing_stubs.len()
    ));
    if report.missing_stubs.is_empty() {
        out.push_str("_No missing stubs found._\n");
    } else {
        out.push_str("| slug |\n");
        out.push_str("|------|\n");
        for s in &report.missing_stubs {
            out.push_str(&format!("| {s} |\n"));
        }
    }

    out.push('\n');

    // Empty Sections
    out.push_str(&format!(
        "## Empty Sections ({})\n\n",
        report.empty_sections.len()
    ));
    if report.empty_sections.is_empty() {
        out.push_str("_No empty sections found._\n");
    } else {
        out.push_str("| slug |\n");
        out.push_str("|------|\n");
        for s in &report.empty_sections {
            out.push_str(&format!("| {s} |\n"));
        }
    }

    out.push('\n');

    // Missing Connections
    out.push_str(&format!(
        "## Missing Connections ({})\n\n",
        report.missing_connections.len()
    ));
    if report.missing_connections.is_empty() {
        out.push_str("_No missing connections found._\n");
    } else {
        out.push_str("| page_a | page_b | shared terms |\n");
        out.push_str("|--------|--------|--------------|\n");
        for mc in &report.missing_connections {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                mc.slug_a,
                mc.slug_b,
                mc.overlapping_terms.join(", ")
            ));
        }
    }

    out.push('\n');

    // Untyped Sources
    out.push_str(&format!(
        "## Untyped Sources ({})\n\n",
        report.untyped_sources.len()
    ));
    if report.untyped_sources.is_empty() {
        out.push_str("_No untyped sources found._\n");
    } else {
        out.push_str("| slug | current type |\n");
        out.push_str("|------|-------------|\n");
        for s in &report.untyped_sources {
            out.push_str(&format!("| {s} | |\n"));
        }
    }

    std::fs::write(repo_root.join("LINT.md"), out)?;
    Ok(())
}

// ── lint_fix ──────────────────────────────────────────────────────────────────

pub fn lint_fix(
    wiki_root: &Path,
    config: &ResolvedConfig,
    only: Option<&str>,
    wiki_name: &str,
) -> Result<LintReport> {
    let report = lint(wiki_root, config, wiki_name)?;

    let fix_stubs = only.is_none() || only == Some("missing-stubs");
    let fix_sections = only.is_none() || only == Some("empty-sections");

    let mut stubs_created = 0;
    let mut sections_created = 0;

    if fix_stubs && config.lint.fix_missing_stubs {
        for stub in &report.missing_stubs {
            if create_page(stub, false, wiki_root).is_ok() {
                stubs_created += 1;
            }
        }
    }

    if fix_sections && config.lint.fix_empty_sections {
        for section in &report.empty_sections {
            if create_section(section, wiki_root).is_ok() {
                sections_created += 1;
            }
        }
    }

    if stubs_created > 0 || sections_created > 0 {
        let repo_root = wiki_root
            .parent()
            .ok_or_else(|| anyhow::anyhow!("wiki_root has no parent"))?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        git::commit(
            repo_root,
            &format!("lint(fix): {date} — +{stubs_created} stubs, +{sections_created} sections"),
        )?;
    }

    Ok(report)
}
