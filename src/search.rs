use std::cmp::Reverse;
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::{
    DocId, Order, Score, Searcher, Term,
    collector::{Count, TopDocs},
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    snippet::{Snippet, SnippetGenerator},
};

use crate::config::SearchConfig;
use crate::index_schema::IndexSchema;

// ── Return types ──────────────────────────────────────────────────────────────

/// A single search result with BM25 score and optional highlighted excerpt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    /// Page slug (repository-relative path without extension).
    pub slug: String,
    /// Fully-qualified `wiki://` URI for the page.
    pub uri: String,
    /// Page title from frontmatter.
    pub title: String,
    /// Adjusted BM25 score (multiplied by status and confidence).
    pub score: f32,
    /// Frontmatter `confidence` value in [0, 1].
    pub confidence: f32,
    /// HTML-highlighted body excerpt, if requested.
    pub excerpt: Option<String>,
    /// Frontmatter `summary` field, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Lightweight page metadata returned by listing operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    /// Page slug.
    pub slug: String,
    /// Fully-qualified `wiki://` URI.
    pub uri: String,
    /// Page title from frontmatter.
    pub title: String,
    /// Page type from frontmatter.
    pub r#type: String,
    /// Page status from frontmatter.
    pub status: String,
    /// Tags from frontmatter.
    pub tags: Vec<String>,
    /// Frontmatter `confidence` value in [0, 1].
    pub confidence: f32,
    /// Frontmatter `summary` field, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// A paginated list of pages with facet counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageList {
    /// Pages in the current page window.
    pub pages: Vec<PageSummary>,
    /// Total pages matching the filter (across all pages).
    pub total: usize,
    /// Current 1-based page number.
    pub page: usize,
    /// Number of items per page.
    pub page_size: usize,
    /// Facet counts for type, status, and tags.
    #[serde(default, skip_serializing_if = "FacetCounts::is_empty")]
    pub facets: FacetCounts,
}

// ── Facets ────────────────────────────────────────────────────────────────────

/// Distribution counts for type, status, and tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FacetCounts {
    /// Count of pages per frontmatter type.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub r#type: HashMap<String, u64>,
    /// Count of pages per frontmatter status.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub status: HashMap<String, u64>,
    /// Count of pages per tag.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, u64>,
}

impl FacetCounts {
    /// Return true if all three facet maps are empty.
    pub fn is_empty(&self) -> bool {
        self.r#type.is_empty() && self.status.is_empty() && self.tags.is_empty()
    }
}

/// The full result of a search query including ranked results and facets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Ranked search results.
    pub results: Vec<PageRef>,
    /// Facet counts for the result set.
    pub facets: FacetCounts,
}

// ── Options ───────────────────────────────────────────────────────────────────

/// Options for a BM25 search query.
pub struct SearchOptions {
    /// Omit HTML excerpt from results when true.
    pub no_excerpt: bool,
    /// Include section index pages in results when true.
    pub include_sections: bool,
    /// Maximum number of results to return.
    pub top_k: usize,
    /// Optional frontmatter type filter.
    pub r#type: Option<String>,
    /// Maximum tag facet values to return (0 = all).
    pub facets_top_tags: usize,
    /// Status score multiplier config applied to BM25 scores.
    pub search_config: SearchConfig,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            no_excerpt: false,
            include_sections: false,
            top_k: 10,
            r#type: None,
            facets_top_tags: 10,
            search_config: SearchConfig::default(),
        }
    }
}

/// Options for a paginated page list operation.
pub struct ListOptions {
    /// Optional frontmatter type filter.
    pub r#type: Option<String>,
    /// Optional frontmatter status filter.
    pub status: Option<String>,
    /// 1-based page number.
    pub page: usize,
    /// Number of items per page.
    pub page_size: usize,
    /// Maximum tag facet values to return (0 = all).
    pub facets_top_tags: usize,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            r#type: None,
            status: None,
            page: 1,
            page_size: 20,
            facets_top_tags: 10,
        }
    }
}

// ── search ────────────────────────────────────────────────────────────────────

/// Run a BM25 full-text search against a single wiki's index.
pub fn search(
    query_str: &str,
    options: &SearchOptions,
    searcher: &Searcher,
    wiki_name: &str,
    is: &IndexSchema,
) -> Result<SearchResult> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_summary = is.try_field("summary");
    let f_body = is.field("body");
    let f_type = is.field("type");

    let index = searcher.index();
    let mut query_fields = vec![f_title, f_body];
    if let Some(f) = f_summary {
        query_fields.insert(1, f);
    }
    let query_parser = QueryParser::for_index(index, query_fields);
    let parsed = query_parser
        .parse_query(query_str)
        .with_context(|| format!("failed to parse query: {query_str}"))?;

    // Build the filtered query (with type filter)
    let final_query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
        clauses.push((Occur::Must, parsed));

        if !options.include_sections {
            clauses.push((
                Occur::MustNot,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, "section"),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        Box::new(BooleanQuery::new(clauses))
    };

    let sc = options.search_config.clone();
    let has_confidence = is.try_field("confidence").is_some();
    let collector = TopDocs::with_limit(options.top_k).tweak_score(
        move |segment_reader: &tantivy::SegmentReader| {
            let status_col = segment_reader.fast_fields().str("status").ok().flatten();
            let conf_col = if has_confidence {
                segment_reader.fast_fields().f64("confidence").ok()
            } else {
                None
            };
            let status_map = sc.status.clone();
            move |doc: DocId, score: Score| {
                let unknown_mult = status_map.get("unknown").copied().unwrap_or(0.9);
                let status_mult = match &status_col {
                    Some(col) => match col.term_ords(doc).next() {
                        Some(ord) => {
                            let mut buf = String::new();
                            col.ord_to_str(ord, &mut buf).ok();
                            status_map
                                .get(buf.as_str())
                                .copied()
                                .unwrap_or(unknown_mult)
                        }
                        None => unknown_mult,
                    },
                    None => unknown_mult,
                };
                let confidence = conf_col.as_ref().and_then(|c| c.first(doc)).unwrap_or(0.5) as f32;
                score * status_mult * confidence
            }
        },
    );
    let top_docs = searcher.search(&final_query, &collector)?;

    let snippet_gen = if !options.no_excerpt {
        Some(SnippetGenerator::create(searcher, &final_query, f_body)?)
    } else {
        None
    };

    let f_confidence = is.try_field("confidence");

    let mut results = Vec::new();
    for (score, doc_addr) in top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;

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
        let uri = format!("wiki://{wiki_name}/{slug}");

        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let excerpt = snippet_gen.as_ref().map(|sg| {
            let snippet: Snippet = sg.snippet_from_doc(&doc);
            snippet.to_html()
        });

        let summary = f_summary
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        results.push(PageRef {
            slug,
            uri,
            title,
            score,
            confidence,
            excerpt,
            summary,
        });
    }

    // Facets: type is unfiltered, status and tags are filtered
    // Re-parse query for the unfiltered facet query
    let unfiltered_query: Box<dyn tantivy::query::Query> = {
        let parsed2 = query_parser
            .parse_query(query_str)
            .with_context(|| format!("failed to parse query: {query_str}"))?;
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
        clauses.push((Occur::Must, parsed2));
        if !options.include_sections {
            clauses.push((
                Occur::MustNot,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, "section"),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        Box::new(BooleanQuery::new(clauses))
    };

    let type_facet = collect_facet(searcher, &unfiltered_query, is, "type", 0)?;
    let status_facet = collect_facet(searcher, &final_query, is, "status", 0)?;
    let tags_facet = collect_facet(searcher, &final_query, is, "tags", options.facets_top_tags)?;

    Ok(SearchResult {
        results,
        facets: FacetCounts {
            r#type: type_facet,
            status: status_facet,
            tags: tags_facet,
        },
    })
}

// ── list ──────────────────────────────────────────────────────────────────────

/// Return a paginated list of pages from the index, sorted alphabetically by slug.
pub fn list(
    options: &ListOptions,
    searcher: &Searcher,
    wiki_name: &str,
    is: &IndexSchema,
) -> Result<PageList> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_status = is.field("status");
    let f_tags = is.field("tags");
    let f_confidence = is.try_field("confidence");
    let f_summary = is.try_field("summary");

    let query: Box<dyn tantivy::query::Query> = {
        let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        if let Some(ref type_filter) = options.r#type {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_type, type_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(ref status_filter) = options.status {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(f_status, status_filter),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if clauses.is_empty() {
            Box::new(AllQuery)
        } else {
            Box::new(BooleanQuery::new(clauses))
        }
    };

    // Unfiltered query for type facet (no type/status filter)
    let unfiltered_query: Box<dyn tantivy::query::Query> = Box::new(AllQuery);

    // Count total matches
    let total = searcher.search(&query, &Count)?;
    if total == 0 {
        // Still collect facets even with no results in the page window
        let type_facet = collect_facet(searcher, &unfiltered_query, is, "type", 0)?;
        let status_facet = collect_facet(searcher, &query, is, "status", 0)?;
        let tags_facet = collect_facet(searcher, &query, is, "tags", options.facets_top_tags)?;
        return Ok(PageList {
            pages: Vec::new(),
            total: 0,
            page: options.page,
            page_size: options.page_size,
            facets: FacetCounts {
                r#type: type_facet,
                status: status_facet,
                tags: tags_facet,
            },
        });
    }

    // Fetch sorted by _slug_ord, limited to offset + page_size
    let page = options.page;
    let page_size = options.page_size;
    let offset = (page - 1) * page_size;
    let limit = offset + page_size;

    let sorted_docs = searcher.search(
        &query,
        &TopDocs::with_limit(limit).order_by_string_fast_field("slug", Order::Asc),
    )?;

    // Extract full fields only for the page window
    let window = if offset < sorted_docs.len() {
        &sorted_docs[offset..]
    } else {
        &[]
    };

    let mut summaries = Vec::with_capacity(window.len());
    for (_slug_val, doc_addr) in window {
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
        let status = doc
            .get_first(f_status)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags_str = doc
            .get_first(f_tags)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = tags_str
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let summary = f_summary
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let uri = format!("wiki://{wiki_name}/{slug}");

        summaries.push(PageSummary {
            slug,
            uri,
            title,
            r#type: page_type,
            status,
            tags,
            confidence,
            summary,
        });
    }

    Ok(PageList {
        pages: summaries,
        total,
        page,
        page_size,
        facets: {
            let type_facet = collect_facet(searcher, &unfiltered_query, is, "type", 0)?;
            let status_facet = collect_facet(searcher, &query, is, "status", 0)?;
            let tags_facet = collect_facet(searcher, &query, is, "tags", options.facets_top_tags)?;
            FacetCounts {
                r#type: type_facet,
                status: status_facet,
                tags: tags_facet,
            }
        },
    })
}

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
            Err(_) => continue,
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

// ── Facet collection ──────────────────────────────────────────────────────────

/// Collect term frequency counts for a keyword FAST field across matching docs.
/// If `top_n` is 0, return all values. Otherwise return the top N by count.
fn collect_facet(
    searcher: &Searcher,
    query: &dyn tantivy::query::Query,
    is: &IndexSchema,
    field_name: &str,
    top_n: usize,
) -> Result<HashMap<String, u64>> {
    let field = match is.try_field(field_name) {
        Some(f) => f,
        None => return Ok(HashMap::new()),
    };

    let doc_addrs = searcher.search(query, &tantivy::collector::DocSetCollector)?;
    let mut counts: HashMap<String, u64> = HashMap::new();

    for doc_addr in &doc_addrs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;
        for val in doc.get_all(field) {
            if let Some(s) = val.as_str()
                && !s.is_empty()
            {
                *counts.entry(s.to_string()).or_insert(0) += 1;
            }
        }
    }

    if top_n > 0 && counts.len() > top_n {
        let mut entries: Vec<_> = counts.into_iter().collect();
        entries.sort_by_key(|e| Reverse(e.1));
        entries.truncate(top_n);
        return Ok(entries.into_iter().collect());
    }

    Ok(counts)
}

// ── llms renderers ────────────────────────────────────────────────────────────

/// Render a `PageList` as LLM-optimized markdown: pages grouped by type,
/// one line per page with summary. Archived pages shown with strikethrough.
pub fn render_list_llms(result: &PageList) -> String {
    // Group by type, sorted by count desc then name asc
    let mut by_type: std::collections::HashMap<String, Vec<&PageSummary>> =
        std::collections::HashMap::new();
    for page in &result.pages {
        by_type.entry(page.r#type.clone()).or_default().push(page);
    }
    let mut groups: Vec<(String, Vec<&PageSummary>)> = by_type.into_iter().collect();
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));

    let mut out = String::new();
    for (type_name, mut pages) in groups {
        pages.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.title.cmp(&b.title))
        });
        out.push_str(&format!("## {} ({})\n\n", type_name, pages.len()));
        for page in pages {
            let summary = page.summary.as_deref().unwrap_or("");
            let line = if page.status == "archived" {
                if summary.is_empty() {
                    format!("- ~~[{}]({})~~\n", page.title, page.uri)
                } else {
                    format!("- ~~[{}]({}): {}~~\n", page.title, page.uri, summary)
                }
            } else if summary.is_empty() {
                format!("- [{}]({})\n", page.title, page.uri)
            } else {
                format!("- [{}]({}): {}\n", page.title, page.uri, summary)
            };
            out.push_str(&line);
        }
        out.push('\n');
    }

    if result.total > result.page_size {
        let total_pages = (result.total + result.page_size - 1) / result.page_size.max(1);
        out.push_str(&format!(
            "_Page {}/{} — {} total pages_\n",
            result.page, total_pages, result.total
        ));
    }

    out
}

/// Render a `SearchResult` as LLM-optimized markdown: one line per result
/// with title, uri, and summary. No score, no excerpt block.
pub fn render_search_llms(result: &SearchResult) -> String {
    if result.results.is_empty() {
        return "No results found.\n".to_string();
    }
    let mut out = String::new();
    for r in &result.results {
        let summary = r.summary.as_deref().unwrap_or("");
        if summary.is_empty() {
            out.push_str(&format!("- [{}]({})\n", r.title, r.uri));
        } else {
            out.push_str(&format!("- [{}]({}): {}\n", r.title, r.uri, summary));
        }
    }
    out
}
