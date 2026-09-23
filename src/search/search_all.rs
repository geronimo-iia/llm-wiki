use std::cmp::Reverse;

use anyhow::Result;
use tantivy::Searcher;

use crate::index_schema::IndexSchema;

use super::fn_search::search;
use super::options::SearchOptions;
use super::types::{FacetCounts, SearchResult};

// ── search_all ────────────────────────────────────────────────────────────────

/// Search across multiple wikis, merge results by score, and truncate to `top_k`.
pub fn search_all(
    query_str: &str,
    options: &SearchOptions,
    wikis: &[(String, Searcher, &IndexSchema)],
) -> Result<SearchResult> {
    let mut all_results = Vec::new();
    let mut merged_facets = FacetCounts::default();
    for (name, searcher, is) in wikis {
        match search(query_str, options, searcher, name, is) {
            Ok(sr) => {
                all_results.extend(sr.results);
                for (k, v) in sr.facets.r#type {
                    *merged_facets.r#type.entry(k).or_insert(0) += v;
                }
                for (k, v) in sr.facets.status {
                    *merged_facets.status.entry(k).or_insert(0) += v;
                }
                for (k, v) in sr.facets.tags {
                    *merged_facets.tags.entry(k).or_insert(0) += v;
                }
            }
            Err(e) => {
                tracing::warn!(wiki = %name, error = %e, "cross-wiki search failed for wiki; skipping");
                continue;
            }
        }
    }
    all_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.top_k);

    // Re-cap tags after merging
    if options.facets_top_tags > 0 && merged_facets.tags.len() > options.facets_top_tags {
        let mut entries: Vec<_> = merged_facets.tags.into_iter().collect();
        entries.sort_by_key(|e| Reverse(e.1));
        entries.truncate(options.facets_top_tags);
        merged_facets.tags = entries.into_iter().collect();
    }

    Ok(SearchResult {
        results: all_results,
        facets: merged_facets,
    })
}
