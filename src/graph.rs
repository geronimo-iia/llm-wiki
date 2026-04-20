use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use chrono::Utc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value;
use tantivy::Searcher;

use crate::index_schema::IndexSchema;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNode {
    pub slug: String,
    pub title: String,
    pub r#type: String,
}

#[derive(Debug, Clone)]
pub struct LabeledEdge {
    pub relation: String,
}

pub type WikiGraph = DiGraph<PageNode, LabeledEdge>;

#[derive(Debug, Clone, Default)]
pub struct GraphFilter {
    pub root: Option<String>,
    pub depth: Option<usize>,
    pub types: Vec<String>,
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphReport {
    pub nodes: usize,
    pub edges: usize,
    pub output: String,
}

// ── build_graph ───────────────────────────────────────────────────────────────

/// Build the concept graph from the tantivy index. No file I/O.
///
/// Phase 1 uses hardcoded relations:
/// - `sources` field → "fed-by"
/// - `concepts` field → "depends-on"
/// - `body_links` field → "links-to"
pub fn build_graph(searcher: &Searcher, is: &IndexSchema, filter: &GraphFilter) -> Result<WikiGraph> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_body_links = is.field("body_links");

    let top_docs = searcher.search(&AllQuery, &TopDocs::with_limit(100_000))?;

    let mut graph = WikiGraph::new();
    let mut slug_to_idx: HashMap<String, NodeIndex> = HashMap::new();

    // Collect all slug-list fields we need for edges
    // For each doc, store (slug, type, sources, concepts, body_links)
    struct DocEdges {
        slug: String,
        sources: Vec<String>,
        concepts: Vec<String>,
        body_links: Vec<String>,
    }
    let mut all_edges: Vec<DocEdges> = Vec::new();

    // First pass: create nodes
    for (_score, doc_addr) in &top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;

        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Apply type filter
        if !filter.types.is_empty() && !filter.types.contains(&page_type) {
            continue;
        }

        let node = PageNode {
            slug: slug.clone(),
            title,
            r#type: page_type,
        };
        let idx = graph.add_node(node);
        slug_to_idx.insert(slug.clone(), idx);

        // Collect edge fields from stored tags-like fields
        // sources and concepts are stored in the tags field as space-separated
        // but we need them from frontmatter. In Phase 1, they're not in the
        // index as separate fields — we read body_links from the index.
        // For sources/concepts, we parse from the tags field... but actually
        // those aren't stored separately in Phase 1's hardcoded schema.
        //
        // body_links IS stored as a multi-valued keyword field.
        let body_links: Vec<String> = doc
            .get_all(f_body_links)
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        all_edges.push(DocEdges {
            slug,
            sources: Vec::new(),
            concepts: Vec::new(),
            body_links,
        });
    }

    // Second pass: add edges
    for doc_edges in &all_edges {
        let from_idx = match slug_to_idx.get(&doc_edges.slug) {
            Some(idx) => *idx,
            None => continue,
        };

        // sources → "fed-by"
        if filter.relation.is_none() || filter.relation.as_deref() == Some("fed-by") {
            for target in &doc_edges.sources {
                if let Some(&to_idx) = slug_to_idx.get(target) {
                    if from_idx != to_idx {
                        graph.add_edge(
                            from_idx,
                            to_idx,
                            LabeledEdge {
                                relation: "fed-by".into(),
                            },
                        );
                    }
                }
            }
        }

        // concepts → "depends-on"
        if filter.relation.is_none() || filter.relation.as_deref() == Some("depends-on") {
            for target in &doc_edges.concepts {
                if let Some(&to_idx) = slug_to_idx.get(target) {
                    if from_idx != to_idx {
                        graph.add_edge(
                            from_idx,
                            to_idx,
                            LabeledEdge {
                                relation: "depends-on".into(),
                            },
                        );
                    }
                }
            }
        }

        // body_links → "links-to"
        if filter.relation.is_none() || filter.relation.as_deref() == Some("links-to") {
            for target in &doc_edges.body_links {
                if let Some(&to_idx) = slug_to_idx.get(target) {
                    if from_idx != to_idx {
                        graph.add_edge(
                            from_idx,
                            to_idx,
                            LabeledEdge {
                                relation: "links-to".into(),
                            },
                        );
                    }
                }
            }
        }
    }

    // Apply root + depth filter
    if let Some(ref root_slug) = filter.root {
        return Ok(subgraph(&graph, root_slug, filter.depth.unwrap_or(3)));
    }

    Ok(graph)
}

// ── render_mermaid ────────────────────────────────────────────────────────────

pub fn render_mermaid(graph: &WikiGraph) -> String {
    let mut out = String::from("graph LR\n");

    // Collect unique types for classDef
    let mut types_seen: HashSet<String> = HashSet::new();

    // Declare nodes with titles and type classes
    for idx in graph.node_indices() {
        let node = &graph[idx];
        let safe_slug = mermaid_id(&node.slug);
        out.push_str(&format!(
            "  {safe_slug}[\"{}\"]:::{}\n",
            node.title, node.r#type
        ));
        types_seen.insert(node.r#type.clone());
    }

    out.push('\n');

    // Edges with relation labels
    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let from_id = mermaid_id(&graph[from].slug);
        let to_id = mermaid_id(&graph[to].slug);
        let relation = &graph[edge].relation;
        out.push_str(&format!("  {from_id} -->|{relation}| {to_id}\n"));
    }

    // classDef for known types
    out.push('\n');
    let type_colors = [
        ("concept", "#cce5ff"),
        ("query-result", "#cce5ff"),
        ("paper", "#d4edda"),
        ("article", "#d4edda"),
        ("documentation", "#d4edda"),
        ("skill", "#ffeeba"),
        ("doc", "#e2e3e5"),
        ("section", "#f8f9fa"),
    ];
    for (t, color) in &type_colors {
        if types_seen.contains(*t) {
            out.push_str(&format!("  classDef {t} fill:{color}\n"));
        }
    }

    out
}

fn mermaid_id(slug: &str) -> String {
    slug.replace(['/', '-'], "_")
}

// ── render_dot ────────────────────────────────────────────────────────────────

pub fn render_dot(graph: &WikiGraph) -> String {
    let mut out = String::from("digraph wiki {\n");

    for idx in graph.node_indices() {
        let node = &graph[idx];
        out.push_str(&format!(
            "  \"{}\" [label=\"{}\" type=\"{}\"];\n",
            node.slug, node.title, node.r#type
        ));
    }

    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let relation = &graph[edge].relation;
        out.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            graph[from].slug, graph[to].slug, relation
        ));
    }

    out.push_str("}\n");
    out
}

// ── wrap_graph_md ─────────────────────────────────────────────────────────────

pub fn wrap_graph_md(rendered: &str, format: &str, filter: &GraphFilter) -> String {
    let now = Utc::now().to_rfc3339();
    let root = filter.root.as_deref().unwrap_or("");
    let depth = filter.depth.unwrap_or(0);
    let types = if filter.types.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", filter.types.join(", "))
    };

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("title: \"Wiki Graph\"\n");
    out.push_str(&format!("generated: \"{now}\"\n"));
    out.push_str(&format!("format: {format}\n"));
    out.push_str(&format!("root: {root}\n"));
    out.push_str(&format!("depth: {depth}\n"));
    out.push_str(&format!("types: {types}\n"));
    out.push_str("status: generated\n");
    out.push_str("---\n\n");
    out.push_str(&format!("```{format}\n"));
    out.push_str(rendered);
    out.push_str("```\n");
    out
}

// ── subgraph ──────────────────────────────────────────────────────────────────

pub fn subgraph(graph: &WikiGraph, root_slug: &str, depth: usize) -> WikiGraph {
    let root_idx = match graph
        .node_indices()
        .find(|&idx| graph[idx].slug == root_slug)
    {
        Some(idx) => idx,
        None => return WikiGraph::new(),
    };

    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    queue.push_back((root_idx, 0));
    visited.insert(root_idx);

    while let Some((node, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
            if visited.insert(neighbor) {
                queue.push_back((neighbor, d + 1));
            }
        }
        for neighbor in graph.neighbors_directed(node, Direction::Incoming) {
            if visited.insert(neighbor) {
                queue.push_back((neighbor, d + 1));
            }
        }
    }

    let mut new_graph = WikiGraph::new();
    let mut old_to_new: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    for &old_idx in &visited {
        let new_idx = new_graph.add_node(graph[old_idx].clone());
        old_to_new.insert(old_idx, new_idx);
    }

    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        if let (Some(&new_from), Some(&new_to)) = (old_to_new.get(&from), old_to_new.get(&to)) {
            new_graph.add_edge(new_from, new_to, graph[edge].clone());
        }
    }

    new_graph
}
