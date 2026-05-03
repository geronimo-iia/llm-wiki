use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use petgraph::graph::{NodeIndex, UnGraph};
use serde::Serialize;
use tantivy::schema::Value;
use tantivy::{
    Term,
    query::{AllQuery, TermQuery},
    schema::IndexRecordOption,
};

use crate::engine::EngineState;
use crate::graph::{GraphFilter, WikiGraph, get_or_build_graph};
use crate::index_schema::IndexSchema;
use crate::slug::Slug;

/// Severity level of a lint finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A definite problem that should be fixed.
    Error,
    /// A potential issue that may warrant attention.
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// A single lint finding for a wiki page.
#[derive(Debug, Clone, Serialize)]
pub struct LintFinding {
    /// Slug of the page with the finding.
    pub slug: String,
    /// Name of the lint rule that produced this finding.
    pub rule: &'static str,
    /// Severity of the finding.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
    /// Filesystem path of the page file.
    pub path: String,
}

/// Aggregate results of a lint run against a wiki.
#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    /// Name of the wiki that was linted.
    pub wiki: String,
    /// Total number of findings (errors + warnings).
    pub total: usize,
    /// Number of error-severity findings.
    pub errors: usize,
    /// Number of warning-severity findings.
    pub warnings: usize,
    /// Individual lint findings, sorted by slug then rule.
    pub findings: Vec<LintFinding>,
}

/// Run lint rules against a wiki. `rules` is a comma-separated list; `None` runs all rules.
/// `severity_filter` restricts output to `"error"` or `"warning"`.
pub fn run_lint(
    engine: &EngineState,
    wiki_name: &str,
    rules: Option<&str>,
    severity_filter: Option<&str>,
) -> Result<LintReport> {
    let active_rules: HashSet<&str> = match rules {
        None | Some("") => [
            "orphan",
            "broken-link",
            "broken-cross-wiki-link",
            "missing-fields",
            "stale",
            "unknown-type",
            "articulation-point",
            "bridge",
            "periphery",
        ]
        .iter()
        .copied()
        .collect(),
        Some(s) => s.split(',').map(str::trim).collect(),
    };

    let space = engine.space(wiki_name)?;
    let searcher = space.index_manager.searcher()?;
    let is = &space.index_schema;
    let resolved = space.resolved_config(&engine.config);
    let lint_cfg = &resolved.lint;
    let wiki_root = &space.wiki_root;

    let mut findings: Vec<LintFinding> = Vec::new();

    if active_rules.contains("orphan") {
        findings.extend(rule_orphan(&searcher, is, wiki_root)?);
    }
    if active_rules.contains("broken-link") || active_rules.contains("broken-cross-wiki-link") {
        let mounted: HashSet<String> = engine.spaces.keys().cloned().collect();
        findings.extend(rule_broken_link(
            &searcher,
            is,
            wiki_root,
            active_rules.contains("broken-cross-wiki-link"),
            &mounted,
        )?);
    }
    if active_rules.contains("missing-fields") {
        findings.extend(rule_missing_fields(
            &searcher,
            is,
            wiki_root,
            &space.type_registry,
        )?);
    }
    if active_rules.contains("stale") {
        findings.extend(rule_stale(
            &searcher,
            is,
            wiki_root,
            lint_cfg.stale_days,
            lint_cfg.stale_confidence_threshold,
        )?);
    }
    if active_rules.contains("unknown-type") {
        findings.extend(rule_unknown_type(
            &searcher,
            is,
            wiki_root,
            &space.type_registry,
        )?);
    }

    let needs_graph = active_rules.contains("articulation-point")
        || active_rules.contains("bridge")
        || active_rules.contains("periphery");

    if needs_graph {
        let wiki_graph = get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &GraphFilter::default(),
        )?;
        if active_rules.contains("articulation-point") {
            findings.extend(rule_articulation_point(&wiki_graph, wiki_root));
        }
        if active_rules.contains("bridge") {
            findings.extend(rule_bridge(&wiki_graph, wiki_root));
        }
        if active_rules.contains("periphery") {
            findings.extend(rule_periphery(
                &wiki_graph,
                wiki_root,
                resolved.graph.max_nodes_for_diameter,
            ));
        }
    }

    // Apply severity filter
    if let Some(sev) = severity_filter {
        let sev = sev.trim().to_lowercase();
        findings.retain(|f| f.severity.to_string() == sev);
    }

    findings.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.rule.cmp(b.rule)));

    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let total = findings.len();

    Ok(LintReport {
        wiki: wiki_name.to_string(),
        total,
        errors,
        warnings,
        findings,
    })
}

/// Resolve a slug to its filesystem path string. Probes flat then bundle;
/// falls back to the would-be flat path if the file doesn't exist yet.
fn slug_path(slug: &str, wiki_root: &Path) -> String {
    Slug::try_from(slug)
        .ok()
        .and_then(|s| s.resolve(wiki_root).ok())
        .unwrap_or_else(|| wiki_root.join(format!("{slug}.md")))
        .to_string_lossy()
        .into_owned()
}

// ── Rule: orphan ──────────────────────────────────────────────────────────────

fn rule_orphan(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_root: &Path,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    // Collect all slugs referenced in body_links across all docs
    let mut all_linked: HashSet<String> = HashSet::new();
    let f_body_links = is.field("body_links");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        for val in doc.get_all(f_body_links) {
            if let Some(s) = val.as_str() {
                all_linked.insert(s.to_string());
            }
        }
        // Also count frontmatter edge fields as incoming-link evidence
        for field_name in &["sources", "concepts", "document_refs", "superseded_by"] {
            if let Some(f) = is.try_field(field_name) {
                for val in doc.get_all(f) {
                    if let Some(s) = val.as_str() {
                        all_linked.insert(s.to_string());
                    }
                }
            }
        }
    }

    let mut findings = Vec::new();
    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Sections are structural — not flagged as orphans
        if page_type == "section" {
            continue;
        }
        // Root/index pages are exempt
        if slug == "index" || slug.ends_with("/index") {
            continue;
        }

        if !all_linked.contains(&slug) {
            findings.push(LintFinding {
                path: slug_path(&slug, wiki_root),
                slug,
                rule: "orphan",
                severity: Severity::Warning,
                message: "no incoming links".to_string(),
            });
        }
    }

    Ok(findings)
}

// ── Rule: broken-link ─────────────────────────────────────────────────────────

fn slug_exists(searcher: &tantivy::Searcher, is: &IndexSchema, slug: &str) -> Result<bool> {
    let f_slug = is.field("slug");
    let term = Term::from_field_text(f_slug, slug);
    let query = TermQuery::new(term, IndexRecordOption::Basic);
    let results = searcher.search(&query, &tantivy::collector::DocSetCollector)?;
    Ok(!results.is_empty())
}

fn rule_broken_link(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_root: &Path,
    check_cross_wiki: bool,
    mounted_wiki_names: &HashSet<String>,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let link_fields = [
        "body_links",
        "sources",
        "concepts",
        "document_refs",
        "superseded_by",
    ];

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }

        for field_name in &link_fields {
            let f = match is.try_field(field_name) {
                Some(f) => f,
                None => continue,
            };
            for val in doc.get_all(f) {
                let target = match val.as_str() {
                    Some(s) => s,
                    None => continue,
                };
                if target.starts_with("wiki://") {
                    if check_cross_wiki
                        && let Some(wiki_name) = target
                            .strip_prefix("wiki://")
                            .and_then(|r| r.split('/').next())
                        && !mounted_wiki_names.contains(wiki_name)
                    {
                        findings.push(LintFinding {
                            path: slug_path(&slug, wiki_root),
                            slug: slug.clone(),
                            rule: "broken-cross-wiki-link",
                            severity: Severity::Warning,
                            message: format!("cross-wiki link to unmounted wiki: {target}"),
                        });
                    }
                    continue;
                }
                if !slug_exists(searcher, is, target)? {
                    findings.push(LintFinding {
                        path: slug_path(&slug, wiki_root),
                        slug: slug.clone(),
                        rule: "broken-link",
                        severity: Severity::Error,
                        message: format!("broken link in {field_name}: {target}"),
                    });
                }
            }
        }
    }

    Ok(findings)
}

// ── Rule: missing-fields ──────────────────────────────────────────────────────

fn rule_missing_fields(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_root: &Path,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if page_type.is_empty() || !registry.is_known(&page_type) {
            continue;
        }

        // Get required fields from JSON schema
        let required = registry.required_fields(&page_type);
        for field_name in &required {
            // Check via index field presence
            let present = if let Some(f) = is.try_field(field_name) {
                doc.get_first(f).is_some()
            } else {
                // Field not in index schema — can't check, skip
                true
            };
            if !present {
                findings.push(LintFinding {
                    path: slug_path(&slug, wiki_root),
                    slug: slug.clone(),
                    rule: "missing-fields",
                    severity: Severity::Error,
                    message: format!("required field missing: {field_name}"),
                });
            }
        }
    }

    Ok(findings)
}

// ── Rule: stale ───────────────────────────────────────────────────────────────

fn rule_stale(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_root: &Path,
    stale_days: u32,
    stale_confidence_threshold: f32,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_last_updated = match is.try_field("last_updated") {
        Some(f) => f,
        None => return Ok(vec![]),
    };
    let f_confidence = is.try_field("confidence");

    let today = chrono::Utc::now().date_naive();
    let threshold_date = today - chrono::Duration::days(stale_days as i64);

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }

        let date_str = doc
            .get_first(f_last_updated)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let is_old = if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            date < threshold_date
        } else {
            // No valid date — treat as old
            true
        };

        if !is_old {
            continue;
        }

        // Check confidence if the field is indexed
        let is_low_confidence = if let Some(f_conf) = f_confidence {
            match doc.get_first(f_conf).and_then(|v| v.as_f64()) {
                Some(v) => (v as f32) < stale_confidence_threshold,
                None => true, // No confidence value — treat as low
            }
        } else {
            // Field not indexed — fall back to date-only
            true
        };

        if is_old && is_low_confidence {
            let age_note = if date_str.is_empty() {
                "no last_updated date".to_string()
            } else {
                format!("last updated {date_str}")
            };
            findings.push(LintFinding {
                path: slug_path(&slug, wiki_root),
                slug,
                rule: "stale",
                severity: Severity::Warning,
                message: format!("stale page: {age_note}"),
            });
        }
    }

    Ok(findings)
}

// ── Graph helper ─────────────────────────────────────────────────────────────

fn build_undirected(
    graph: &WikiGraph,
) -> (
    UnGraph<NodeIndex, ()>,
    std::collections::HashMap<petgraph::graph::NodeIndex<u32>, NodeIndex>,
) {
    let mut ug: UnGraph<NodeIndex, ()> = UnGraph::new_undirected();
    let mut node_map: std::collections::HashMap<NodeIndex, petgraph::graph::NodeIndex<u32>> =
        std::collections::HashMap::new();
    let mut reverse_map: std::collections::HashMap<petgraph::graph::NodeIndex<u32>, NodeIndex> =
        std::collections::HashMap::new();
    for idx in graph.node_indices() {
        if !graph[idx].external {
            let ug_idx = ug.add_node(idx);
            node_map.insert(idx, ug_idx);
            reverse_map.insert(ug_idx, idx);
        }
    }
    for edge in graph.edge_indices() {
        let (a, b) = graph.edge_endpoints(edge).unwrap();
        if graph[a].external || graph[b].external {
            continue;
        }
        if let (Some(&ua), Some(&ub)) = (node_map.get(&a), node_map.get(&b))
            && ug.find_edge(ua, ub).is_none()
        {
            ug.add_edge(ua, ub, ());
        }
    }
    (ug, reverse_map)
}

// ── Rule: unknown-type ────────────────────────────────────────────────────────

fn rule_unknown_type(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_root: &Path,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Result<Vec<LintFinding>> {
    let f_slug = is.field("slug");
    let f_type = is.field("type");

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let mut findings = Vec::new();

    for addr in &all_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*addr)?;
        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let page_type = doc.get_first(f_type).and_then(|v| v.as_str()).unwrap_or("");
        if page_type.is_empty() {
            continue;
        }
        if !registry.is_known(page_type) {
            findings.push(LintFinding {
                path: slug_path(&slug, wiki_root),
                slug,
                rule: "unknown-type",
                severity: Severity::Error,
                message: format!("unknown type: {page_type}"),
            });
        }
    }

    Ok(findings)
}

// ── Rule: articulation-point ──────────────────────────────────────────────────

fn rule_articulation_point(wiki_graph: &Arc<WikiGraph>, wiki_root: &Path) -> Vec<LintFinding> {
    let (ug, reverse_map) = build_undirected(wiki_graph);
    let aps = petgraph_live::connect::articulation_points(&ug);
    aps.iter()
        .filter_map(|&ug_idx| reverse_map.get(&ug_idx))
        .map(|&orig_idx| {
            let slug = wiki_graph[orig_idx].slug.clone();
            LintFinding {
                path: slug_path(&slug, wiki_root),
                slug,
                rule: "articulation-point",
                severity: Severity::Warning,
                message:
                    "removing this page would disconnect the graph — add alternative link paths"
                        .to_string(),
            }
        })
        .collect()
}

// ── Rule: bridge ──────────────────────────────────────────────────────────────

fn rule_bridge(wiki_graph: &Arc<WikiGraph>, wiki_root: &Path) -> Vec<LintFinding> {
    let (ug, reverse_map) = build_undirected(wiki_graph);
    let bridges = petgraph_live::connect::find_bridges(&ug);
    bridges
        .iter()
        .filter_map(|&(ua, ub)| {
            let a = reverse_map.get(&ua)?;
            let b = reverse_map.get(&ub)?;
            Some((*a, *b))
        })
        .map(|(a, b)| {
            let slug_a = wiki_graph[a].slug.clone();
            let slug_b = wiki_graph[b].slug.clone();
            LintFinding {
                path: slug_path(&slug_a, wiki_root),
                slug: slug_a.clone(),
                rule: "bridge",
                severity: Severity::Warning,
                message: format!(
                    "link {slug_a} → {slug_b} is a bridge — its removal disconnects the graph"
                ),
            }
        })
        .collect()
}

// ── Rule: periphery ───────────────────────────────────────────────────────────

fn rule_periphery(
    wiki_graph: &Arc<WikiGraph>,
    wiki_root: &Path,
    max_nodes: usize,
) -> Vec<LintFinding> {
    let local_count = wiki_graph
        .node_indices()
        .filter(|&idx| !wiki_graph[idx].external)
        .count();
    if local_count > max_nodes {
        return vec![];
    }
    let periph = petgraph_live::metrics::periphery(&**wiki_graph);
    periph
        .iter()
        .filter(|&&idx| !wiki_graph[idx].external)
        .map(|&idx| {
            let slug = wiki_graph[idx].slug.clone();
            LintFinding {
                path: slug_path(&slug, wiki_root),
                slug,
                rule: "periphery",
                severity: Severity::Warning,
                message: "most structurally isolated page — furthest from all others in the graph"
                    .to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{LabeledEdge, PageNode};
    use petgraph::graph::DiGraph;

    fn make_graph(slugs: &[&str], edges: &[(&str, &str)]) -> WikiGraph {
        let mut g = DiGraph::new();
        let indices: std::collections::HashMap<&str, petgraph::graph::NodeIndex> = slugs
            .iter()
            .map(|&s| {
                (
                    s,
                    g.add_node(PageNode {
                        slug: s.to_string(),
                        title: s.to_string(),
                        r#type: "page".to_string(),
                        external: false,
                    }),
                )
            })
            .collect();
        for &(a, b) in edges {
            g.add_edge(
                indices[a],
                indices[b],
                LabeledEdge {
                    relation: "links-to".to_string(),
                },
            );
        }
        g
    }

    #[test]
    fn build_undirected_excludes_external() {
        let mut g = DiGraph::new();
        let local = g.add_node(PageNode {
            slug: "a".into(),
            title: "a".into(),
            r#type: "page".into(),
            external: false,
        });
        let ext = g.add_node(PageNode {
            slug: "b".into(),
            title: "b".into(),
            r#type: "page".into(),
            external: true,
        });
        g.add_edge(
            local,
            ext,
            LabeledEdge {
                relation: "links-to".into(),
            },
        );
        let (ug, _) = build_undirected(&g);
        assert_eq!(ug.node_count(), 1);
        assert_eq!(ug.edge_count(), 0);
    }

    #[test]
    fn articulation_point_detected() {
        // a -- b -- c  →  b is articulation point
        let g = make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
        let (ug, rev) = build_undirected(&g);
        let aps = petgraph_live::connect::articulation_points(&ug);
        let slugs: Vec<String> = aps
            .iter()
            .filter_map(|&ui| rev.get(&ui))
            .map(|&idx| g[idx].slug.clone())
            .collect();
        assert!(
            slugs.contains(&"b".to_string()),
            "b must be AP, got: {slugs:?}"
        );
    }

    #[test]
    fn no_articulation_points_in_cycle() {
        let g = make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c"), ("c", "a")]);
        let (ug, _) = build_undirected(&g);
        assert!(petgraph_live::connect::articulation_points(&ug).is_empty());
    }

    #[test]
    fn bridge_detected() {
        // a -- b -- c  →  both edges are bridges
        let g = make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
        let (ug, rev) = build_undirected(&g);
        let bridges = petgraph_live::connect::find_bridges(&ug);
        assert_eq!(bridges.len(), 2);
        let pairs: Vec<(String, String)> = bridges
            .iter()
            .filter_map(|&(ua, ub)| {
                Some((
                    g[*rev.get(&ua)?].slug.clone(),
                    g[*rev.get(&ub)?].slug.clone(),
                ))
            })
            .collect();
        let has_ab = pairs
            .iter()
            .any(|(a, b)| (a == "a" && b == "b") || (a == "b" && b == "a"));
        let has_bc = pairs
            .iter()
            .any(|(a, b)| (a == "b" && b == "c") || (a == "c" && b == "b"));
        assert!(has_ab && has_bc);
    }

    #[test]
    fn rule_articulation_point_produces_finding_for_connector() {
        // a -- b -- c: b is the only articulation point
        let g = Arc::new(make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]));
        let findings = rule_articulation_point(&g, Path::new("/wiki"));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].slug, "b");
        assert_eq!(findings[0].rule, "articulation-point");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("disconnect"));
    }

    #[test]
    fn rule_articulation_point_empty_for_cycle() {
        let g = Arc::new(make_graph(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c"), ("c", "a")],
        ));
        assert!(rule_articulation_point(&g, Path::new("/wiki")).is_empty());
    }

    #[test]
    fn rule_bridge_produces_findings_with_correct_fields() {
        // a -- b -- c: both edges are bridges
        let g = Arc::new(make_graph(&["a", "b", "c"], &[("a", "b"), ("b", "c")]));
        let findings = rule_bridge(&g, Path::new("/wiki"));
        assert_eq!(findings.len(), 2);
        for f in &findings {
            assert_eq!(f.rule, "bridge");
            assert_eq!(f.severity, Severity::Warning);
            assert!(
                f.message.contains("→"),
                "message must contain arrow, got: {}",
                f.message
            );
            assert!(f.message.contains("is a bridge"));
        }
        let slugs: Vec<&str> = findings.iter().map(|f| f.slug.as_str()).collect();
        assert!(slugs.contains(&"a") || slugs.contains(&"b"));
    }

    #[test]
    fn rule_bridge_empty_for_cycle() {
        let g = Arc::new(make_graph(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c"), ("c", "a")],
        ));
        assert!(rule_bridge(&g, Path::new("/wiki")).is_empty());
    }

    #[test]
    fn rule_periphery_produces_findings() {
        // a→b→c→a: directed cycle, all nodes have eccentricity 2 = diameter
        let g = Arc::new(make_graph(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c"), ("c", "a")],
        ));
        let findings = rule_periphery(&g, Path::new("/wiki"), 100);
        assert!(!findings.is_empty());
        for f in &findings {
            assert_eq!(f.rule, "periphery");
            assert_eq!(f.severity, Severity::Warning);
            assert!(f.message.contains("isolated"));
        }
    }

    #[test]
    fn rule_periphery_skips_above_threshold() {
        // 3 nodes, threshold 2 → local_count(3) > max_nodes(2) → empty
        let g = Arc::new(make_graph(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c"), ("c", "a")],
        ));
        assert!(rule_periphery(&g, Path::new("/wiki"), 2).is_empty());
    }
}
