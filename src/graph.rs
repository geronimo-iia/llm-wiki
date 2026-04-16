use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::frontmatter::parse_frontmatter;
use crate::links::extract_links;
use crate::markdown::slug_for;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageNode {
    pub slug: String,
    pub title: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Default)]
pub struct GraphFilter {
    pub root: Option<String>,
    pub depth: Option<usize>,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphReport {
    pub nodes: usize,
    pub edges: usize,
    pub output: String,
    pub committed: bool,
}

// ── build_graph ───────────────────────────────────────────────────────────────

pub fn build_graph(wiki_root: &Path, filter: &GraphFilter) -> DiGraph<PageNode, ()> {
    let mut graph = DiGraph::new();
    let mut slug_to_idx: HashMap<String, NodeIndex> = HashMap::new();

    // First pass: collect all pages as nodes
    for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let slug = slug_for(path, wiki_root);
        let (title, page_type) = if let Ok((fm, _)) = parse_frontmatter(&content) {
            (fm.title, fm.r#type)
        } else {
            continue;
        };

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
        slug_to_idx.insert(slug, idx);
    }

    // Second pass: add edges
    for entry in WalkDir::new(wiki_root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let slug = slug_for(path, wiki_root);
        let from_idx = match slug_to_idx.get(&slug) {
            Some(idx) => *idx,
            None => continue,
        };

        for link in extract_links(&content) {
            // Only add edge if target exists (skip broken references)
            if let Some(&to_idx) = slug_to_idx.get(&link) {
                if from_idx != to_idx {
                    graph.add_edge(from_idx, to_idx, ());
                }
            }
        }
    }

    // Apply root + depth filter if specified
    if let Some(ref root_slug) = filter.root {
        return subgraph(&graph, root_slug, filter.depth.unwrap_or(3));
    }

    graph
}

// ── render_mermaid ────────────────────────────────────────────────────────────

pub fn render_mermaid(graph: &DiGraph<PageNode, ()>) -> String {
    let mut out = String::from("graph TD\n");
    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let from_slug = &graph[from].slug;
        let to_slug = &graph[to].slug;
        out.push_str(&format!("  {from_slug} --> {to_slug}\n"));
    }
    if graph.edge_count() == 0 && graph.node_count() > 0 {
        // Show isolated nodes
        for idx in graph.node_indices() {
            out.push_str(&format!("  {}\n", graph[idx].slug));
        }
    }
    out
}

// ── render_dot ────────────────────────────────────────────────────────────────

pub fn render_dot(graph: &DiGraph<PageNode, ()>) -> String {
    let mut out = String::from("digraph wiki {\n");
    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        let from_slug = &graph[from].slug;
        let to_slug = &graph[to].slug;
        out.push_str(&format!("  \"{from_slug}\" -> \"{to_slug}\";\n"));
    }
    out.push_str("}\n");
    out
}

// ── subgraph ──────────────────────────────────────────────────────────────────

pub fn subgraph(
    graph: &DiGraph<PageNode, ()>,
    root_slug: &str,
    depth: usize,
) -> DiGraph<PageNode, ()> {
    let root_idx = match graph
        .node_indices()
        .find(|&idx| graph[idx].slug == root_slug)
    {
        Some(idx) => idx,
        None => return DiGraph::new(),
    };

    // BFS to find all nodes within depth hops
    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    queue.push_back((root_idx, 0));
    visited.insert(root_idx);

    while let Some((node, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        // Follow both outgoing and incoming edges
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

    // Build new graph with only visited nodes
    let mut new_graph = DiGraph::new();
    let mut old_to_new: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    for &old_idx in &visited {
        let new_idx = new_graph.add_node(graph[old_idx].clone());
        old_to_new.insert(old_idx, new_idx);
    }

    for edge in graph.edge_indices() {
        let (from, to) = graph.edge_endpoints(edge).unwrap();
        if let (Some(&new_from), Some(&new_to)) = (old_to_new.get(&from), old_to_new.get(&to)) {
            new_graph.add_edge(new_from, new_to, ());
        }
    }

    new_graph
}

// ── in_degree ─────────────────────────────────────────────────────────────────

pub fn in_degree(graph: &DiGraph<PageNode, ()>, slug: &str) -> usize {
    graph
        .node_indices()
        .find(|&idx| graph[idx].slug == slug)
        .map(|idx| graph.neighbors_directed(idx, Direction::Incoming).count())
        .unwrap_or(0)
}
