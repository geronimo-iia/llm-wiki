use std::sync::Arc;

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
    /// Output format: `"mermaid"`, `"dot"`, `"llms"`, or `"json"`.
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
    let g: Arc<graph::WikiGraph> = if params.cross_wiki {
        // Build each space graph through its cache, then merge
        let mut per_space: Vec<(&str, Arc<graph::WikiGraph>)> = Vec::new();
        for (name, sp) in engine.spaces.iter() {
            if let Ok(searcher) = sp.index_manager.searcher() {
                let g = graph::get_or_build_graph(
                    &sp.index_schema,
                    &sp.type_registry,
                    &sp.index_manager,
                    &sp.graph_cache,
                    &searcher,
                    &filter,
                )?;
                per_space.push((name.as_str(), g));
            }
        }
        Arc::new(graph::merge_cached_graphs(&per_space, &filter)?)
    } else {
        let searcher = space.index_manager.searcher()?;
        graph::get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &filter,
        )?
    };

    let rendered = match fmt {
        "dot" => graph::render_dot(&g),
        "llms" => graph::render_llms(&g),
        "json" => graph::render_json(&g),
        "mermaid" => graph::render_mermaid(&g),
        other => {
            anyhow::bail!("unknown graph format {other:?}: expected mermaid, dot, llms, or json")
        }
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
