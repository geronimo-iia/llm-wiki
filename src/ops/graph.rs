use anyhow::Result;

use crate::engine::EngineState;
use crate::graph;

pub struct GraphResult {
    pub rendered: String,
    pub report: graph::GraphReport,
}

pub struct GraphParams<'a> {
    pub format: Option<&'a str>,
    pub root: Option<String>,
    pub depth: Option<usize>,
    pub type_filter: Option<&'a str>,
    pub relation: Option<String>,
    pub output: Option<&'a str>,
}

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
    let searcher = space.index_manager.searcher()?;
    let g = graph::build_graph(&searcher, &space.index_schema, &filter)?;

    let rendered = match fmt {
        "dot" => graph::render_dot(&g),
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
