use std::sync::Arc;

use anyhow::Result;

use crate::config::GraphFormat;
use crate::engine::EngineState;
use crate::graph;

/// Runtime graph output format. Extends [`GraphFormat`] with `Summary`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphRenderFormat {
    Mermaid,
    Dot,
    Llms,
    Json,
    Summary,
}

impl GraphRenderFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Dot => "dot",
            Self::Llms => "llms",
            Self::Json => "json",
            Self::Summary => "summary",
        }
    }
}

impl std::str::FromStr for GraphRenderFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mermaid" => Ok(Self::Mermaid),
            "dot" => Ok(Self::Dot),
            "llms" => Ok(Self::Llms),
            "json" => Ok(Self::Json),
            "summary" => Ok(Self::Summary),
            other => Err(format!(
                "unknown graph format {other:?}: expected mermaid, dot, llms, json, or summary"
            )),
        }
    }
}

impl From<GraphFormat> for GraphRenderFormat {
    fn from(f: GraphFormat) -> Self {
        match f {
            GraphFormat::Mermaid => Self::Mermaid,
            GraphFormat::Dot => Self::Dot,
            GraphFormat::Llms => Self::Llms,
            GraphFormat::Json => Self::Json,
        }
    }
}

/// Rendered graph output plus the associated report.
pub struct GraphResult {
    /// Rendered graph string (Mermaid, DOT, or llms format).
    pub rendered: String,
    /// Metadata about the generated graph.
    pub report: graph::GraphReport,
}

/// Parameters for `graph_build`.
pub struct GraphParams<'a> {
    /// Output format. `None` falls back to the wiki config value.
    pub format: Option<GraphRenderFormat>,
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
    /// Top-hub count for `format: "summary"` (default 10).
    pub limit: Option<usize>,
}

/// Build and render the concept graph according to `params`.
pub fn graph_build(
    engine: &EngineState,
    wiki_name: &str,
    params: &GraphParams<'_>,
) -> Result<GraphResult> {
    let space = engine.space(wiki_name)?;
    let resolved = space.resolved_config();

    let fmt: GraphRenderFormat = params
        .format
        .unwrap_or_else(|| resolved.graph.format.clone().into());
    let types: Vec<String> = params
        .type_filter
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let filter = graph::GraphFilter {
        root: params.root.clone(),
        depth: params.depth.or(Some(resolved.graph.depth as usize)),
        types,
        relation: params.relation.clone(),
        max_pages: resolved.graph.max_pages,
    };
    let top_n = params.limit.unwrap_or(10);

    let (g, render_ctx) = if params.cross_wiki {
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
        let merged = Arc::new(graph::merge_cached_graphs(&per_space, &filter)?);
        let ctx = graph::RenderContext {
            top_n,
            communities: None,
        };
        (merged, ctx)
    } else {
        let searcher = space.index_manager.searcher()?;
        let g = graph::get_or_build_graph(
            &space.index_schema,
            &space.type_registry,
            &space.index_manager,
            &space.graph_cache,
            &searcher,
            &filter,
        )?;
        let communities = if fmt == GraphRenderFormat::Summary {
            let min_nodes = resolved.graph.min_nodes_for_communities;
            graph::get_cached_community_stats(
                &space.index_schema,
                &space.type_registry,
                &space.index_manager,
                &space.graph_cache,
                &space.community_cache,
                &searcher,
                min_nodes,
            )
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "community stats unavailable for summary; omitting");
                None
            })
        } else {
            None
        };
        let ctx = graph::RenderContext { top_n, communities };
        (g, ctx)
    };

    let rendered = match fmt {
        GraphRenderFormat::Dot => graph::render_dot(&g),
        GraphRenderFormat::Llms => graph::render_llms(&g),
        GraphRenderFormat::Json => graph::render_json(&g, resolved.graph.min_nodes_for_communities),
        GraphRenderFormat::Mermaid => graph::render_mermaid(&g),
        GraphRenderFormat::Summary => graph::render_summary(&g, &render_ctx),
    };

    let out = if let Some(out_path) = params.output {
        let content = if out_path.ends_with(".md") {
            graph::wrap_graph_md(&rendered, fmt.as_str(), &filter)
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
