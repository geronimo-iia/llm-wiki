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
}

// ── Options ───────────────────────────────────────────────────────────────────

pub struct SearchOptions {
    pub no_excerpt: bool,
    pub include_sections: bool,
    pub top_k: usize,
    pub r#type: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            no_excerpt: false,
            include_sections: false,
            top_k: 10,
            r#type: None,
        }
    }
}

pub struct ListOptions {
    pub r#type: Option<String>,
    pub status: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            r#type: None,
            status: None,
            page: 1,
            page_size: 20,
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
) -> Result<Vec<PageRef>> {
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

    Ok(results)
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

    // Count total matches
    let total = searcher.search(&query, &Count)?;
    if total == 0 {
        return Ok(PageList {
            pages: Vec::new(),
            total: 0,
            page: options.page,
            page_size: options.page_size,
        });
    }

    // Fetch sorted by _slug_ord, limited to offset + page_size
    let page = options.page;
    let page_size = options.page_size;
    let offset = (page - 1) * page_size;
    let limit = offset + page_size;

    let sorted_docs = searcher.search(
        &query,
        &TopDocs::with_limit(limit).order_by_fast_field::<u64>("_slug_ord", Order::Asc),
    )?;

    // Extract full fields only for the page window
    let window = if offset < sorted_docs.len() {
        &sorted_docs[offset..]
    } else {
        &[]
    };

    let mut summaries = Vec::with_capacity(window.len());
    for (_ord, doc_addr) in window {
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

    // Stable sort within the window for ties (same 8-byte prefix)
    summaries.sort_by(|a, b| a.slug.cmp(&b.slug));

    Ok(PageList {
        pages: summaries,
        total,
        page,
        page_size,
    })
}

// ── search_all ────────────────────────────────────────────────────────────────

pub fn search_all(
    query_str: &str,
    options: &SearchOptions,
    wikis: &[(String, Searcher, &IndexSchema)],
) -> Result<Vec<PageRef>> {
    let mut all_results = Vec::new();
    for (name, searcher, is) in wikis {
        match search(query_str, options, searcher, name, is) {
            Ok(results) => all_results.extend(results),
            Err(_) => continue,
        }
    }
    all_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(options.top_k);
    Ok(all_results)
}
