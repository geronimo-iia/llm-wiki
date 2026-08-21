use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use petgraph::graph::{NodeIndex, UnGraph};
use serde::Serialize;
use tantivy::query::AllQuery;
use tantivy::schema::Value;

use crate::engine::EngineState;
use crate::graph::{GraphFilter, WikiGraph, get_or_build_graph};
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
    /// Total number of findings after prefix/severity filters, before pagination.
    pub total: usize,
    /// Number of error-severity findings (after filters, before pagination).
    pub errors: usize,
    /// Number of warning-severity findings (after filters, before pagination).
    pub warnings: usize,
    /// Individual lint findings (empty when `summary: true`).
    pub findings: Vec<LintFinding>,
    /// Whether more pages exist beyond the current window.
    pub has_more: bool,
    /// Offset for the next page; absent when `has_more` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<usize>,
    /// Finding count per rule; present only when `summary: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_rule: Option<HashMap<&'static str, usize>>,
}

/// Options for a `run_lint` call. All fields default to their zero/None values.
#[derive(Default)]
pub struct LintOptions<'a> {
    /// Comma-separated rule names; `None` runs all rules.
    pub rules: Option<&'a str>,
    /// Restrict to `"error"` or `"warning"`; `None` returns all severities.
    pub severity: Option<&'a str>,
    /// Return counts only — no `findings` array.
    pub summary: bool,
    /// Restrict findings to slugs starting with this prefix.
    pub path_prefix: Option<&'a str>,
    /// Maximum findings per response; `None` returns all.
    pub page_size: Option<usize>,
    /// Zero-based offset into the sorted findings list.
    pub cursor: Option<usize>,
}

/// Per-document data extracted in a single shared tantivy pass.
struct DocRecord {
    slug: String,
    page_type: String,
    status: String,
    last_updated: String,
    confidence: Option<f64>,
    /// True when the confidence field is absent from the index schema entirely.
    /// Distinguishes schema-absent (fall back to date-only) from value-absent
    /// (not low confidence — page hasn't declared a confidence score).
    confidence_field_absent: bool,
    body_links: Vec<String>,
    sources: Vec<String>,
    concepts: Vec<String>,
    document_refs: Vec<String>,
    superseded_by: Vec<String>,
    /// Required-field presence map. Keyed by field name from the union of all
    /// types' required fields. `true` = present or not in index schema (can't check).
    fields_present: HashMap<String, bool>,
}

/// Run lint rules against a wiki. `opts.rules` is a comma-separated list; `None` runs all rules.
/// `opts.severity` restricts output to `"error"` or `"warning"`.
pub fn run_lint(
    engine: &EngineState,
    wiki_name: &str,
    opts: &LintOptions<'_>,
) -> Result<LintReport> {
    let active_rules: HashSet<&str> = match opts.rules {
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

    // ── Shared fetch pass ──────────────────────────────────────────────────────
    // Single AllQuery + N doc reads replaces the previous 5 AllQuery + 8×N reads.

    let all_addrs = searcher.search(&AllQuery, &tantivy::collector::DocSetCollector)?;

    let all_required: HashSet<String> = space
        .type_registry
        .list_types()
        .into_iter()
        .flat_map(|(t, _)| space.type_registry.required_fields(t).iter().cloned())
        .collect();

    let f_slug = is.field("slug");
    let f_type = is.field("type");
    let f_status = is.try_field("status");
    let f_last_updated = is.try_field("last_updated");
    let has_last_updated_field = f_last_updated.is_some();
    let f_confidence = is.try_field("confidence");
    let f_body_links = is.field("body_links");

    let mut records: Vec<DocRecord> = Vec::with_capacity(all_addrs.len());

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
        let status = f_status
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let last_updated = f_last_updated
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let confidence_field_absent = f_confidence.is_none();
        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64());

        let body_links = doc
            .get_all(f_body_links)
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        let mut sources = Vec::new();
        let mut concepts = Vec::new();
        let mut document_refs = Vec::new();
        let mut superseded_by = Vec::new();
        for (field_name, vec) in [
            ("sources", &mut sources),
            ("concepts", &mut concepts),
            ("document_refs", &mut document_refs),
            ("superseded_by", &mut superseded_by),
        ] {
            if let Some(f) = is.try_field(field_name) {
                *vec = doc
                    .get_all(f)
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }

        // unwrap_or(true): field absent from index schema → can't check → treat as present (skip).
        let fields_present: HashMap<String, bool> = all_required
            .iter()
            .map(|name| {
                let present = is
                    .try_field(name)
                    .map(|f| doc.get_first(f).is_some())
                    .unwrap_or(true);
                (name.clone(), present)
            })
            .collect();

        records.push(DocRecord {
            slug,
            page_type,
            status,
            last_updated,
            confidence,
            confidence_field_absent,
            body_links,
            sources,
            concepts,
            document_refs,
            superseded_by,
            fields_present,
        });
    }

    // ── Rule dispatch ──────────────────────────────────────────────────────────

    let mut findings: Vec<LintFinding> = Vec::new();

    if active_rules.contains("orphan") {
        findings.extend(rule_orphan(&records, wiki_root));
    }
    if active_rules.contains("broken-link") || active_rules.contains("broken-cross-wiki-link") {
        let mounted: HashSet<String> = engine.spaces.keys().cloned().collect();
        findings.extend(rule_broken_link(
            &records,
            wiki_root,
            active_rules.contains("broken-cross-wiki-link"),
            &mounted,
        ));
    }
    if active_rules.contains("missing-fields") {
        findings.extend(rule_missing_fields(
            &records,
            wiki_root,
            &space.type_registry,
        ));
    }
    // Guard mirrors the original early-return when last_updated is absent from the schema.
    if active_rules.contains("stale") && has_last_updated_field {
        findings.extend(rule_stale(
            &records,
            wiki_root,
            lint_cfg.stale_days,
            lint_cfg.stale_confidence_threshold,
        ));
    }
    if active_rules.contains("unknown-type") {
        findings.extend(rule_unknown_type(&records, wiki_root, &space.type_registry));
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

    // Apply path_prefix filter before severity
    if let Some(prefix) = opts.path_prefix {
        findings.retain(|f| f.slug.starts_with(prefix));
    }

    // Apply severity filter
    if let Some(sev) = opts.severity {
        let sev = sev.trim().to_lowercase();
        findings.retain(|f| f.severity.to_string() == sev);
    }

    findings.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.rule.cmp(b.rule)));

    // Counts are over the full filtered list, before pagination.
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let total = findings.len();

    // Build by_rule before any pagination (summary mode only).
    let by_rule: Option<HashMap<&'static str, usize>> = if opts.summary {
        let mut map: HashMap<&'static str, usize> = HashMap::new();
        for f in &findings {
            *map.entry(f.rule).or_insert(0) += 1;
        }
        Some(map)
    } else {
        None
    };

    // Apply pagination. When summary: true, page_findings is built then discarded below —
    // wasteful but harmless. has_more and next_cursor remain correct in all cases.
    let (page_findings, has_more, next_cursor) = if let Some(size) = opts.page_size {
        let start = opts.cursor.unwrap_or(0);
        let end = (start + size).min(findings.len());
        let more = end < findings.len();
        let next = if more { Some(end) } else { None };
        (findings[start..end].to_vec(), more, next)
    } else {
        (findings, false, None)
    };

    // summary mode: drop the findings array (by_rule carries the information instead).
    let final_findings = if opts.summary { vec![] } else { page_findings };

    Ok(LintReport {
        wiki: wiki_name.to_string(),
        total,
        errors,
        warnings,
        findings: final_findings,
        has_more,
        next_cursor,
        by_rule,
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

fn rule_orphan(records: &[DocRecord], wiki_root: &Path) -> Vec<LintFinding> {
    let all_linked: HashSet<&str> = records
        .iter()
        .flat_map(|r| {
            r.body_links
                .iter()
                .map(String::as_str)
                .chain(r.sources.iter().map(String::as_str))
                .chain(r.concepts.iter().map(String::as_str))
                .chain(r.document_refs.iter().map(String::as_str))
                .chain(r.superseded_by.iter().map(String::as_str))
        })
        .collect();

    records
        .iter()
        .filter(|r| r.page_type != "section" && r.slug != "index" && !r.slug.ends_with("/index"))
        .filter(|r| !all_linked.contains(r.slug.as_str()))
        .map(|r| LintFinding {
            path: slug_path(&r.slug, wiki_root),
            slug: r.slug.clone(),
            rule: "orphan",
            severity: Severity::Warning,
            message: "no incoming links".to_string(),
        })
        .collect()
}

// ── Rule: broken-link ─────────────────────────────────────────────────────────

fn rule_broken_link(
    records: &[DocRecord],
    wiki_root: &Path,
    check_cross_wiki: bool,
    mounted: &HashSet<String>,
) -> Vec<LintFinding> {
    let known_slugs: HashSet<&str> = records.iter().map(|r| r.slug.as_str()).collect();
    let mut findings = Vec::new();
    for r in records {
        let all_links = r
            .body_links
            .iter()
            .map(|s| ("body_links", s.as_str()))
            .chain(r.sources.iter().map(|s| ("sources", s.as_str())))
            .chain(r.concepts.iter().map(|s| ("concepts", s.as_str())))
            .chain(
                r.document_refs
                    .iter()
                    .map(|s| ("document_refs", s.as_str())),
            )
            .chain(
                r.superseded_by
                    .iter()
                    .map(|s| ("superseded_by", s.as_str())),
            );
        for (field_name, target) in all_links {
            if target.starts_with("wiki://") {
                if check_cross_wiki
                    && let Some(wiki_name) = target
                        .strip_prefix("wiki://")
                        .and_then(|rest| rest.split('/').next())
                    && !mounted.contains(wiki_name)
                {
                    findings.push(LintFinding {
                        path: slug_path(&r.slug, wiki_root),
                        slug: r.slug.clone(),
                        rule: "broken-cross-wiki-link",
                        severity: Severity::Warning,
                        message: format!("cross-wiki link to unmounted wiki: {target}"),
                    });
                }
                continue;
            }
            if !known_slugs.contains(target) {
                findings.push(LintFinding {
                    path: slug_path(&r.slug, wiki_root),
                    slug: r.slug.clone(),
                    rule: "broken-link",
                    severity: Severity::Error,
                    message: format!("broken link in {field_name}: {target}"),
                });
            }
        }
    }
    findings
}

// ── Rule: missing-fields ──────────────────────────────────────────────────────

fn rule_missing_fields(
    records: &[DocRecord],
    wiki_root: &Path,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Vec<LintFinding> {
    records
        .iter()
        .filter(|r| !r.page_type.is_empty() && registry.is_known(&r.page_type))
        .flat_map(|r| {
            registry
                .required_fields(&r.page_type)
                .iter()
                .filter(|field| !r.fields_present.get(*field).copied().unwrap_or(true))
                .map(|field| LintFinding {
                    path: slug_path(&r.slug, wiki_root),
                    slug: r.slug.clone(),
                    rule: "missing-fields",
                    severity: Severity::Error,
                    message: format!("required field missing: {field}"),
                })
        })
        .collect()
}

// ── Rule: stale ───────────────────────────────────────────────────────────────

fn rule_stale(
    records: &[DocRecord],
    wiki_root: &Path,
    stale_days: u32,
    threshold: f32,
) -> Vec<LintFinding> {
    let today = chrono::Utc::now().date_naive();
    let threshold_date = today - chrono::Duration::days(stale_days as i64);
    records
        .iter()
        .filter(|r| r.status == "active" || r.status.is_empty())
        .filter_map(|r| {
            let is_old = chrono::NaiveDate::parse_from_str(&r.last_updated, "%Y-%m-%d")
                .map(|d| d < threshold_date)
                .unwrap_or(true);
            if !is_old {
                return None;
            }
            // Mirrors original: absent schema field → date-only (low confidence = true).
            // Schema field present but no value → NOT low confidence (page hasn't declared one).
            let is_low_confidence = if r.confidence_field_absent {
                true
            } else {
                r.confidence
                    .map(|v| (v as f32) < threshold)
                    .unwrap_or(false)
            };
            if !is_low_confidence {
                return None;
            }
            let age_note = if r.last_updated.is_empty() {
                "no last_updated date".to_string()
            } else {
                format!("last updated {}", r.last_updated)
            };
            Some(LintFinding {
                path: slug_path(&r.slug, wiki_root),
                slug: r.slug.clone(),
                rule: "stale",
                severity: Severity::Warning,
                message: format!("stale page: {age_note}"),
            })
        })
        .collect()
}

// ── Rule: unknown-type ────────────────────────────────────────────────────────

fn rule_unknown_type(
    records: &[DocRecord],
    wiki_root: &Path,
    registry: &crate::type_registry::SpaceTypeRegistry,
) -> Vec<LintFinding> {
    records
        .iter()
        .filter(|r| !r.page_type.is_empty() && !registry.is_known(&r.page_type))
        .map(|r| LintFinding {
            path: slug_path(&r.slug, wiki_root),
            slug: r.slug.clone(),
            rule: "unknown-type",
            severity: Severity::Error,
            message: format!("unknown type: {}", r.page_type),
        })
        .collect()
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

    fn make_graph(slugs: &[&str], edges: &[(&str, &str)]) -> WikiGraph {
        let mut g = WikiGraph::new();
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
        let mut g = WikiGraph::new();
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

    fn make_record(slug: &str, page_type: &str) -> DocRecord {
        DocRecord {
            slug: slug.to_string(),
            page_type: page_type.to_string(),
            status: String::new(),
            last_updated: String::new(),
            confidence: None,
            confidence_field_absent: false,
            body_links: vec![],
            sources: vec![],
            concepts: vec![],
            document_refs: vec![],
            superseded_by: vec![],
            fields_present: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn rule_orphan_flags_unlinked_page() {
        let root = std::path::Path::new("/wiki");
        // a↔b form a mutually-linked pair; "c" has no incoming links → only orphan
        let mut a = make_record("a", "note");
        a.body_links = vec!["b".to_string()];
        let mut b = make_record("b", "note");
        b.body_links = vec!["a".to_string()];
        let c = make_record("c", "note");
        let findings = rule_orphan(&[a, b, c], root);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].slug, "c");
        assert_eq!(findings[0].rule, "orphan");
        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn rule_orphan_skips_section_and_index_slugs() {
        let root = std::path::Path::new("/wiki");
        // section pages and slugs ending in /index are exempt
        let sec = make_record("intro", "section");
        let idx = make_record("projects/index", "note");
        let bare_idx = make_record("index", "note");
        let findings = rule_orphan(&[sec, idx, bare_idx], root);
        assert!(findings.is_empty(), "got findings: {findings:?}");
    }

    #[test]
    fn rule_unknown_type_flags_unrecognised_type() {
        use crate::type_registry::SpaceTypeRegistry;
        let registry = SpaceTypeRegistry::default();
        let root = std::path::Path::new("/wiki");
        // "ghost" is not in any embedded schema
        let r = make_record("some-page", "ghost");
        let findings = rule_unknown_type(&[r], root, &registry);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "unknown-type");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("ghost"));
    }

    #[test]
    fn rule_broken_link_flags_missing_slug() {
        let root = std::path::Path::new("/wiki");
        let mut a = make_record("a", "note");
        a.body_links = vec!["nonexistent".to_string()];
        let b = make_record("b", "note");
        let findings = rule_broken_link(&[a, b], root, false, &std::collections::HashSet::new());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].slug, "a");
        assert_eq!(findings[0].rule, "broken-link");
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("nonexistent"));
    }

    #[test]
    fn rule_stale_flags_old_page_with_no_confidence_field() {
        let root = std::path::Path::new("/wiki");
        let mut r = make_record("old-page", "note");
        r.last_updated = "2000-01-01".to_string();
        r.confidence_field_absent = true;
        let findings = rule_stale(&[r], root, 1, 0.7);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "stale");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("2000-01-01"));
    }

    #[test]
    fn rule_stale_skips_non_active_status() {
        let root = std::path::Path::new("/wiki");
        let mut r = make_record("archived-page", "note");
        r.last_updated = "2000-01-01".to_string();
        r.status = "archived".to_string();
        r.confidence_field_absent = true;
        let findings = rule_stale(&[r], root, 1, 0.7);
        assert!(findings.is_empty(), "archived pages must not be flagged");
    }

    #[test]
    fn rule_stale_skips_high_confidence_page() {
        let root = std::path::Path::new("/wiki");
        let mut r = make_record("solid-page", "note");
        r.last_updated = "2000-01-01".to_string();
        r.confidence = Some(0.9);
        r.confidence_field_absent = false;
        let findings = rule_stale(&[r], root, 1, 0.7);
        assert!(
            findings.is_empty(),
            "high-confidence page must not be flagged"
        );
    }

    #[test]
    fn rule_missing_fields_flags_absent_required_field() {
        use crate::type_registry::SpaceTypeRegistry;
        let registry = SpaceTypeRegistry::default();
        let root = std::path::Path::new("/wiki");
        let type_name = registry
            .list_types()
            .into_iter()
            .find(|(t, _)| !registry.required_fields(t).is_empty())
            .expect("at least one type with required fields in embedded schemas")
            .0
            .to_string();
        let required = registry.required_fields(&type_name).to_vec();

        let mut r = make_record("test-page", &type_name);
        r.fields_present = required.iter().map(|f| (f.clone(), false)).collect();

        let findings = rule_missing_fields(&[r], root, &registry);
        assert!(
            !findings.is_empty(),
            "expected findings for absent required fields"
        );
        for f in &findings {
            assert_eq!(f.rule, "missing-fields");
            assert_eq!(f.severity, Severity::Error);
        }
    }

    #[test]
    fn rule_missing_fields_no_findings_when_all_present() {
        use crate::type_registry::SpaceTypeRegistry;
        let registry = SpaceTypeRegistry::default();
        let root = std::path::Path::new("/wiki");
        let type_name = registry
            .list_types()
            .into_iter()
            .find(|(t, _)| !registry.required_fields(t).is_empty())
            .expect("at least one type with required fields")
            .0
            .to_string();
        let required = registry.required_fields(&type_name).to_vec();

        let mut r = make_record("test-page", &type_name);
        r.fields_present = required.iter().map(|f| (f.clone(), true)).collect();

        let findings = rule_missing_fields(&[r], root, &registry);
        assert!(findings.is_empty(), "expected no findings: {findings:?}");
    }

    #[test]
    fn rule_broken_link_cross_wiki_flagged_when_unmounted() {
        let root = std::path::Path::new("/wiki");
        let mut a = make_record("a", "note");
        a.body_links = vec!["wiki://other-wiki/some-page".to_string()];
        let mounted: std::collections::HashSet<String> = ["my-wiki".to_string()].into();
        let findings = rule_broken_link(&[a], root, true, &mounted);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "broken-cross-wiki-link");
        assert_eq!(findings[0].severity, Severity::Warning);
    }
}
