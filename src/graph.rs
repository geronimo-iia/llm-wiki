use std::cmp::Reverse;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use tantivy::Searcher;
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value;

use petgraph_live::cache::GenerationCache;
use petgraph_live::live::GraphState;

use crate::index_manager::SpaceIndexManager;
use crate::index_schema::IndexSchema;
use crate::links::ParsedLink;
use crate::type_registry::SpaceTypeRegistry;

// ── Types ─────────────────────────────────────────────────────────────────────

/// A node in the concept graph representing one wiki page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNode {
    /// Slug identifying this page within its wiki.
    pub slug: String,
    /// Display title of the page.
    pub title: String,
    /// Frontmatter type of the page.
    pub r#type: String,
    /// True for cross-wiki placeholder nodes not present in the local index.
    #[serde(default)]
    pub external: bool,
}

/// A directed edge in the wiki concept graph with a relation label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledEdge {
    /// Relation label (e.g. `"links-to"`, `"cites"`, `"supersedes"`).
    pub relation: String,
}

/// Directed graph type used for the wiki concept graph.
pub type WikiGraph = DiGraph<PageNode, LabeledEdge>;

/// Filtering parameters for graph construction and subgraph extraction.
#[derive(Debug, Clone, Default)]
pub struct GraphFilter {
    /// Root slug for subgraph extraction (None = full graph).
    pub root: Option<String>,
    /// Maximum hop depth from root (None = use config default).
    pub depth: Option<usize>,
    /// Page types to include (empty = all types).
    pub types: Vec<String>,
    /// Edge relation label to filter on (None = all relations).
    pub relation: Option<String>,
}

impl GraphFilter {
    /// Returns `true` when the filter represents an unfiltered full-graph request.
    /// Note: `depth` is intentionally excluded — a depth-limited full graph still uses the full cache.
    pub fn is_default(&self) -> bool {
        self.root.is_none() && self.types.is_empty() && self.relation.is_none()
    }
}

/// Summary of a completed graph build or render operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphReport {
    /// Total number of nodes in the graph.
    pub nodes: usize,
    /// Total number of edges in the graph.
    pub edges: usize,
    /// Rendered graph content (Mermaid, DOT, or LLM text).
    pub output: String,
}

/// Health metrics computed from a built wiki graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Total number of nodes.
    pub nodes: usize,
    /// Total number of edges.
    pub edges: usize,
    /// Number of nodes with no incoming or outgoing edges.
    pub orphans: usize,
    /// Mean edge count per node (edges × 2 / nodes).
    pub avg_connections: f64,
    /// Graph density: edges / (nodes × (nodes − 1)).
    pub density: f64,
}

/// Compute health metrics from a built graph.
pub fn compute_metrics(graph: &WikiGraph) -> GraphMetrics {
    let nodes = graph.node_count();
    let edges = graph.edge_count();

    let orphans = graph
        .node_indices()
        .filter(|&idx| {
            graph.neighbors_directed(idx, Direction::Incoming).count() == 0
                && graph.neighbors_directed(idx, Direction::Outgoing).count() == 0
        })
        .count();

    let avg_connections = if nodes > 0 {
        (edges as f64 * 2.0) / nodes as f64
    } else {
        0.0
    };

    let density = if nodes > 1 {
        edges as f64 / (nodes as f64 * (nodes as f64 - 1.0))
    } else {
        0.0
    };

    GraphMetrics {
        nodes,
        edges,
        orphans,
        avg_connections,
        density,
    }
}

// ── Community detection (Louvain) ─────────────────────────────────────────────

/// Louvain community detection results for a wiki graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityStats {
    /// Number of distinct clusters found.
    pub count: usize,
    /// Size (node count) of the largest cluster.
    pub largest: usize,
    /// Size (node count) of the smallest cluster.
    pub smallest: usize,
    /// Slugs of pages in communities of size ≤ 2 — weakly connected pages.
    pub isolated: Vec<String>,
}

/// Cached community detection results for a space.
pub struct CommunityData {
    /// Number of local (non-external) nodes in the graph at cache time.
    pub local_count: usize,
    /// Slug → community id map.
    pub map: Arc<HashMap<String, usize>>,
    /// Aggregated Louvain stats.
    pub stats: CommunityStats,
}

/// Graph cache abstraction — either in-memory only or snapshot-backed.
///
/// Constructed by `mount_space` based on `GraphConfig.snapshot`.
/// `NoSnapshot` preserves Phase 1 behaviour; `WithSnapshot` adds warm-start.
pub enum WikiGraphCache {
    NoSnapshot(GenerationCache<WikiGraph>),
    WithSnapshot(GraphState<WikiGraph>),
}

impl WikiGraphCache {
    /// Return the current graph, rebuilding if the generation key changed.
    pub fn get_fresh(
        &self,
        current_gen: u64,
        builder: impl FnOnce() -> anyhow::Result<WikiGraph>,
    ) -> anyhow::Result<Arc<WikiGraph>> {
        match self {
            WikiGraphCache::NoSnapshot(cache) => cache.get_or_build(current_gen, builder),
            WikiGraphCache::WithSnapshot(state) => {
                state.get_fresh().map_err(|e| anyhow::anyhow!("{e}"))
            }
        }
    }

    /// Force a full rebuild and (if snapshot-backed) persist a new snapshot.
    pub fn rebuild(
        &self,
        current_gen: u64,
        builder: impl FnOnce() -> anyhow::Result<WikiGraph>,
    ) -> anyhow::Result<Arc<WikiGraph>> {
        match self {
            WikiGraphCache::NoSnapshot(cache) => {
                cache.invalidate();
                cache.get_or_build(current_gen, builder)
            }
            WikiGraphCache::WithSnapshot(state) => {
                state.rebuild().map_err(|e| anyhow::anyhow!("{e}"))
            }
        }
    }
}

/// Build undirected adjacency by symmetrizing the directed graph. External nodes excluded.
fn build_adjacency(graph: &WikiGraph) -> HashMap<NodeIndex, HashSet<NodeIndex>> {
    let mut adj: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();
    for idx in graph.node_indices() {
        if !graph[idx].external {
            adj.entry(idx).or_default();
        }
    }
    for edge in graph.edge_indices() {
        let (a, b) = graph.edge_endpoints(edge).unwrap();
        if graph[a].external || graph[b].external {
            continue;
        }
        adj.entry(a).or_default().insert(b);
        adj.entry(b).or_default().insert(a);
    }
    adj
}

/// Louvain phase 1: greedy modularity optimisation — each node moves to the neighboring
/// community with the highest modularity gain, applied immediately (in-place).
///
/// Repeats until no node moves in a full pass, capped at `n × 10` passes to prevent
/// oscillation: mid-pass moves alter `sigma_tot` for later nodes, which can cause
/// them to swap back, creating a cycle that never terminates on small or cyclic graphs.
///
/// Returns `true` if any move occurred.
fn louvain_phase1(
    adj: &HashMap<NodeIndex, HashSet<NodeIndex>>,
    community: &mut HashMap<NodeIndex, usize>,
    degrees: &HashMap<NodeIndex, usize>,
    m: usize,
) -> bool {
    if m == 0 {
        return false;
    }
    let m_f = m as f64;

    let mut sorted_nodes: Vec<NodeIndex> = adj.keys().copied().collect();
    // Sort by slug for determinism — we need the graph ref here; use NodeIndex raw id as proxy
    // (caller guarantees deterministic ordering via node insertion order from sorted-slug pass)
    sorted_nodes.sort_by_key(|n| n.index());

    let mut moved = false;
    let max_passes = sorted_nodes.len().max(10) * 10;
    let mut pass = 0;

    loop {
        if pass >= max_passes {
            break;
        }
        pass += 1;
        let mut any_move = false;
        for &node in &sorted_nodes {
            let current_c = *community.get(&node).unwrap();
            let k_i = *degrees.get(&node).unwrap_or(&0) as f64;

            // Gather neighboring communities and k_i_in for each
            let mut neighbor_c_edges: HashMap<usize, usize> = HashMap::new();
            for &nb in adj.get(&node).into_iter().flatten() {
                let nb_c = *community.get(&nb).unwrap();
                *neighbor_c_edges.entry(nb_c).or_default() += 1;
            }

            // sigma_tot per community (sum of degrees)
            let mut sigma_tot: HashMap<usize, f64> = HashMap::new();
            for (&n2, &c2) in community.iter() {
                if n2 == node {
                    continue;
                }
                let d = *degrees.get(&n2).unwrap_or(&0) as f64;
                *sigma_tot.entry(c2).or_default() += d;
            }

            // Find best community
            let mut best_c = current_c;
            let mut best_gain = 0.0_f64;

            for (&c, &k_i_in) in &neighbor_c_edges {
                if c == current_c {
                    continue;
                }
                let st = *sigma_tot.get(&c).unwrap_or(&0.0);
                let gain = (k_i_in as f64) / m_f - st * k_i / (2.0 * m_f * m_f);
                if gain > best_gain {
                    best_gain = gain;
                    best_c = c;
                }
            }

            if best_c != current_c {
                community.insert(node, best_c);
                any_move = true;
                moved = true;
            }
        }
        if !any_move {
            break;
        }
    }
    moved
}

/// Run Louvain community detection on `graph`. Returns `None` when local node count < `min_nodes`.
/// Delegates to `build_community_data` — see its doc for algorithm details.
pub fn compute_communities(graph: &WikiGraph, min_nodes: usize) -> Option<CommunityStats> {
    build_community_data(graph, min_nodes).0
}

/// Returns slug → community id map, or `None` when below threshold.
/// Delegates to `build_community_data` — shares the same Louvain run as `compute_communities`.
pub fn node_community_map(graph: &WikiGraph, min_nodes: usize) -> Option<HashMap<String, usize>> {
    build_community_data(graph, min_nodes).1
}

// ── build_graph ───────────────────────────────────────────────────────────────

/// Build the concept graph from the tantivy index. No file I/O.
/// Edge relations come from `x-graph-edges` declarations in the type registry.
/// Body `[[wikilinks]]` get a generic `links-to` relation.
pub fn build_graph(
    searcher: &Searcher,
    is: &IndexSchema,
    filter: &GraphFilter,
    registry: &SpaceTypeRegistry,
) -> Result<WikiGraph> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_body_links = is.field("body_links");

    let top_docs = searcher.search(&AllQuery, &TopDocs::with_limit(100_000).order_by_score())?;

    let mut graph = WikiGraph::new();
    let mut slug_to_idx: HashMap<String, NodeIndex> = HashMap::new();

    struct DocInfo {
        slug: String,
        page_type: String,
        body_links: Vec<String>,
        edge_fields: Vec<(String, Vec<String>)>, // (field_name, target_slugs)
    }
    let mut all_docs: Vec<DocInfo> = Vec::new();

    // First pass: create nodes and collect edge data
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

        if !filter.types.is_empty() && !filter.types.contains(&page_type) {
            continue;
        }

        let node = PageNode {
            slug: slug.clone(),
            title,
            r#type: page_type.clone(),
            external: false,
        };
        let idx = graph.add_node(node);
        slug_to_idx.insert(slug.clone(), idx);

        // Read body wiki-links
        let body_links: Vec<String> = doc
            .get_all(f_body_links)
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        // Read declared edge fields from the index
        let mut edge_fields = Vec::new();
        for decl in registry.edges(&page_type) {
            if let Some(field_handle) = is.try_field(&decl.field) {
                let targets: Vec<String> = doc
                    .get_all(field_handle)
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if !targets.is_empty() {
                    edge_fields.push((decl.field.clone(), targets));
                }
            }
        }

        all_docs.push(DocInfo {
            slug,
            page_type,
            body_links,
            edge_fields,
        });
    }

    // Second pass: add edges
    for doc_info in &all_docs {
        let from_idx = match slug_to_idx.get(&doc_info.slug) {
            Some(idx) => *idx,
            None => continue,
        };

        // Declared edges (from x-graph-edges)
        let edge_decls = registry.edges(&doc_info.page_type);
        for (field_name, targets) in &doc_info.edge_fields {
            let relation = edge_decls
                .iter()
                .find(|d| d.field == *field_name)
                .map(|d| d.relation.as_str())
                .unwrap_or("links-to");

            if filter.relation.is_some() && filter.relation.as_deref() != Some(relation) {
                continue;
            }

            for target in targets {
                let to_idx = resolve_or_external(target, &mut graph, &mut slug_to_idx);
                if let Some(to_idx) = to_idx
                    && from_idx != to_idx
                {
                    graph.add_edge(
                        from_idx,
                        to_idx,
                        LabeledEdge {
                            relation: relation.to_string(),
                        },
                    );
                }
            }
        }

        // Body wiki-links → "links-to"
        if filter.relation.is_none() || filter.relation.as_deref() == Some("links-to") {
            for target in &doc_info.body_links {
                let to_idx = resolve_or_external(target, &mut graph, &mut slug_to_idx);
                if let Some(to_idx) = to_idx
                    && from_idx != to_idx
                {
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

    // Apply root + depth filter
    if let Some(ref root_slug) = filter.root {
        return Ok(subgraph(&graph, root_slug, filter.depth.unwrap_or(3)));
    }

    Ok(graph)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Resolve a target slug to a node index. If the target is a `wiki://` URI,
/// insert an external placeholder node on demand. Returns `None` only for
/// plain local slugs that don't exist in the index.
fn resolve_or_external(
    target: &str,
    graph: &mut WikiGraph,
    slug_to_idx: &mut HashMap<String, NodeIndex>,
) -> Option<NodeIndex> {
    if target.starts_with("wiki://") {
        let key = target.to_string();
        let idx = *slug_to_idx.entry(key.clone()).or_insert_with(|| {
            let (_wiki, slug) = match ParsedLink::parse(target) {
                ParsedLink::CrossWiki { wiki, slug } => (wiki, slug),
                ParsedLink::Local(_) => ("external".to_string(), target.to_string()),
            };
            graph.add_node(PageNode {
                slug: slug.clone(),
                title: key.clone(),
                r#type: "external".to_string(),
                external: true,
            })
        });
        Some(idx)
    } else {
        slug_to_idx.get(target).copied()
    }
}

// ── build_graph_cross_wiki ────────────────────────────────────────────────────

/// Build a unified graph merging all provided wikis. Cross-wiki edges that
/// were external placeholders in single-wiki graphs become resolved connections
/// when both endpoint wikis are present in `wikis`.
pub fn build_graph_cross_wiki(
    wikis: &[(&str, &Searcher, &IndexSchema, &SpaceTypeRegistry)],
    filter: &GraphFilter,
) -> Result<WikiGraph> {
    // Build per-wiki graphs and merge into one, prefixing slugs with wiki name
    let mut merged = WikiGraph::new();
    // Map from "wikiname/slug" -> NodeIndex in merged graph
    let mut global_idx: HashMap<String, NodeIndex> = HashMap::new();

    // First: add all local (non-external) nodes from each wiki
    for (wiki_name, searcher, is, registry) in wikis {
        let g = build_graph(searcher, is, filter, registry)?;
        for idx in g.node_indices() {
            let node = &g[idx];
            if node.external {
                continue; // will re-resolve below
            }
            let key = format!("{wiki_name}/{}", node.slug);
            let new_idx = merged.add_node(PageNode {
                slug: key.clone(),
                title: node.title.clone(),
                r#type: node.r#type.clone(),
                external: false,
            });
            global_idx.insert(key, new_idx);
        }
    }

    // Second: add edges, re-resolving cross-wiki targets
    for (wiki_name, searcher, is, registry) in wikis {
        let g = build_graph(searcher, is, filter, registry)?;
        for edge_idx in g.edge_indices() {
            let (from, to) = g.edge_endpoints(edge_idx).unwrap();
            let from_node = &g[from];
            let to_node = &g[to];

            let from_key = format!("{wiki_name}/{}", from_node.slug);
            let from_merged = match global_idx.get(&from_key) {
                Some(&i) => i,
                None => continue,
            };

            // to_node is external if it has external=true; its title is the wiki:// URI
            let to_key = if to_node.external {
                // title was set to "wiki://otherwiki/slug"
                if let ParsedLink::CrossWiki { wiki, slug } = ParsedLink::parse(&to_node.title) {
                    format!("{wiki}/{slug}")
                } else {
                    continue;
                }
            } else {
                format!("{wiki_name}/{}", to_node.slug)
            };

            let to_merged = match global_idx.get(&to_key) {
                Some(&i) => i,
                None => {
                    // target wiki not mounted — keep as external placeholder
                    *global_idx.entry(to_key.clone()).or_insert_with(|| {
                        merged.add_node(PageNode {
                            slug: to_key.clone(),
                            title: to_node.title.clone(),
                            r#type: "external".to_string(),
                            external: true,
                        })
                    })
                }
            };

            if from_merged != to_merged {
                merged.add_edge(
                    from_merged,
                    to_merged,
                    LabeledEdge {
                        relation: g[edge_idx].relation.clone(),
                    },
                );
            }
        }
    }

    Ok(merged)
}

// ── merge_cached_graphs ──────────────────────────────────────────────────────

/// Merge pre-built per-space graphs into a single cross-wiki graph.
/// Accepts `Arc<WikiGraph>` inputs (from cache) instead of building from index.
/// Matches the slug-prefixing and external-node resolution of `build_graph_cross_wiki`.
///
/// # Precondition
/// Each `Arc<WikiGraph>` in `wikis` should have been built with the same `filter`.
/// The relation and type filters are re-applied here as a safety gate, but if the
/// cached graph was built without a filter, this function is the only gate.
///
/// `filter.root` and `filter.depth` are NOT re-applied — `get_or_build_graph` only
/// caches the full unfiltered graph (it bypasses cache for non-default filters), so
/// subgraph traversal from a root must be done by the caller after merging.
/// In `ops/graph.rs`, the cross-wiki path always calls `get_or_build_graph` with
/// `is_default()` filter, so this precondition holds in practice.
pub fn merge_cached_graphs(
    wikis: &[(&str, Arc<WikiGraph>)],
    filter: &GraphFilter,
) -> Result<WikiGraph> {
    let mut merged = WikiGraph::new();
    let mut global_idx: HashMap<String, NodeIndex> = HashMap::new();

    // First pass: add all local (non-external) nodes with "wikiname/slug" keys
    for (wiki_name, graph) in wikis {
        for idx in graph.node_indices() {
            let node = &graph[idx];
            if node.external {
                continue;
            }
            // Type filter re-applied here — matches build_graph_cross_wiki's first-pass filter.
            // Precondition: input graphs should have been built with matching filter.
            if !filter.types.is_empty() && !filter.types.contains(&node.r#type) {
                continue;
            }
            let key = format!("{wiki_name}/{}", node.slug);
            let new_idx = merged.add_node(PageNode {
                slug: key.clone(),
                title: node.title.clone(),
                r#type: node.r#type.clone(),
                external: false,
            });
            global_idx.insert(key, new_idx);
        }
    }

    // Second pass: add edges, re-resolving cross-wiki external nodes
    for (wiki_name, graph) in wikis {
        for edge_idx in graph.edge_indices() {
            let (from, to) = graph.edge_endpoints(edge_idx).unwrap();
            let from_node = &graph[from];
            let to_node = &graph[to];

            if from_node.external {
                continue;
            }

            // Relation filter re-applied here. If graphs were built without this filter,
            // this is the only gate — see precondition in doc comment.
            let relation = graph[edge_idx].relation.clone();
            if let Some(ref rel_filter) = filter.relation
                && &relation != rel_filter
            {
                continue;
            }

            let from_key = format!("{wiki_name}/{}", from_node.slug);
            let from_merged = match global_idx.get(&from_key) {
                Some(&i) => i,
                None => continue,
            };

            // Resolve destination: external nodes have title = "wiki://otherwiki/slug"
            let to_key = if to_node.external {
                if let ParsedLink::CrossWiki { wiki, slug } = ParsedLink::parse(&to_node.title) {
                    format!("{wiki}/{slug}")
                } else {
                    continue;
                }
            } else {
                format!("{wiki_name}/{}", to_node.slug)
            };

            let to_merged = match global_idx.get(&to_key) {
                Some(&i) => i,
                None => {
                    // Target wiki not mounted — keep as external placeholder
                    *global_idx.entry(to_key.clone()).or_insert_with(|| {
                        merged.add_node(PageNode {
                            slug: to_key.clone(),
                            title: to_node.title.clone(),
                            r#type: "external".to_string(),
                            external: true,
                        })
                    })
                }
            };

            if from_merged != to_merged {
                merged.add_edge(from_merged, to_merged, LabeledEdge { relation });
            }
        }
    }

    Ok(merged)
}

// ── render_llms ───────────────────────────────────────────────────────────────

/// Natural language description of graph structure for direct LLM consumption.
pub fn render_llms(graph: &WikiGraph) -> String {
    let nodes = graph.node_count();
    let edges = graph.edge_count();

    // Separate external placeholder nodes
    let external_refs: Vec<String> = graph
        .node_indices()
        .filter(|&idx| graph[idx].external)
        .map(|idx| graph[idx].title.clone())
        .collect();

    // Group local nodes by type
    let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
    for idx in graph.node_indices() {
        let node = &graph[idx];
        if node.external {
            continue;
        }
        by_type
            .entry(node.r#type.clone())
            .or_default()
            .push(node.title.clone());
    }

    // Sort type groups by count descending
    let mut type_groups: Vec<(String, Vec<String>)> = by_type.into_iter().collect();
    type_groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

    // Count edge relations
    let mut relation_counts: HashMap<String, usize> = HashMap::new();
    for edge in graph.edge_indices() {
        *relation_counts
            .entry(graph[edge].relation.clone())
            .or_default() += 1;
    }
    let mut relations: Vec<(String, usize)> = relation_counts.into_iter().collect();
    relations.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // Compute per-node total degree for hub detection
    let mut degree: Vec<(usize, String)> = graph
        .node_indices()
        .map(|idx| {
            let d = graph.neighbors_directed(idx, Direction::Incoming).count()
                + graph.neighbors_directed(idx, Direction::Outgoing).count();
            (d, graph[idx].title.clone())
        })
        .collect();
    degree.sort_by_key(|a| Reverse(a.0));
    let top_hubs: Vec<String> = degree
        .iter()
        .take(5)
        .filter(|(d, _)| *d > 0)
        .map(|(d, t)| format!("{t} ({d} edges)"))
        .collect();

    // Isolated nodes (no edges at all)
    let isolated: Vec<String> = graph
        .node_indices()
        .filter(|&idx| {
            graph.neighbors_directed(idx, Direction::Incoming).count() == 0
                && graph.neighbors_directed(idx, Direction::Outgoing).count() == 0
        })
        .map(|idx| graph[idx].title.clone())
        .collect();

    let cluster_count = type_groups.len();

    let mut out = String::new();
    out.push_str(&format!(
        "The wiki graph has {nodes} nodes and {edges} edges across {cluster_count} type groups.\n\n"
    ));

    for (type_name, mut titles) in type_groups {
        titles.sort();
        let count = titles.len();
        let sample = if titles.len() > 8 {
            format!("{}, ...", titles[..8].join(", "))
        } else {
            titles.join(", ")
        };
        out.push_str(&format!("**{type_name}** ({count} nodes): {sample}\n"));
    }

    if !top_hubs.is_empty() {
        out.push_str(&format!("\nKey hubs: {}\n", top_hubs.join(", ")));
    }

    if !relations.is_empty() {
        out.push_str("\n**Edges by relation:**\n");
        for (rel, count) in &relations {
            out.push_str(&format!("- `{rel}` ({count})\n"));
        }
    }

    if !isolated.is_empty() {
        out.push_str(&format!(
            "\n**Isolated nodes ({}):** {}\n",
            isolated.len(),
            isolated.join(", ")
        ));
    }

    if !external_refs.is_empty() {
        let mut sorted = external_refs.clone();
        sorted.sort();
        out.push_str(&format!(
            "\n**External references ({}):** {}\n",
            sorted.len(),
            sorted.join(", ")
        ));
    }

    out
}

// ── render_mermaid ────────────────────────────────────────────────────────────

/// Render the wiki graph as a Mermaid `graph LR` diagram.
pub fn render_mermaid(graph: &WikiGraph) -> String {
    let mut out = String::from("graph LR\n");

    // Collect unique types for classDef
    let mut types_seen: HashSet<String> = HashSet::new();

    let mut has_external = false;

    // Declare nodes with titles and type classes
    for idx in graph.node_indices() {
        let node = &graph[idx];
        let safe_id = mermaid_id(&node.title);
        if node.external {
            out.push_str(&format!("  {safe_id}[\"{}\"]:::external\n", node.title));
            has_external = true;
        } else {
            out.push_str(&format!(
                "  {safe_id}[\"{}\"]:::{}\n",
                node.title, node.r#type
            ));
            types_seen.insert(node.r#type.clone());
        }
    }

    out.push('\n');

    // Edges with relation labels
    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let from_id = mermaid_id(&graph[from].title);
        let to_id = mermaid_id(&graph[to].title);
        let relation = &graph[edge].relation;
        out.push_str(&format!("  {from_id} -->|{relation}| {to_id}\n"));
    }

    // classDef for known types + external
    out.push('\n');
    if has_external {
        out.push_str("  classDef external fill:#eee,stroke:#999,stroke-dasharray:5 5\n");
    }
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
    slug.replace("://", "__").replace(['/', '-', ':'], "_")
}

// ── render_dot ────────────────────────────────────────────────────────────────

/// Render the wiki graph as a Graphviz DOT `digraph`.
pub fn render_dot(graph: &WikiGraph) -> String {
    let mut out = String::from("digraph wiki {\n");

    for idx in graph.node_indices() {
        let node = &graph[idx];
        if node.external {
            out.push_str(&format!(
                "  \"{}\" [label=\"{}\" type=\"external\" style=\"dashed\"];\n",
                node.title, node.title
            ));
        } else {
            out.push_str(&format!(
                "  \"{}\" [label=\"{}\" type=\"{}\"];\n",
                node.slug, node.title, node.r#type
            ));
        }
    }

    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let relation = &graph[edge].relation;
        let from_id = if graph[from].external {
            &graph[from].title
        } else {
            &graph[from].slug
        };
        let to_id = if graph[to].external {
            &graph[to].title
        } else {
            &graph[to].slug
        };
        out.push_str(&format!(
            "  \"{from_id}\" -> \"{to_id}\" [label=\"{relation}\"];\n"
        ));
    }

    out.push_str("}\n");
    out
}

// ── wrap_graph_md ─────────────────────────────────────────────────────────────

/// Wrap rendered graph content in a YAML frontmatter + code-fence Markdown document.
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

/// Extract a BFS subgraph rooted at `root_slug` up to `depth` hops in both directions.
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

// ── build_community_data ─────────────────────────────────────────────────────

/// Run Louvain once and return both community outputs.
/// Returns `(None, None)` when local node count < `min_nodes` (pass 0 to always run).
fn build_community_data(
    graph: &WikiGraph,
    min_nodes: usize,
) -> (Option<CommunityStats>, Option<HashMap<String, usize>>) {
    let local_nodes: Vec<NodeIndex> = {
        let mut v: Vec<NodeIndex> = graph
            .node_indices()
            .filter(|&idx| !graph[idx].external)
            .collect();
        v.sort_by(|&a, &b| graph[a].slug.cmp(&graph[b].slug));
        v
    };

    if local_nodes.len() < min_nodes {
        return (None, None);
    }

    let adj = build_adjacency(graph);
    let degrees: HashMap<NodeIndex, usize> =
        local_nodes.iter().map(|&n| (n, adj[&n].len())).collect();
    let m: usize = adj.values().map(|s| s.len()).sum::<usize>() / 2;

    let mut community: HashMap<NodeIndex, usize> = local_nodes
        .iter()
        .enumerate()
        .map(|(i, &n)| (n, i))
        .collect();

    louvain_phase1(&adj, &mut community, &degrees, m);

    // Normalize community ids to contiguous 0..k
    let mut id_remap: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0usize;
    for &n in &local_nodes {
        let c = *community.get(&n).unwrap();
        id_remap.entry(c).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
    }
    for val in community.values_mut() {
        *val = *id_remap.get(val).unwrap();
    }

    // Build community_map
    let community_map: HashMap<String, usize> = local_nodes
        .iter()
        .map(|&n| (graph[n].slug.clone(), community[&n]))
        .collect();

    // Build community_stats (mirrors compute_communities logic)
    let count = next_id;
    let mut sizes: HashMap<usize, usize> = HashMap::new();
    for &c in community.values() {
        *sizes.entry(c).or_default() += 1;
    }
    let largest = sizes.values().copied().max().unwrap_or(0);
    let smallest = sizes.values().copied().min().unwrap_or(0);
    let mut isolated: Vec<String> = local_nodes
        .iter()
        .filter(|&&n| {
            let c = *community.get(&n).unwrap();
            *sizes.get(&c).unwrap_or(&0) <= 2
        })
        .map(|&n| graph[n].slug.clone())
        .collect();
    isolated.sort();

    let stats = CommunityStats {
        count,
        largest,
        smallest,
        isolated,
    };

    (Some(stats), Some(community_map))
}

// ── Cached graph accessors ───────────────────────────────────────────────────

/// Return cached full graph, or build and cache on miss.
/// Filtered (non-default) requests bypass cache entirely.
pub fn get_or_build_graph(
    index_schema: &IndexSchema,
    type_registry: &SpaceTypeRegistry,
    index_manager: &SpaceIndexManager,
    graph_cache: &WikiGraphCache,
    searcher: &Searcher,
    filter: &GraphFilter,
) -> Result<Arc<WikiGraph>> {
    if !filter.is_default() {
        let g = build_graph(searcher, index_schema, filter, type_registry)?;
        return Ok(Arc::new(g));
    }

    let current_gen = index_manager.generation();
    graph_cache.get_fresh(current_gen, || {
        build_graph(
            searcher,
            index_schema,
            &GraphFilter::default(),
            type_registry,
        )
    })
}

/// Return cached community map, or None if graph is below `min_nodes` threshold.
///
/// Hot path (both caches warm): community_cache hits immediately — graph_cache not touched.
/// Cold path (miss): graph built and cached first, community built and cached inside closure.
pub fn get_cached_community_map(
    index_schema: &IndexSchema,
    type_registry: &SpaceTypeRegistry,
    index_manager: &SpaceIndexManager,
    graph_cache: &WikiGraphCache,
    community_cache: &petgraph_live::cache::GenerationCache<CommunityData>,
    searcher: &Searcher,
    min_nodes: usize,
) -> Result<Option<Arc<HashMap<String, usize>>>> {
    let current_gen = index_manager.generation();

    let community = community_cache.get_or_build(current_gen, || -> Result<CommunityData> {
        let graph = graph_cache.get_fresh(current_gen, || {
            build_graph(
                searcher,
                index_schema,
                &GraphFilter::default(),
                type_registry,
            )
        })?;
        let local_count = graph.node_indices().filter(|&i| !graph[i].external).count();
        let (stats_opt, map_opt) = build_community_data(&graph, 0);
        let stats = stats_opt.unwrap_or(CommunityStats {
            count: 0,
            largest: 0,
            smallest: 0,
            isolated: vec![],
        });
        let map = map_opt.unwrap_or_default();
        Ok(CommunityData {
            local_count,
            map: Arc::new(map),
            stats,
        })
    })?;

    if community.local_count < min_nodes {
        return Ok(None);
    }
    Ok(Some(Arc::clone(&community.map)))
}

/// Return cached CommunityStats, or None if graph is below threshold.
///
/// Hot path: community_cache hits immediately — graph_cache not touched.
pub fn get_cached_community_stats(
    index_schema: &IndexSchema,
    type_registry: &SpaceTypeRegistry,
    index_manager: &SpaceIndexManager,
    graph_cache: &WikiGraphCache,
    community_cache: &petgraph_live::cache::GenerationCache<CommunityData>,
    searcher: &Searcher,
    min_nodes: usize,
) -> Result<Option<CommunityStats>> {
    let current_gen = index_manager.generation();

    let community = community_cache.get_or_build(current_gen, || -> Result<CommunityData> {
        let graph = graph_cache.get_fresh(current_gen, || {
            build_graph(
                searcher,
                index_schema,
                &GraphFilter::default(),
                type_registry,
            )
        })?;
        let local_count = graph.node_indices().filter(|&i| !graph[i].external).count();
        let (stats_opt, map_opt) = build_community_data(&graph, 0);
        let stats = stats_opt.unwrap_or(CommunityStats {
            count: 0,
            largest: 0,
            smallest: 0,
            isolated: vec![],
        });
        let map = map_opt.unwrap_or_default();
        Ok(CommunityData {
            local_count,
            map: Arc::new(map),
            stats,
        })
    })?;

    if community.local_count < min_nodes {
        return Ok(None);
    }
    Ok(Some(community.stats.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labeled_edge_serializes() {
        let e = LabeledEdge {
            relation: "links-to".to_string(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: LabeledEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.relation, "links-to");
    }

    #[test]
    fn community_data_constructs() {
        use std::collections::HashMap;
        let data = CommunityData {
            local_count: 3,
            map: std::sync::Arc::new(HashMap::from([("slug-a".to_string(), 0usize)])),
            stats: CommunityStats {
                count: 1,
                largest: 1,
                smallest: 1,
                isolated: vec![],
            },
        };
        assert_eq!(data.local_count, 3);
        assert_eq!(data.map.get("slug-a"), Some(&0));
        assert_eq!(data.stats.count, 1);
    }
}
