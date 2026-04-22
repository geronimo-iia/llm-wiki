use std::cmp::Reverse;
use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::{Count, TopDocs},
    query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery},
    schema::{IndexRecordOption, Value},
    snippet::{Snippet, SnippetGenerator},
    Order, Searcher, Term,
};

use crate::index_schema::IndexSchema;

// ── Return types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRef {
    pub slug: String,
    pub uri: String,
    pub title: String,
    pub score: f32,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub slug: String,
    pub uri: String,
    pub title: String,
    pub r#type: String,
    pub status: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageList {
    pub pages: Vec<PageSummary>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    #[serde(default, skip_serializing_if = "FacetCounts::is_empty")]
    pub facets: FacetCounts,
}

// ── Facets ────────────────────────────────────────────────────────────────────

/// Distribution counts for type, status, and tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FacetCounts {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub r#type: HashMap<String, u64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub status: HashMap<String, u64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, u64>,
}

impl FacetCounts {
    pub fn is_empty(&self) -> bool {
        self.r#type.is_empty() && self.status.is_empty() && self.tags.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub results: Vec<PageRef>,
    pub facets: FacetCounts,
}

// ── Options ───────────────────────────────────────────────────────────────────

pub struct SearchOptions {
    pub no_excerpt: bool,
    pub include_sections: bool,
    pub top_k: usize,
    pub r#type: Option<String>,
    pub facets_top_tags: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            no_excerpt: false,
            include_sections: false,
            top_k: 10,
            r#type: None,
            facets_top_tags: 10,
        }
    }
}

pub struct ListOptions {
    pub r#type: Option<String>,
    pub status: Option<String>,
    pub page: usize,
    pub page_size: usize,
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

pub fn search(
    query_str: &str,
    options: &SearchOptions,
    searcher: &Searcher,
    wiki_name: &str,
    is: &IndexSchema,
) -> Result<SearchResult> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_summary = is.field("summary");
    let f_body = is.field("body");
    let f_type = is.field("type");

    let index = searcher.index();
    let query_parser = QueryParser::for_index(index, vec![f_title, f_summary, f_body]);
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

    let top_docs = searcher.search(
        &final_query,
        &TopDocs::with_limit(options.top_k).order_by_score(),
    )?;

    let snippet_gen = if !options.no_excerpt {
        Some(SnippetGenerator::create(searcher, &final_query, f_body)?)
    } else {
        None
    };

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

        let excerpt = snippet_gen.as_ref().map(|gen| {
            let snippet: Snippet = gen.snippet_from_doc(&doc);
            snippet.to_html()
        });

        results.push(PageRef {
            slug,
            uri,
            title,
            score,
            excerpt,
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

        let uri = format!("wiki://{wiki_name}/{slug}");

        summaries.push(PageSummary {
            slug,
            uri,
            title,
            r#type: page_type,
            status,
            tags,
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
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    *counts.entry(s.to_string()).or_insert(0) += 1;
                }
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
