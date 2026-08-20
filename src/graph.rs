#![allow(unreachable_pub)]
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use petgraph::Direction;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use petgraph::visit::{
    Data, EdgeRef as _, GraphBase, IntoEdgeReferences, IntoEdges, IntoNeighbors,
    IntoNodeIdentifiers, NodeCount, NodeIndexable, Visitable,
};
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

type InnerGraph = DiGraph<PageNode, LabeledEdge>;

/// Directed graph type used for the wiki concept graph.
/// Wraps petgraph `DiGraph` and maintains `slug_to_node` for O(1) slug lookup.
#[derive(Clone)]
pub struct WikiGraph {
    inner: InnerGraph,
    slug_to_node: HashMap<String, NodeIndex>,
}

impl WikiGraph {
    /// Create an empty graph with no nodes or edges.
    pub fn new() -> Self {
        Self {
            inner: InnerGraph::new(),
            slug_to_node: HashMap::new(),
        }
    }

    /// Return the `NodeIndex` for `slug`, or `None` if not present.
    pub fn node_for_slug(&self, slug: &str) -> Option<NodeIndex> {
        self.slug_to_node.get(slug).copied()
    }

    /// Add a page node and register it in the slug index; return its index.
    pub fn add_node(&mut self, node: PageNode) -> NodeIndex {
        let slug = node.slug.clone();
        let idx = self.inner.add_node(node);
        self.slug_to_node.insert(slug, idx);
        idx
    }

    /// Add a directed edge from `a` to `b` with label `w`; return its index.
    pub fn add_edge(&mut self, a: NodeIndex, b: NodeIndex, w: LabeledEdge) -> EdgeIndex {
        self.inner.add_edge(a, b, w)
    }

    /// Iterate over all node indices in the graph.
    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.inner.node_indices()
    }

    /// Iterate over all edge indices in the graph.
    pub fn edge_indices(&self) -> impl Iterator<Item = EdgeIndex> + '_ {
        self.inner.edge_indices()
    }

    /// Return the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Return the total number of edges.
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Iterate over neighbors of `n` in direction `d` (Incoming or Outgoing).
    pub fn neighbors_directed(
        &self,
        n: NodeIndex,
        d: petgraph::Direction,
    ) -> impl Iterator<Item = NodeIndex> + '_ {
        self.inner.neighbors_directed(n, d)
    }

    /// Iterate over edges incident to `n` in direction `d`.
    pub fn edges_directed(
        &self,
        n: NodeIndex,
        d: petgraph::Direction,
    ) -> petgraph::graph::Edges<'_, LabeledEdge, petgraph::Directed> {
        self.inner.edges_directed(n, d)
    }

    /// Return the source and target node indices for edge `e`, or `None` if removed.
    pub fn edge_endpoints(&self, e: EdgeIndex) -> Option<(NodeIndex, NodeIndex)> {
        self.inner.edge_endpoints(e)
    }

    /// Find the edge index between `a` and `b`, or `None` if no such edge exists.
    pub fn find_edge(&self, a: NodeIndex, b: NodeIndex) -> Option<EdgeIndex> {
        self.inner.find_edge(a, b)
    }

    /// Iterate over neighbors of `n` ignoring edge direction.
    pub fn neighbors_undirected(&self, n: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.inner.neighbors_undirected(n)
    }

    /// Iterate over all outgoing edges from `n`.
    pub fn edges(
        &self,
        n: NodeIndex,
    ) -> petgraph::graph::Edges<'_, LabeledEdge, petgraph::Directed> {
        self.inner.edges(n)
    }
}

impl std::ops::Index<NodeIndex> for WikiGraph {
    type Output = PageNode;
    fn index(&self, idx: NodeIndex) -> &PageNode {
        &self.inner[idx]
    }
}

impl std::ops::Index<EdgeIndex> for WikiGraph {
    type Output = LabeledEdge;
    fn index(&self, idx: EdgeIndex) -> &LabeledEdge {
        &self.inner[idx]
    }
}

impl Default for WikiGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBase for WikiGraph {
    type NodeId = NodeIndex;
    type EdgeId = EdgeIndex;
}

impl Data for WikiGraph {
    type NodeWeight = PageNode;
    type EdgeWeight = LabeledEdge;
}

impl NodeCount for WikiGraph {
    fn node_count(&self) -> usize {
        self.inner.node_count()
    }
}

impl NodeIndexable for WikiGraph {
    fn node_bound(&self) -> usize {
        self.inner.node_bound()
    }

    fn to_index(&self, ix: NodeIndex) -> usize {
        self.inner.to_index(ix)
    }

    fn from_index(&self, ix: usize) -> NodeIndex {
        self.inner.from_index(ix)
    }
}

impl Visitable for WikiGraph {
    type Map = <InnerGraph as Visitable>::Map;

    fn visit_map(&self) -> Self::Map {
        self.inner.visit_map()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        self.inner.reset_map(map);
    }
}

impl<'a> IntoNodeIdentifiers for &'a WikiGraph {
    type NodeIdentifiers = <&'a InnerGraph as IntoNodeIdentifiers>::NodeIdentifiers;

    fn node_identifiers(self) -> Self::NodeIdentifiers {
        (&self.inner).node_identifiers()
    }
}

impl<'a> IntoNeighbors for &'a WikiGraph {
    type Neighbors = <&'a InnerGraph as IntoNeighbors>::Neighbors;

    fn neighbors(self, n: NodeIndex) -> Self::Neighbors {
        self.inner.neighbors(n)
    }
}

impl<'a> IntoEdges for &'a WikiGraph {
    type Edges = <&'a InnerGraph as IntoEdges>::Edges;

    fn edges(self, a: NodeIndex) -> Self::Edges {
        self.inner.edges(a)
    }
}

impl<'a> IntoEdgeReferences for &'a WikiGraph {
    type EdgeRef = <&'a InnerGraph as IntoEdgeReferences>::EdgeRef;
    type EdgeReferences = <&'a InnerGraph as IntoEdgeReferences>::EdgeReferences;

    fn edge_references(self) -> Self::EdgeReferences {
        self.inner.edge_references()
    }
}

impl Serialize for WikiGraph {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(s)
    }
}

impl<'de> Deserialize<'de> for WikiGraph {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let inner = InnerGraph::deserialize(d)?;
        let slug_to_node = inner
            .node_indices()
            .map(|idx| (inner[idx].slug.clone(), idx))
            .collect();
        Ok(Self {
            inner,
            slug_to_node,
        })
    }
}

/// DiGraph never returns None for edges that exist in the graph.
/// All call sites hold an EdgeIndex obtained from the same graph iteration,
/// so the edge is guaranteed to be present.
fn endpoints(g: &WikiGraph, e: EdgeIndex) -> (NodeIndex, NodeIndex) {
    g.edge_endpoints(e)
        .expect("edge index from graph iteration must be valid")
}

/// Filtering parameters for graph construction and subgraph extraction.
#[derive(Debug, Clone)]
pub struct GraphFilter {
    /// Root slug for subgraph extraction (None = full graph).
    pub root: Option<String>,
    /// Maximum hop depth from root (None = use config default).
    pub depth: Option<usize>,
    /// Page types to include (empty = all types).
    pub types: Vec<String>,
    /// Edge relation label to filter on (None = all relations).
    pub relation: Option<String>,
    /// Maximum pages to fetch from the index in one pass (default: 100_000).
    pub max_pages: usize,
}

impl Default for GraphFilter {
    fn default() -> Self {
        Self {
            root: None,
            depth: None,
            types: Vec::new(),
            relation: None,
            max_pages: 100_000,
        }
    }
}

impl GraphFilter {
    /// Returns `true` when the filter represents an unfiltered full-graph request.
    /// `depth` is intentionally excluded: a depth-limited full graph still loads from the full
    /// snapshot cache and applies the hop limit at render time, so the cache key must not vary
    /// by depth.
    /// `max_pages` intentionally excluded — fetch limit, not a graph structure filter;
    /// must not affect snapshot cache key (same as depth).
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

/// A node in the JSON graph output.
#[derive(Debug, Serialize)]
pub struct JsonNode {
    /// Slug identifying this page within its wiki.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Frontmatter type.
    #[serde(rename = "type")]
    pub page_type: String,
    /// True for cross-wiki placeholder nodes not in the local index.
    pub external: bool,
}

/// A directed edge in the JSON graph output.
#[derive(Debug, Serialize)]
pub struct JsonEdge {
    /// Slug of the source node.
    pub from: String,
    /// Slug of the target node.
    pub to: String,
    /// Relation label.
    pub relation: String,
}

/// Full machine-readable graph output for `wiki_graph --format json`.
#[derive(Debug, Serialize)]
pub struct WikiGraphJson {
    /// All nodes in the graph.
    pub nodes: Vec<JsonNode>,
    /// All directed edges.
    pub edges: Vec<JsonEdge>,
    /// Aggregate graph health metrics.
    pub metrics: GraphMetrics,
    /// Louvain community assignments: slug → community_id.
    /// `null` when the graph is too small for community detection (< 3 nodes per community).
    pub communities: Option<std::collections::HashMap<String, usize>>,
}

/// Render a wiki graph as pretty-printed JSON.
///
/// Produces a `WikiGraphJson` with all nodes, edges, metrics, and community
/// assignments. Edges reference nodes by slug. Community assignments are
/// `null` for very small graphs (Louvain requires ≥ 3 nodes per group).
pub fn render_json(graph: &WikiGraph) -> String {
    let nodes: Vec<JsonNode> = graph
        .node_indices()
        .map(|idx| {
            let n = &graph[idx];
            JsonNode {
                slug: n.slug.clone(),
                title: n.title.clone(),
                page_type: n.r#type.clone(),
                external: n.external,
            }
        })
        .collect();

    let edges: Vec<JsonEdge> = graph
        .edge_indices()
        .map(|eidx| {
            let (src, dst) = endpoints(graph, eidx);
            JsonEdge {
                from: graph[src].slug.clone(),
                to: graph[dst].slug.clone(),
                relation: graph[eidx].relation.clone(),
            }
        })
        .collect();

    let output = WikiGraphJson {
        nodes,
        edges,
        metrics: compute_metrics(graph),
        communities: node_community_map(graph, 3).unwrap_or_else(|e| {
            tracing::error!(error = %e, "community map computation failed; omitting from JSON output");
            None
        }),
    };

    serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
        tracing::error!(error = %e, "render_json serialization failed");
        "{}".to_string()
    })
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
            graph
                .neighbors_directed(idx, Direction::Incoming)
                .next()
                .is_none()
                && graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .next()
                    .is_none()
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
}

type CommunityPair = (Option<CommunityStats>, Option<HashMap<String, usize>>);

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
        // SAFETY: edge index comes from edge_indices() on the same graph; always valid.
        let (a, b) = endpoints(graph, edge);
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
) -> Result<bool> {
    if m == 0 {
        return Ok(false);
    }
    debug_assert!(
        community.len() == adj.len(),
        "community map must contain exactly the same nodes as the adjacency map"
    );
    let m_f = m as f64;

    let mut sorted_nodes: Vec<NodeIndex> = adj.keys().copied().collect();
    // Sort by slug for determinism — we need the graph ref here; use NodeIndex raw id as proxy
    // (caller guarantees deterministic ordering via node insertion order from sorted-slug pass)
    sorted_nodes.sort_by_key(|n| n.index());

    let mut moved = false;
    let max_passes = sorted_nodes.len().max(10) * 10;
    let mut pass = 0;
    // Hoisted outside the loop so the heap allocation is reused across passes.
    let mut sigma_tot: HashMap<usize, f64> = HashMap::new();

    loop {
        if pass >= max_passes {
            break;
        }
        pass += 1;
        let mut any_move = false;

        // Precompute sigma_tot once per pass — O(N) instead of O(N) per node.
        // sigma_tot[c] = sum of degrees of all nodes currently in community c.
        // Incremental updates keep it accurate after each move within the pass.
        sigma_tot.clear();
        for (&n2, &c2) in community.iter() {
            let d = *degrees.get(&n2).unwrap_or(&0) as f64;
            *sigma_tot.entry(c2).or_default() += d;
        }

        for &node in &sorted_nodes {
            let current_c = *community.get(&node).ok_or_else(|| {
                anyhow::anyhow!("Louvain: node {:?} absent from community map", node)
            })?;
            let k_i = *degrees.get(&node).unwrap_or(&0) as f64;

            // Gather neighboring communities and k_i_in for each
            let mut neighbor_c_edges: HashMap<usize, usize> = HashMap::new();
            for &nb in adj.get(&node).into_iter().flatten() {
                let nb_c = *community.get(&nb).ok_or_else(|| {
                    anyhow::anyhow!("Louvain: neighbour {:?} absent from community map", nb)
                })?;
                *neighbor_c_edges.entry(nb_c).or_default() += 1;
            }

            // Full ΔQ: gain of joining c minus cost of leaving current_c.
            // Using the full formula guarantees modularity strictly increases on
            // every accepted move, preventing oscillation and ensuring convergence.
            let k_i_in_current = *neighbor_c_edges.get(&current_c).unwrap_or(&0) as f64;
            // sigma_tot[current_c] includes node itself; remove it for leave cost.
            let sigma_s_minus_i = sigma_tot.get(&current_c).unwrap_or(&0.0) - k_i;
            let leave_gain = k_i_in_current / m_f - sigma_s_minus_i * k_i / (2.0 * m_f * m_f);

            // Find best community
            let mut best_c = current_c;
            let mut best_gain = 0.0_f64;

            for (&c, &k_i_in) in &neighbor_c_edges {
                if c == current_c {
                    continue;
                }
                let st = *sigma_tot.get(&c).unwrap_or(&0.0);
                let join_gain = (k_i_in as f64) / m_f - st * k_i / (2.0 * m_f * m_f);
                let gain = join_gain - leave_gain;
                if gain > best_gain {
                    best_gain = gain;
                    best_c = c;
                }
            }

            if best_c != current_c {
                community.insert(node, best_c);
                any_move = true;
                moved = true;
                // Incremental update: node leaves current_c, joins best_c.
                *sigma_tot.entry(current_c).or_default() -= k_i;
                *sigma_tot.entry(best_c).or_default() += k_i;
            }
        }
        if !any_move {
            break;
        }
    }
    Ok(moved)
}

/// Run Louvain community detection on `graph`. Returns `None` when local node count < `min_nodes`.
/// Delegates to `build_community_data` — see its doc for algorithm details.
pub fn compute_communities(graph: &WikiGraph, min_nodes: usize) -> Result<Option<CommunityStats>> {
    Ok(build_community_data(graph, min_nodes)?.0)
}

/// Returns slug → community id map, or `None` when below threshold.
/// Delegates to `build_community_data` — shares the same Louvain run as `compute_communities`.
pub fn node_community_map(
    graph: &WikiGraph,
    min_nodes: usize,
) -> Result<Option<HashMap<String, usize>>> {
    Ok(build_community_data(graph, min_nodes)?.1)
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

    let top_docs = searcher.search(
        &AllQuery,
        &TopDocs::with_limit(filter.max_pages).order_by_score(),
    )?;

    if top_docs.len() >= filter.max_pages {
        tracing::warn!(
            count = top_docs.len(),
            limit = filter.max_pages,
            "graph: TopDocs limit reached — index has ≥{} pages; graph may be silently truncated",
            filter.max_pages
        );
    }

    let mut graph = WikiGraph::new();

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
        graph.add_node(node);

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
        let from_idx = match graph.node_for_slug(&doc_info.slug) {
            Some(idx) => idx,
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
                let to_idx = resolve_or_external(target, &mut graph);
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
                let to_idx = resolve_or_external(target, &mut graph);
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
fn resolve_or_external(target: &str, graph: &mut WikiGraph) -> Option<NodeIndex> {
    if target.starts_with("wiki://") {
        if let Some(idx) = graph.node_for_slug(target) {
            return Some(idx);
        }
        // slug = full URI so slug_to_node key matches node_for_slug(target) above.
        // build_graph_cross_wiki uses node.title (not node.slug) for external resolution,
        // so changing slug from the parsed short form to the full URI is safe.
        let idx = graph.add_node(PageNode {
            slug: target.to_string(),
            title: target.to_string(),
            r#type: "external".to_string(),
            external: true,
        });
        Some(idx)
    } else {
        graph.node_for_slug(target)
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

    // Build per-wiki graphs once; reuse in both passes.
    let per_wiki: Vec<(&str, WikiGraph)> = wikis
        .iter()
        .map(|(wiki_name, searcher, is, registry)| {
            build_graph(searcher, is, filter, registry).map(|g| (*wiki_name, g))
        })
        .collect::<Result<_>>()?;

    // First: add all local (non-external) nodes from each wiki
    for (wiki_name, g) in &per_wiki {
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
    for (wiki_name, g) in &per_wiki {
        for edge_idx in g.edge_indices() {
            // SAFETY: edge index comes from edge_indices() on the same graph; always valid.
            let (from, to) = endpoints(g, edge_idx);
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
            // SAFETY: edge index comes from edge_indices() on the same graph; always valid.
            let (from, to) = endpoints(graph, edge_idx);
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

    // Single pass: collect external refs, type groups, hub degrees, and isolated nodes.
    let mut external_refs: Vec<String> = Vec::new();
    let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
    let mut degree: Vec<(usize, String)> = Vec::new();
    let mut isolated: Vec<String> = Vec::new();
    for idx in graph.node_indices() {
        let node = &graph[idx];
        let d = graph.neighbors_directed(idx, Direction::Incoming).count()
            + graph.neighbors_directed(idx, Direction::Outgoing).count();
        if node.external {
            external_refs.push(node.title.clone());
        } else {
            by_type
                .entry(node.r#type.clone())
                .or_default()
                .push(node.title.clone());
        }
        degree.push((d, node.title.clone()));
        if d == 0 {
            isolated.push(node.title.clone());
        }
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

    degree.sort_by_key(|a| Reverse(a.0));
    let top_hubs: Vec<String> = degree
        .iter()
        .take(5)
        .filter(|(d, _)| *d > 0)
        .map(|(d, t)| format!("{t} ({d} edges)"))
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
        let mut external_refs = external_refs;
        external_refs.sort();
        out.push_str(&format!(
            "\n**External references ({}):** {}\n",
            external_refs.len(),
            external_refs.join(", ")
        ));
    }

    out
}

// ── render_mermaid ────────────────────────────────────────────────────────────

/// Render the wiki graph as a Mermaid `graph LR` diagram.
pub fn render_mermaid(graph: &WikiGraph) -> String {
    let mut out = String::from("graph LR\n");

    // Collect unique types for classDef
    let mut types_seen: HashSet<&str> = HashSet::new();

    let mut has_external = false;

    // Declare nodes with titles and type classes
    for idx in graph.node_indices() {
        let node = &graph[idx];
        let safe_id = format!("N{}", idx.index());
        if node.external {
            out.push_str(&format!("  {safe_id}[\"{}\"]:::external\n", node.title));
            has_external = true;
        } else {
            out.push_str(&format!(
                "  {safe_id}[\"{}\"]:::{}\n",
                node.title, node.r#type
            ));
            types_seen.insert(&node.r#type);
        }
    }

    out.push('\n');

    // Edges with relation labels
    for edge in graph.edge_indices() {
        // SAFETY: edge index comes from edge_indices() on the same graph; always valid.
        let (from, to) = endpoints(graph, edge);
        let from_id = format!("N{}", from.index());
        let to_id = format!("N{}", to.index());
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
        // SAFETY: edge index comes from edge_indices() on the same graph; always valid.
        let (from, to) = endpoints(graph, edge);
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
    let root_idx = match graph.node_for_slug(root_slug) {
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

    for &old_from in &visited {
        for edge_ref in graph.edges_directed(old_from, Direction::Outgoing) {
            let old_to = edge_ref.target();
            if let (Some(&new_from), Some(&new_to)) =
                (old_to_new.get(&old_from), old_to_new.get(&old_to))
            {
                new_graph.add_edge(new_from, new_to, edge_ref.weight().clone());
            }
        }
    }

    new_graph
}

// ── build_community_data ─────────────────────────────────────────────────────

/// Run Louvain once and return both community outputs.
/// Returns `(None, None)` when local node count < `min_nodes` (pass 0 to always run).
fn build_community_data(graph: &WikiGraph, min_nodes: usize) -> Result<CommunityPair> {
    let local_nodes: Vec<NodeIndex> = {
        let mut v: Vec<NodeIndex> = graph
            .node_indices()
            .filter(|&idx| !graph[idx].external)
            .collect();
        v.sort_by(|&a, &b| graph[a].slug.cmp(&graph[b].slug));
        v
    };

    if local_nodes.len() < min_nodes {
        return Ok((None, None));
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

    louvain_phase1(&adj, &mut community, &degrees, m)?;

    // Normalize community ids to contiguous 0..k
    let mut id_remap: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0usize;
    for &n in &local_nodes {
        let c = *community
            .get(&n)
            .ok_or_else(|| anyhow::anyhow!("Louvain: node {:?} absent after phase1", n))?;
        id_remap.entry(c).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
    }
    for val in community.values_mut() {
        *val = *id_remap
            .get(val)
            .ok_or_else(|| anyhow::anyhow!("Louvain: community id {:?} absent from remap", val))?;
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
    let stats = CommunityStats {
        count,
        largest,
        smallest,
    };

    Ok((Some(stats), Some(community_map)))
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

    // generation() increments on every reload_reader() call. last_commit() is NOT used
    // because same-commit schema-triggered rebuilds produce a new index without changing
    // the commit hash — those must also invalidate the graph cache.
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

fn ensure_community_data(
    index_schema: &IndexSchema,
    type_registry: &SpaceTypeRegistry,
    index_manager: &SpaceIndexManager,
    graph_cache: &WikiGraphCache,
    community_cache: &petgraph_live::cache::GenerationCache<CommunityData>,
    searcher: &Searcher,
) -> Result<std::sync::Arc<CommunityData>> {
    // generation() increments on every reload_reader() call. last_commit() is NOT used
    // because same-commit schema-triggered rebuilds produce a new index without changing
    // the commit hash — those must also invalidate the graph cache.
    let current_gen = index_manager.generation();
    community_cache.get_or_build(current_gen, || -> Result<CommunityData> {
        let graph = graph_cache.get_fresh(current_gen, || {
            build_graph(
                searcher,
                index_schema,
                &GraphFilter::default(),
                type_registry,
            )
        })?;
        let local_count = graph.node_indices().filter(|&i| !graph[i].external).count();
        let (stats_opt, map_opt) = build_community_data(&graph, 0)?;
        let stats = stats_opt.unwrap_or(CommunityStats {
            count: 0,
            largest: 0,
            smallest: 0,
        });
        let map = map_opt.unwrap_or_default();
        Ok(CommunityData {
            local_count,
            map: Arc::new(map),
            stats,
        })
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
    let community = ensure_community_data(
        index_schema,
        type_registry,
        index_manager,
        graph_cache,
        community_cache,
        searcher,
    )?;
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
    let community = ensure_community_data(
        index_schema,
        type_registry,
        index_manager,
        graph_cache,
        community_cache,
        searcher,
    )?;
    if community.local_count < min_nodes {
        return Ok(None);
    }
    Ok(Some(community.stats.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_json_output_is_valid_json_with_expected_keys() {
        use serde_json::Value;

        let mut g = WikiGraph::new();
        let a = g.add_node(PageNode {
            slug: "concepts/a".into(),
            title: "A".into(),
            r#type: "concept".into(),
            external: false,
        });
        let b = g.add_node(PageNode {
            slug: "concepts/b".into(),
            title: "B".into(),
            r#type: "concept".into(),
            external: false,
        });
        g.add_edge(
            a,
            b,
            LabeledEdge {
                relation: "links-to".into(),
            },
        );

        let json_str = render_json(&g);
        let v: Value =
            serde_json::from_str(&json_str).expect("render_json must produce valid JSON");

        assert!(
            v.get("nodes").and_then(|n| n.as_array()).is_some(),
            "must have nodes array"
        );
        assert!(
            v.get("edges").and_then(|e| e.as_array()).is_some(),
            "must have edges array"
        );
        assert!(v.get("metrics").is_some(), "must have metrics object");
        assert!(v.get("communities").is_some(), "must have communities key");

        let nodes = v["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0]["slug"], "concepts/a");
        assert_eq!(nodes[0]["type"], "concept");
        assert_eq!(nodes[0]["external"], false);

        let edges = v["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0]["from"], "concepts/a");
        assert_eq!(edges[0]["to"], "concepts/b");
        assert_eq!(edges[0]["relation"], "links-to");

        assert_eq!(v["metrics"]["nodes"], 2);
        assert_eq!(v["metrics"]["edges"], 1);
    }

    #[test]
    fn render_json_empty_graph() {
        let g = WikiGraph::new();
        let json_str = render_json(&g);
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["nodes"].as_array().unwrap().len(), 0);
        assert_eq!(v["edges"].as_array().unwrap().len(), 0);
        assert_eq!(v["metrics"]["nodes"], 0);
    }

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
            },
        };
        assert_eq!(data.local_count, 3);
        assert_eq!(data.map.get("slug-a"), Some(&0));
        assert_eq!(data.stats.count, 1);
    }

    fn make_node(slug: &str, title: &str, r#type: &str, external: bool) -> PageNode {
        PageNode {
            slug: slug.to_string(),
            title: title.to_string(),
            r#type: r#type.to_string(),
            external,
        }
    }

    #[test]
    fn render_mermaid_node_ids_are_valid_and_unique() {
        let mut g = WikiGraph::new();
        // Titles with spaces, special chars, angle brackets — all would break old mermaid_id
        let a = g.add_node(make_node(
            "concepts/arc-str",
            "Arc<str> for Shared Immutable Strings",
            "concept",
            false,
        ));
        let b = g.add_node(make_node(
            "concepts/send-sync",
            "Send and Sync: Thread Safety in Rust",
            "concept",
            false,
        ));
        let c = g.add_node(make_node(
            "concepts/try-join",
            "try_join! macro",
            "concept",
            false,
        ));
        g.add_edge(
            a,
            b,
            LabeledEdge {
                relation: "links-to".to_string(),
            },
        );
        g.add_edge(
            b,
            c,
            LabeledEdge {
                relation: "links-to".to_string(),
            },
        );

        let output = render_mermaid(&g);

        // Every node declaration must use N{digit} as ID
        assert!(
            output.contains(&format!("N{}[", a.index())),
            "node a ID missing: {output}"
        );
        assert!(
            output.contains(&format!("N{}[", b.index())),
            "node b ID missing: {output}"
        );
        assert!(
            output.contains(&format!("N{}[", c.index())),
            "node c ID missing: {output}"
        );

        // Labels must preserve the original title
        assert!(
            output.contains("Arc<str> for Shared Immutable Strings"),
            "label a missing"
        );
        assert!(
            output.contains("Send and Sync: Thread Safety in Rust"),
            "label b missing"
        );
        assert!(output.contains("try_join! macro"), "label c missing");

        // Edges must reference the correct node IDs
        assert!(
            output.contains(&format!("N{} -->|links-to| N{}", a.index(), b.index())),
            "edge a→b missing: {output}"
        );
    }

    #[test]
    fn render_mermaid_external_node_id_valid() {
        let mut g = WikiGraph::new();
        let local = g.add_node(make_node("concepts/foo", "Foo Concept", "concept", false));
        let ext = g.add_node(make_node("bar", "wiki://otherwiki/bar", "external", true));
        g.add_edge(
            local,
            ext,
            LabeledEdge {
                relation: "links-to".to_string(),
            },
        );

        let output = render_mermaid(&g);

        // External node must also get a valid N{n} ID, not the raw wiki:// URI
        assert!(
            output.contains(&format!("N{}[", ext.index())),
            "external node ID must be N{{n}}, got: {output}"
        );
        // The wiki:// URI must appear only in the label, not as a bare ID
        assert!(
            !output.contains("wiki__"),
            "wiki:// must not appear as sanitized ID fragment: {output}"
        );
    }

    // Helper — free function avoids borrow-checker conflict with closure + loop reborrow.
    fn connect(
        adj: &mut HashMap<NodeIndex, HashSet<NodeIndex>>,
        degrees: &mut HashMap<NodeIndex, usize>,
        a: NodeIndex,
        b: NodeIndex,
    ) {
        adj.entry(a).or_default().insert(b);
        adj.entry(b).or_default().insert(a);
        *degrees.entry(a).or_default() += 1;
        *degrees.entry(b).or_default() += 1;
    }

    /// Two clear clusters of 4 nodes each, with one bridge edge.
    /// Louvain should assign all nodes in cluster A the same community id,
    /// and all nodes in cluster B the same community id (different from A).
    #[test]
    fn test_louvain_two_clusters() {
        // Cluster A: nodes 0,1,2,3 — fully connected (6 edges)
        // Cluster B: nodes 4,5,6,7 — fully connected (6 edges)
        // Bridge: 3 -- 4 (1 edge)
        // Total edges m = 13

        use petgraph::graph::NodeIndex;
        use std::collections::{HashMap, HashSet};

        let make = |i: usize| NodeIndex::new(i);

        let cluster_a: Vec<NodeIndex> = (0..4).map(make).collect();
        let cluster_b: Vec<NodeIndex> = (4..8).map(make).collect();

        let mut adj: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();
        let mut degrees: HashMap<NodeIndex, usize> = HashMap::new();

        // Fully connect cluster A
        for i in 0..4usize {
            for j in (i + 1)..4 {
                connect(&mut adj, &mut degrees, make(i), make(j));
            }
        }
        // Fully connect cluster B
        for i in 4..8usize {
            for j in (i + 1)..8 {
                connect(&mut adj, &mut degrees, make(i), make(j));
            }
        }
        // Bridge edge
        connect(&mut adj, &mut degrees, make(3), make(4));

        // Ensure every node has an adjacency entry (even if empty)
        for i in 0..8usize {
            adj.entry(make(i)).or_default();
            degrees.entry(make(i)).or_insert(0);
        }

        let m: usize = degrees.values().sum::<usize>() / 2;

        // Each node starts in its own community
        let mut community: HashMap<NodeIndex, usize> = (0..8usize).map(|i| (make(i), i)).collect();

        louvain_phase1(&adj, &mut community, &degrees, m).unwrap();

        // All cluster A nodes must share one community id
        let ca: HashSet<usize> = cluster_a.iter().map(|n| community[n]).collect();
        assert_eq!(
            ca.len(),
            1,
            "cluster A nodes must all share one community, got {:?}",
            ca
        );

        // All cluster B nodes must share one community id
        let cb: HashSet<usize> = cluster_b.iter().map(|n| community[n]).collect();
        assert_eq!(
            cb.len(),
            1,
            "cluster B nodes must all share one community, got {:?}",
            cb
        );

        // The two clusters must be in different communities
        assert_ne!(
            ca.iter().next(),
            cb.iter().next(),
            "cluster A and cluster B must be in different communities"
        );
    }

    /// 8-node cycle: 0-1-2-3-4-5-6-7-0.
    /// Symmetric topology is oscillation-prone without incremental sigma_tot updates.
    /// The cycle has multiple valid local optima, so independent runs may produce different
    /// valid partitions — that is not oscillation. The oscillation regression is:
    ///   after phase1 reaches a local optimum, a second pass must make zero moves.
    /// Without the sigma_tot incremental update, the algorithm keeps accepting moves
    /// that were only beneficial due to stale sigma_tot, and never stabilises.
    #[test]
    fn test_louvain_converges_no_oscillation() {
        use petgraph::graph::NodeIndex;
        use std::collections::{HashMap, HashSet};

        let n = 8usize;
        let make = |i: usize| NodeIndex::new(i);

        let mut adj: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();
        let mut degrees: HashMap<NodeIndex, usize> = HashMap::new();
        for i in 0..n {
            connect(&mut adj, &mut degrees, make(i), make((i + 1) % n));
        }
        for i in 0..n {
            adj.entry(make(i)).or_default();
            degrees.entry(make(i)).or_insert(0);
        }
        let m: usize = degrees.values().sum::<usize>() / 2;

        // Run 12 times; each run must converge (second pass makes no moves).
        for run in 0..12 {
            let mut community: HashMap<NodeIndex, usize> = (0..n).map(|i| (make(i), i)).collect();
            louvain_phase1(&adj, &mut community, &degrees, m).unwrap();
            let moved = louvain_phase1(&adj, &mut community, &degrees, m).unwrap();
            assert!(
                !moved,
                "run {run}: second louvain_phase1 pass on a converged partition should make no moves — oscillation detected"
            );
        }
    }

    #[test]
    fn wiki_graph_node_for_slug_is_o1() {
        let mut g = WikiGraph::new();
        g.add_node(PageNode {
            slug: "alpha".into(),
            title: "Alpha".into(),
            r#type: "page".into(),
            external: false,
        });
        g.add_node(PageNode {
            slug: "beta".into(),
            title: "Beta".into(),
            r#type: "page".into(),
            external: false,
        });
        assert!(g.node_for_slug("alpha").is_some());
        assert!(g.node_for_slug("beta").is_some());
        assert!(g.node_for_slug("missing").is_none());
    }

    #[test]
    fn subgraph_root_not_found_returns_empty() {
        let g = WikiGraph::new();
        let sub = subgraph(&g, "no-such-slug", 2);
        assert_eq!(sub.node_count(), 0);
        assert_eq!(sub.edge_count(), 0);
    }

    #[test]
    fn subgraph_single_node_depth_zero() {
        let mut g = WikiGraph::new();
        g.add_node(PageNode {
            slug: "root".into(),
            title: "Root".into(),
            r#type: "page".into(),
            external: false,
        });
        g.add_node(PageNode {
            slug: "other".into(),
            title: "Other".into(),
            r#type: "page".into(),
            external: false,
        });
        g.add_edge(
            g.node_for_slug("root").unwrap(),
            g.node_for_slug("other").unwrap(),
            LabeledEdge {
                relation: "links-to".into(),
            },
        );
        let sub = subgraph(&g, "root", 0);
        assert_eq!(sub.node_count(), 1);
    }
}
