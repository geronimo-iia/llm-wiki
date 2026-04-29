use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::engine::EngineState;
use crate::graph::{self, CommunityStats, GraphFilter, get_or_build_graph};
use crate::search;
use tantivy::schema::Value;

/// Page staleness bucketed by last-updated age.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StalenessBuckets {
    /// Pages updated within the last 7 days.
    pub fresh: usize,
    /// Pages updated 7–30 days ago.
    pub stale_7d: usize,
    /// Pages updated more than 30 days ago (or with no date).
    pub stale_30d: usize,
}

/// Summary health status of the tantivy search index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHealth {
    /// True if the index is out of date relative to the wiki files.
    pub stale: bool,
    /// ISO-8601 timestamp of the last successful index build, if known.
    pub built: Option<String>,
}

/// Aggregate statistics for a single wiki space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiStats {
    /// Name of the wiki.
    pub wiki: String,
    /// Total number of indexed pages.
    pub pages: usize,
    /// Number of pages whose type is `"section"`.
    pub sections: usize,
    /// Page count per frontmatter type.
    pub types: HashMap<String, u64>,
    /// Page count per frontmatter status.
    pub status: HashMap<String, u64>,
    /// Number of pages with no incoming links.
    pub orphans: usize,
    /// Mean number of links per page (rounded to 2 decimal places).
    pub avg_connections: f64,
    /// Graph density (edges / max-possible-edges, rounded to 2 decimal places).
    pub graph_density: f64,
    /// Page staleness buckets by last-updated date.
    pub staleness: StalenessBuckets,
    /// Index health — staleness and last build timestamp.
    pub index: IndexHealth,
    /// Louvain community detection results; `None` when graph is below `min_nodes_for_communities`.
    pub communities: Option<CommunityStats>,
}

/// Compute aggregate stats for a wiki — page counts, graph metrics, staleness, and index health.
pub fn stats(engine: &EngineState, wiki_name: &str) -> Result<WikiStats> {
    let space = engine.space(wiki_name)?;

    // Page counts + facets from list
    let searcher = space.index_manager.searcher()?;
    let list_result = search::list(
        &search::ListOptions {
            page_size: 1,
            facets_top_tags: 0,
            ..Default::default()
        },
        &searcher,
        wiki_name,
        &space.index_schema,
    )?;

    let pages = list_result.total;
    let sections = *list_result.facets.r#type.get("section").unwrap_or(&0) as usize;
    let types = list_result.facets.r#type;
    let status = list_result.facets.status;

    // Graph metrics
    let wiki_graph = get_or_build_graph(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &searcher,
        &GraphFilter::default(),
    )?;
    let metrics = graph::compute_metrics(&*wiki_graph);
    let resolved = space.resolved_config(&engine.config);
    let communities =
        graph::compute_communities(&*wiki_graph, resolved.graph.min_nodes_for_communities);

    // Staleness buckets from last_updated field
    let staleness = compute_staleness(&searcher, &space.index_schema)?;

    // Index health
    let index_status = space.index_manager.status(&space.repo_root);
    let index = IndexHealth {
        stale: index_status.as_ref().map(|s| s.stale).unwrap_or(true),
        built: index_status.ok().and_then(|s| s.built),
    };

    Ok(WikiStats {
        wiki: wiki_name.to_string(),
        pages,
        sections,
        types,
        status,
        orphans: metrics.orphans,
        avg_connections: (metrics.avg_connections * 100.0).round() / 100.0,
        graph_density: (metrics.density * 100.0).round() / 100.0,
        staleness,
        index,
        communities,
    })
}

fn compute_staleness(
    searcher: &tantivy::Searcher,
    is: &crate::index_schema::IndexSchema,
) -> Result<StalenessBuckets> {
    let f_last_updated = match is.try_field("last_updated") {
        Some(f) => f,
        None => {
            return Ok(StalenessBuckets {
                fresh: 0,
                stale_7d: 0,
                stale_30d: 0,
            });
        }
    };

    let today = chrono::Utc::now().date_naive();
    let seven_days_ago = today - chrono::Duration::days(7);
    let thirty_days_ago = today - chrono::Duration::days(30);

    let all_docs = searcher.search(
        &tantivy::query::AllQuery,
        &tantivy::collector::DocSetCollector,
    )?;

    let mut fresh = 0usize;
    let mut stale_7d = 0usize;
    let mut stale_30d = 0usize;

    for doc_addr in &all_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;
        let date_str = doc
            .get_first(f_last_updated)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            if date >= seven_days_ago {
                fresh += 1;
            } else if date >= thirty_days_ago {
                stale_7d += 1;
            } else {
                stale_30d += 1;
            }
        } else {
            // No valid date — count as stale
            stale_30d += 1;
        }
    }

    Ok(StalenessBuckets {
        fresh,
        stale_7d,
        stale_30d,
    })
}
