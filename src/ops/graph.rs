use anyhow::Result;

use crate::engine::EngineState;
use crate::graph;

/// Rendered graph output plus the associated report.
pub struct GraphResult {
    /// Rendered graph string (Mermaid, DOT, or llms format).
    pub rendered: String,
    /// Metadata about the generated graph.
    pub report: graph::GraphReport,
}

/// Parameters for `graph_build`.
pub struct GraphParams<'a> {
    /// Output format: `"mermaid"`, `"dot"`, or `"llms"`.
    pub format: Option<&'a str>,
    /// Slug of the root node for a subgraph traversal.
    pub root: Option<String>,
    /// Maximum hops from root.
    pub depth: Option<usize>,
    /// Comma-separated page types to include.
    pub type_filter: Option<&'a str>,
    /// Filter edges by this relation label.
    pub relation: Option<String>,
    /// File path to write output to; `None` for returning only.
    pub output: Option<&'a str>,
    /// If true, merge all mounted wikis into a single graph.
    pub cross_wiki: bool,
}

/// Build and render the concept graph according to `params`.
pub fn graph_build(
    engine: &EngineState,
    wiki_name: &str,
    params: &GraphParams<'_>,
) -> Result<GraphResult> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config(&engine.config);

    let fmt = params.format.unwrap_or(&resolved.graph.format);
    let types: Vec<String> = params
        .type_filter
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let filter = graph::GraphFilter {
        root: params.root.clone(),
        depth: params.depth.or(Some(resolved.graph.depth as usize)),
        types,
        relation: params.relation.clone(),
    };
    let g = if params.cross_wiki {
        // Collect (name, searcher) pairs; Arc keeps schema/registry alive
        let space_data: Vec<(String, tantivy::Searcher)> = engine
            .spaces
            .iter()
            .filter_map(|(name, sp)| sp.index_manager.searcher().ok().map(|s| (name.clone(), s)))
            .collect();
        let tuples: Vec<(
            &str,
            &tantivy::Searcher,
            &crate::index_schema::IndexSchema,
            &crate::type_registry::SpaceTypeRegistry,
        )> = space_data
            .iter()
            .filter_map(|(name, searcher)| {
                engine
                    .spaces
                    .get(name)
                    .map(|sp| (name.as_str(), searcher, &sp.index_schema, &sp.type_registry))
            })
            .collect();
        graph::build_graph_cross_wiki(&tuples, &filter)?
    } else {
        let searcher = space.index_manager.searcher()?;
        graph::build_graph(
            &searcher,
            &space.index_schema,
            &filter,
            &space.type_registry,
        )?
    };

    let rendered = match fmt {
        "dot" => graph::render_dot(&g),
        "llms" => graph::render_llms(&g),
        _ => graph::render_mermaid(&g),
    };

    let out = if let Some(out_path) = params.output {
        let content = if out_path.ends_with(".md") {
            graph::wrap_graph_md(&rendered, fmt, &filter)
        } else {
            rendered.clone()
        };
        std::fs::write(out_path, &content)?;
        out_path.to_string()
    } else {
        "stdout".to_string()
    };

    Ok(GraphResult {
        rendered,
        report: graph::GraphReport {
            nodes: g.node_count(),
            edges: g.edge_count(),
            output: out,
        },
    })
}
