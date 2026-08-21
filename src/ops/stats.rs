//! Aggregate statistics for a wiki — page counts, graph metrics, staleness buckets, and index health.

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::engine::EngineState;
use crate::graph::{
    self, CommunityStats, GraphFilter, get_cached_community_stats, get_or_build_graph,
};
use crate::search;
use tantivy::SegmentReader;
use tantivy::collector::{Collector, SegmentCollector};

/// Controls how much detail `stats()` returns for expensive list fields.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum StatsDetail {
    /// Return `center_count` only — no `center` slug list. Default.
    #[default]
    Summary,
    /// Return the full `center` slug list.
    Full,
}

/// Options for a `stats()` call. All fields default to their zero values.
#[derive(Default)]
pub struct StatsOptions {
    pub detail: StatsDetail,
}

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
    /// Maximum shortest directed-path length between any two pages.
    /// `None` when graph exceeds `max_nodes_for_diameter` or `structural_algorithms` is false.
    pub diameter: Option<f32>,
    /// Minimum eccentricity — closest page to all others on average.
    /// `None` under same conditions as `diameter`.
    pub radius: Option<f32>,
    /// Slugs with eccentricity equal to `radius` (central hub pages).
    /// Empty when `diameter` is `None` or `detail` is `"summary"`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub center: Vec<String>,
    /// Number of central hub pages (present only when `detail` is `"summary"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_count: Option<usize>,
    /// Non-null when O(n²) algorithms were skipped due to graph size.
    pub structural_note: Option<String>,
}

/// Compute aggregate stats for a wiki — page counts, graph metrics, staleness, and index health.
pub fn stats(engine: &EngineState, wiki_name: &str, opts: &StatsOptions) -> Result<WikiStats> {
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
    let metrics = graph::compute_metrics(&wiki_graph);
    let resolved = space.resolved_config(&engine.config);
    let communities = get_cached_community_stats(
        &space.index_schema,
        &space.type_registry,
        &space.index_manager,
        &space.graph_cache,
        &space.community_cache,
        &searcher,
        resolved.graph.min_nodes_for_communities,
    )?;

    // Staleness buckets from last_updated field
    let staleness = compute_staleness(&searcher, &space.index_schema)?;

    // Index health
    let index_status = space.index_manager.status(&space.repo_root);
    let index = IndexHealth {
        stale: index_status.as_ref().map(|s| s.stale).unwrap_or(true),
        built: index_status.ok().and_then(|s| s.built),
    };

    // Structural topology fields
    let local_count = wiki_graph
        .node_indices()
        .filter(|&idx| !wiki_graph[idx].external)
        .count();
    let max_n = resolved.graph.max_nodes_for_diameter;

    let (diameter, radius, center, structural_note) = if !resolved.graph.structural_algorithms {
        (
            None,
            None,
            vec![],
            Some(
                "structural algorithms disabled in config (graph.structural_algorithms = false)"
                    .to_string(),
            ),
        )
    } else if local_count <= max_n {
        let d_raw = petgraph_live::metrics::diameter(&*wiki_graph);
        let r_raw = petgraph_live::metrics::radius(&*wiki_graph);
        // petgraph_live returns Some(INFINITY) for disconnected graphs; normalise to None.
        let disconnected = d_raw.is_some_and(|v| v.is_infinite());
        let d = if disconnected { None } else { d_raw };
        let r = if disconnected { None } else { r_raw };
        let c: Vec<String> = if disconnected {
            vec![]
        } else {
            petgraph_live::metrics::center(&*wiki_graph)
                .into_iter()
                .filter(|&idx| !wiki_graph[idx].external)
                .map(|idx| wiki_graph[idx].slug.clone())
                .collect()
        };
        let note = if disconnected {
            Some(
                "graph is not strongly connected — diameter undefined; \
                 use wiki_lint(rules: \"periphery,orphan\") to find disconnected pages"
                    .to_string(),
            )
        } else {
            None
        };
        (d, r, c, note)
    } else {
        let note = format!(
            "graph too large for diameter computation ({local_count} nodes > max_nodes_for_diameter={max_n})"
        );
        (None, None, vec![], Some(note))
    };

    let (center_out, center_count_out) = match opts.detail {
        StatsDetail::Full => (center, None),
        StatsDetail::Summary => {
            let n = center.len();
            (vec![], Some(n))
        }
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
        diameter,
        radius,
        center: center_out,
        center_count: center_count_out,
        structural_note,
    })
}

struct StalenessCollector {
    seven_days_ago: chrono::NaiveDate,
    thirty_days_ago: chrono::NaiveDate,
    field_name: String,
}

struct StalenessSegmentCollector {
    column: tantivy::columnar::StrColumn,
    seven_days_ago: chrono::NaiveDate,
    thirty_days_ago: chrono::NaiveDate,
    fresh: usize,
    stale_7d: usize,
    stale_30d: usize,
}

impl Collector for StalenessCollector {
    type Fruit = StalenessBuckets;
    type Child = StalenessSegmentCollector;

    fn for_segment(
        &self,
        _segment_ord: u32,
        reader: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let column = reader
            .fast_fields()
            .str(&self.field_name)?
            .ok_or_else(|| tantivy::TantivyError::FieldNotFound(self.field_name.clone()))?;
        Ok(StalenessSegmentCollector {
            column,
            seven_days_ago: self.seven_days_ago,
            thirty_days_ago: self.thirty_days_ago,
            fresh: 0,
            stale_7d: 0,
            stale_30d: 0,
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(
        &self,
        segment_fruits: Vec<StalenessBuckets>,
    ) -> tantivy::Result<StalenessBuckets> {
        Ok(segment_fruits.into_iter().fold(
            StalenessBuckets {
                fresh: 0,
                stale_7d: 0,
                stale_30d: 0,
            },
            |mut acc, b| {
                acc.fresh += b.fresh;
                acc.stale_7d += b.stale_7d;
                acc.stale_30d += b.stale_30d;
                acc
            },
        ))
    }
}

impl SegmentCollector for StalenessSegmentCollector {
    type Fruit = StalenessBuckets;

    fn collect(&mut self, doc: u32, _score: tantivy::Score) {
        let mut date_str = String::new();
        if let Some(ord) = self.column.term_ords(doc).next() {
            let _ = self.column.ord_to_str(ord, &mut date_str);
        } else {
            self.stale_30d += 1;
            return;
        }
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            if date >= self.seven_days_ago {
                self.fresh += 1;
            } else if date >= self.thirty_days_ago {
                self.stale_7d += 1;
            } else {
                self.stale_30d += 1;
            }
        } else {
            self.stale_30d += 1;
        }
    }

    fn harvest(self) -> StalenessBuckets {
        StalenessBuckets {
            fresh: self.fresh,
            stale_7d: self.stale_7d,
            stale_30d: self.stale_30d,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staleness_merge_fruits_accumulates_all_buckets() {
        let collector = StalenessCollector {
            seven_days_ago: chrono::NaiveDate::from_ymd_opt(2026, 8, 14).unwrap(),
            thirty_days_ago: chrono::NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            field_name: "last_updated".into(),
        };
        let fruits = vec![
            StalenessBuckets { fresh: 3, stale_7d: 2, stale_30d: 1 },
            StalenessBuckets { fresh: 1, stale_7d: 0, stale_30d: 4 },
        ];
        let merged = Collector::merge_fruits(&collector, fruits).unwrap();
        assert_eq!(merged.fresh, 4);
        assert_eq!(merged.stale_7d, 2);
        assert_eq!(merged.stale_30d, 5);
    }

    #[test]
    fn staleness_merge_fruits_empty_input_yields_zeros() {
        let collector = StalenessCollector {
            seven_days_ago: chrono::NaiveDate::from_ymd_opt(2026, 8, 14).unwrap(),
            thirty_days_ago: chrono::NaiveDate::from_ymd_opt(2026, 7, 22).unwrap(),
            field_name: "last_updated".into(),
        };
        let merged = Collector::merge_fruits(&collector, vec![]).unwrap();
        assert_eq!(merged.fresh, 0);
        assert_eq!(merged.stale_7d, 0);
        assert_eq!(merged.stale_30d, 0);
    }

    #[test]
    fn stats_detail_default_is_summary() {
        assert_eq!(StatsDetail::default(), StatsDetail::Summary);
    }
}

fn compute_staleness(
    searcher: &tantivy::Searcher,
    is: &crate::index_schema::IndexSchema,
) -> Result<StalenessBuckets> {
    let f_name = match is.try_field("last_updated") {
        Some(_) => "last_updated",
        None => {
            return Ok(StalenessBuckets {
                fresh: 0,
                stale_7d: 0,
                stale_30d: 0,
            });
        }
    };

    let today = chrono::Utc::now().date_naive();
    let collector = StalenessCollector {
        seven_days_ago: today - chrono::Duration::days(7),
        thirty_days_ago: today - chrono::Duration::days(30),
        field_name: f_name.to_string(),
    };
    Ok(searcher.search(&tantivy::query::AllQuery, &collector)?)
}
