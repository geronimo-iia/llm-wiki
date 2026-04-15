//! Concept graph — build an in-memory directed graph from wiki page links and
//! emit it as DOT or Mermaid for visualisation.

use anyhow::Result;
use comrak::{nodes::NodeValue, Arena, Options};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef as _;
use petgraph::Direction;
use petgraph::Graph;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use walkdir::WalkDir;

use crate::markdown::slug_for;

/// The kind of relationship an edge represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
    /// Link written as `[[target]]` in a page body.
    WikiLink,
    /// Slug listed in the `related_concepts` frontmatter field.
    RelatedConcept,
    /// Slug listed in the `contradictions` frontmatter field.
    Contradiction,
}

/// In-memory directed concept graph built from frontmatter links and
/// `[[wikilinks]]` across all wiki pages.
///
/// Nodes are page slugs (relative path without `.md`).
/// Edges are typed by [`EdgeKind`].
pub struct WikiGraph {
    /// Directed graph: nodes are page slugs, edges are typed links.
    pub inner: Graph<String, EdgeKind>,
    /// Fast slug → NodeIndex lookup.
    pub(crate) node_map: HashMap<String, NodeIndex>,
}

impl WikiGraph {
    fn new() -> Self {
        WikiGraph {
            inner: Graph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Return the [`NodeIndex`] for `slug`, inserting a new node if absent.
    fn get_or_insert(&mut self, slug: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(slug) {
            return idx;
        }
        let idx = self.inner.add_node(slug.to_string());
        self.node_map.insert(slug.to_string(), idx);
        idx
    }
}

// ── Partial frontmatter helpers ───────────────────────────────────────────────

/// Minimal frontmatter struct for edge extraction.
///
/// Uses `#[serde(default)]` on every field so both `PageFrontmatter`-shaped
/// pages *and* `ContradictionFrontmatter`-shaped pages parse without error.
#[derive(Debug, Deserialize, Default)]
struct GraphFrontmatter {
    #[serde(default)]
    contradictions: Vec<String>,
    #[serde(default)]
    related_concepts: Vec<String>,
}

/// Extract the raw YAML block from a frontmatter-delimited Markdown file.
///
/// Returns `None` if the file has no `---` frontmatter delimiters.
fn yaml_block(content: &str) -> Option<&str> {
    let after_open = content.strip_prefix("---\n")?;
    let end = after_open.find("\n---\n")?;
    Some(&after_open[..end])
}

/// Return the body portion of a frontmatter-delimited Markdown file.
fn body_after_frontmatter(content: &str) -> &str {
    if !content.starts_with("---\n") {
        return content;
    }
    let after_open = &content[4..];
    if let Some(end) = after_open.find("\n---\n") {
        let after_close = &after_open[end + 5..];
        after_close.strip_prefix('\n').unwrap_or(after_close)
    } else {
        content
    }
}

/// Normalise a link target to a slug by stripping a trailing `.md`.
fn normalise(target: &str) -> String {
    target.trim_end_matches(".md").to_string()
}

// ── Wikilink extraction ───────────────────────────────────────────────────────

/// Extract all `[[target]]` wikilink targets from Markdown body text using comrak.
fn extract_wikilinks(body: &str) -> Vec<String> {
    let arena = Arena::new();
    let mut opts = Options::default();
    opts.extension.wikilinks_title_after_pipe = true;

    let root = comrak::parse_document(&arena, body, &opts);
    let mut targets = Vec::new();

    for node in root.descendants() {
        if let NodeValue::WikiLink(ref wl) = node.data.borrow().value {
            targets.push(wl.url.clone());
        }
    }
    targets
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Build the concept graph by walking all Markdown files under `wiki_root`.
///
/// Edges come from three sources:
/// - `[[wikilink]]` syntax in page bodies (parsed with comrak)
/// - `related_concepts` frontmatter field
/// - `contradictions` frontmatter field
pub fn build_graph(wiki_root: &Path) -> Result<WikiGraph> {
    let mut g = WikiGraph::new();

    // First pass — register every page as a node.
    for entry in WalkDir::new(wiki_root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        let rel = path.strip_prefix(wiki_root).unwrap_or(path);
        if rel.starts_with(".wiki") {
            continue;
        }
        // Skip non-index.md files inside bundle folders (they are assets).
        if let Some(filename) = path.file_name() {
            if filename != OsStr::new("index.md") {
                if let Some(parent) = path.parent() {
                    if parent.join("index.md").exists() {
                        continue;
                    }
                }
            }
        }
        let slug = slug_for(path, wiki_root);
        g.get_or_insert(&slug);
    }

    // Second pass — parse links and add edges.
    for entry in WalkDir::new(wiki_root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        let rel = path.strip_prefix(wiki_root).unwrap_or(path);
        if rel.starts_with(".wiki") {
            continue;
        }
        // Skip non-index.md files inside bundle folders.
        if let Some(filename) = path.file_name() {
            if filename != OsStr::new("index.md") {
                if let Some(parent) = path.parent() {
                    if parent.join("index.md").exists() {
                        continue;
                    }
                }
            }
        }
        let source_slug = slug_for(path, wiki_root);
        let source_idx = *g.node_map.get(source_slug.as_str()).unwrap();

        let content = std::fs::read_to_string(path)?;

        // Frontmatter-based edges.
        let gfm: GraphFrontmatter = yaml_block(&content)
            .and_then(|yaml| serde_yaml::from_str(yaml).ok())
            .unwrap_or_default();

        for target in &gfm.contradictions {
            let t = normalise(target);
            let tidx = g.get_or_insert(&t);
            g.inner.add_edge(source_idx, tidx, EdgeKind::Contradiction);
        }
        for target in &gfm.related_concepts {
            let t = normalise(target);
            let tidx = g.get_or_insert(&t);
            g.inner.add_edge(source_idx, tidx, EdgeKind::RelatedConcept);
        }

        // Wikilink-based edges (from body).
        let body = body_after_frontmatter(&content);
        for target in extract_wikilinks(body) {
            let t = normalise(&target);
            if !t.is_empty() {
                let tidx = g.get_or_insert(&t);
                g.inner.add_edge(source_idx, tidx, EdgeKind::WikiLink);
            }
        }
    }

    Ok(g)
}

/// Return all page slugs with in-degree 0, excluding pages under `raw/`.
pub fn orphans(graph: &WikiGraph) -> Vec<String> {
    let mut result: Vec<String> = graph
        .inner
        .node_indices()
        .filter(|&idx| {
            let slug = &graph.inner[idx];
            if slug == "raw" || slug.starts_with("raw/") {
                return false;
            }
            graph
                .inner
                .edges_directed(idx, Direction::Incoming)
                .next()
                .is_none()
        })
        .map(|idx| graph.inner[idx].clone())
        .collect();
    result.sort();
    result
}

/// Return all slugs that are referenced by an edge but whose `.md` file does
/// not exist on disk under `wiki_root`.
pub fn missing_stubs(graph: &WikiGraph, wiki_root: &Path) -> Vec<String> {
    let mut missing: Vec<String> = graph
        .inner
        .node_indices()
        .filter(|&idx| {
            let slug = &graph.inner[idx];
            // Only include slugs that are *targets* of at least one edge.
            if graph
                .inner
                .edges_directed(idx, Direction::Incoming)
                .next()
                .is_none()
            {
                return false;
            }
            crate::markdown::resolve_slug(wiki_root, slug).is_none()
        })
        .map(|idx| graph.inner[idx].clone())
        .collect();
    missing.sort();
    missing.dedup();
    missing
}

/// Serialise `graph` as a GraphViz DOT string.
///
/// - Nodes labelled with their slug.
/// - Edge styles: `WikiLink` = solid, `RelatedConcept` = dashed, `Contradiction` = dotted.
pub fn dot_output(graph: &WikiGraph) -> String {
    let mut out = String::from("digraph wiki {\n");
    out.push_str("    node [shape=box fontname=\"Helvetica\"];\n");

    for idx in graph.inner.node_indices() {
        let slug = &graph.inner[idx];
        if slug.is_empty() {
            continue;
        }
        let label = slug.replace('"', "\\\"");
        out.push_str(&format!("    n{} [label=\"{}\"];\n", idx.index(), label));
    }

    for edge in graph.inner.edge_references() {
        let (src, tgt) = (edge.source(), edge.target());
        let style = match edge.weight() {
            EdgeKind::WikiLink => "solid",
            EdgeKind::RelatedConcept => "dashed",
            EdgeKind::Contradiction => "dotted",
        };
        out.push_str(&format!(
            "    n{} -> n{} [style={}];\n",
            src.index(),
            tgt.index(),
            style
        ));
    }

    out.push('}');
    out
}

/// Serialise `graph` as a Mermaid `graph TD` string.
pub fn mermaid_output(graph: &WikiGraph) -> String {
    let mut out = String::from("graph TD\n");

    for idx in graph.inner.node_indices() {
        let slug = &graph.inner[idx];
        if slug.is_empty() {
            continue;
        }
        // Mermaid labels: escape double-quotes, replace brackets (reserved by Mermaid).
        let label = slug
            .replace('"', "&quot;")
            .replace('[', "(")
            .replace(']', ")");
        out.push_str(&format!("    n{}[\"{}\"]\n", idx.index(), label));
    }

    for edge in graph.inner.edge_references() {
        let (src, tgt) = (edge.source(), edge.target());
        out.push_str(&format!("    n{} --> n{}\n", src.index(), tgt.index()));
    }

    out
}
